// app-reports — the shared report Worker for the apps published under iSltanX.
//
// Implements contract v1 of the private repository iSltanX/app-reports (its README is the
// contract): an app posts a report here, and this files it as an issue in that repository and
// stores its attachments under reports/<product>/<id>/. Nothing in this file is specific to one
// app: a product is enabled by creating its `product:<id>` label in the repository.
//
// Environment: GITHUB_TOKEN and RATE_LIMIT_SALT (secrets), REPORTS_REPO (variable),
// REPORTS_KV (KV namespace: rate-limit counters, idempotency keys, the product-label cache).

const KINDS = new Set(['bug', 'crash', 'suggestion', 'other'])
const FIELDS = new Set([
  'product', 'app_version', 'os', 'os_version', 'arch', 'locale', 'kind', 'category',
  'description', 'diagnostics', 'attachments', 'test',
])
const MAX_BODY = 16 * 1024 * 1024
const MAX_DIAGNOSTICS = 16 * 1024
const MAX_ATTACHMENTS = 3
const MAX_ATTACHMENT = 3 * 1024 * 1024
const MAX_DESCRIPTION = 2000
const PER_IP_PER_HOUR = 5
const PER_DAY = 100
const IDEMPOTENCY_TTL = 24 * 60 * 60
const LABEL_CACHE_TTL = 10 * 60
const GITHUB = 'https://api.github.com'

export class HttpError extends Error {
  constructor(status, body, headers = {}) {
    super(body.error)
    this.status = status
    this.body = body
    this.headers = headers
  }
}

const invalid = (field) => new HttpError(400, { error: 'invalid_field', field })

function json(status, body, headers = {}) {
  // No CORS headers, on purpose: reports come from apps, never from web pages.
  return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json', ...headers } })
}

export default {
  async fetch(request, env) {
    try {
      return await handle(request, env)
    } catch (error) {
      if (error instanceof HttpError) return json(error.status, error.body, error.headers)
      return json(502, { error: 'upstream' })
    }
  },
}

export async function handle(request, env) {
  const url = new URL(request.url)
  if (url.pathname !== '/v1/reports') return json(404, { error: 'not_found' })
  if (request.method !== 'POST') return json(405, { error: 'method_not_allowed' }, { allow: 'POST' })

  const type = (request.headers.get('content-type') ?? '').split(';')[0].trim().toLowerCase()
  if (type !== 'application/json') throw new HttpError(415, { error: 'unsupported_media_type' })
  const declared = Number(request.headers.get('content-length') ?? 0)
  if (declared > MAX_BODY) throw new HttpError(413, { error: 'too_large' })
  const raw = await readLimited(request, MAX_BODY)

  let report
  try {
    report = JSON.parse(raw)
  } catch {
    throw new HttpError(400, { error: 'invalid_json' })
  }
  const clean = validate(report)
  if (!(await productEnabled(env, clean.product))) throw new HttpError(400, { error: 'unknown_product' })

  // A retry after a lost response gets the same report number instead of a duplicate.
  const key = idempotencyKey(request)
  if (key) {
    const known = await env.REPORTS_KV.get(`idem:${key}`)
    if (known) return json(200, { id: Number(known) })
  }

  await rateLimit(request, env)
  const id = await fileReport(env, clean)
  if (key) await env.REPORTS_KV.put(`idem:${key}`, String(id), { expirationTtl: IDEMPOTENCY_TTL })
  return json(201, { id })
}

/** Reads the body, refusing to buffer more than `limit` bytes. */
async function readLimited(request, limit) {
  if (!request.body) return ''
  const reader = request.body.getReader()
  const chunks = []
  let size = 0
  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    size += value.byteLength
    if (size > limit) {
      await reader.cancel()
      throw new HttpError(413, { error: 'too_large' })
    }
    chunks.push(value)
  }
  const bytes = new Uint8Array(size)
  let offset = 0
  for (const chunk of chunks) {
    bytes.set(chunk, offset)
    offset += chunk.byteLength
  }
  return new TextDecoder().decode(bytes)
}

const isString = (value, max, min = 1) => typeof value === 'string' && value.length >= min && value.length <= max
const codePoints = (s) => [...s].length

/** Checks every field against the contract and returns the report with its attachments decoded. */
export function validate(report) {
  if (report === null || typeof report !== 'object' || Array.isArray(report)) throw new HttpError(400, { error: 'invalid_json' })
  // Unknown fields fail loudly instead of losing data.
  for (const key of Object.keys(report)) if (!FIELDS.has(key)) throw invalid(key)

  const { product, app_version, os, os_version, arch, locale, kind, category, description, diagnostics, attachments, test } = report
  if (typeof product !== 'string' || !/^[a-z][a-z0-9-]{1,31}$/.test(product)) throw invalid('product')
  if (!isString(app_version, 32)) throw invalid('app_version')
  if (!isString(os, 16)) throw invalid('os')
  if (!isString(os_version, 32)) throw invalid('os_version')
  if (!isString(arch, 16)) throw invalid('arch')
  if (locale !== undefined && !isString(locale, 16)) throw invalid('locale')
  if (!KINDS.has(kind)) throw invalid('kind')
  if (category !== undefined && (typeof category !== 'string' || !/^[a-z][a-z0-9-]{0,31}$/.test(category))) throw invalid('category')
  if (typeof description !== 'string') throw invalid('description')
  const text = description.trim()
  if (codePoints(text) < 1 || codePoints(text) > MAX_DESCRIPTION) throw invalid('description')
  if (diagnostics !== undefined) {
    if (diagnostics === null || typeof diagnostics !== 'object' || Array.isArray(diagnostics)) throw invalid('diagnostics')
    if (new TextEncoder().encode(JSON.stringify(diagnostics)).length > MAX_DIAGNOSTICS) throw new HttpError(413, { error: 'too_large' })
  }
  if (test !== undefined && typeof test !== 'boolean') throw invalid('test')

  const files = []
  if (attachments !== undefined) {
    if (!Array.isArray(attachments) || attachments.length > MAX_ATTACHMENTS) throw invalid('attachments')
    for (const attachment of attachments) files.push(decodeAttachment(attachment))
  }
  return { product, app_version, os, os_version, arch, locale, kind, category, description: text, diagnostics, test: test === true, files }
}

function decodeAttachment(attachment) {
  if (attachment === null || typeof attachment !== 'object' || Array.isArray(attachment)) throw invalid('attachments')
  for (const key of Object.keys(attachment)) if (key !== 'type' && key !== 'data') throw invalid('attachments')
  const { type, data } = attachment
  if (type !== 'image/png' && type !== 'image/jpeg') throw new HttpError(415, { error: 'unsupported_media_type' })
  if (typeof data !== 'string' || !/^[A-Za-z0-9+/]*={0,2}$/.test(data) || data.length % 4 !== 0) throw invalid('attachments')
  if ((data.length / 4) * 3 > MAX_ATTACHMENT + 2) throw new HttpError(413, { error: 'too_large' })
  const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0))
  if (bytes.length > MAX_ATTACHMENT) throw new HttpError(413, { error: 'too_large' })
  // The declared type must match the file's own signature.
  const png = bytes.length > 8 && [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].every((b, i) => bytes[i] === b)
  const jpeg = bytes.length > 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff
  if ((type === 'image/png' && !png) || (type === 'image/jpeg' && !jpeg)) throw new HttpError(415, { error: 'unsupported_media_type' })
  return { ext: png ? 'png' : 'jpg', base64: data }
}

function idempotencyKey(request) {
  const key = request.headers.get('idempotency-key')
  if (!key) return null
  // A random UUID or similar token; anything else is ignored rather than trusted.
  return /^[A-Za-z0-9-]{8,64}$/.test(key) ? key : null
}

async function sha256Hex(text) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text))
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('')
}

/** 5 reports per IP per hour, and 100 per day across all products. The IP is kept only as a
 *  salted hash, under a key that expires with its window. */
async function rateLimit(request, env, now = Date.now()) {
  const ip = request.headers.get('cf-connecting-ip') ?? 'unknown'
  const hour = Math.floor(now / 3_600_000)
  const day = Math.floor(now / 86_400_000)
  const ipKey = `rl:ip:${await sha256Hex(`${env.RATE_LIMIT_SALT}:${ip}`)}:${hour}`
  const dayKey = `rl:day:${day}`
  const [ipCount, dayCount] = await Promise.all([env.REPORTS_KV.get(ipKey), env.REPORTS_KV.get(dayKey)])
  const retry = (windowMs) => String(Math.max(1, Math.ceil((windowMs - (now % windowMs)) / 1000)))
  if (Number(ipCount ?? 0) >= PER_IP_PER_HOUR) throw new HttpError(429, { error: 'rate_limited' }, { 'retry-after': retry(3_600_000) })
  if (Number(dayCount ?? 0) >= PER_DAY) throw new HttpError(429, { error: 'rate_limited' }, { 'retry-after': retry(86_400_000) })
  await Promise.all([
    env.REPORTS_KV.put(ipKey, String(Number(ipCount ?? 0) + 1), { expirationTtl: 3600 + 60 }),
    env.REPORTS_KV.put(dayKey, String(Number(dayCount ?? 0) + 1), { expirationTtl: 86400 + 60 }),
  ])
}

async function github(env, method, path, body) {
  const response = await fetch(`${GITHUB}${path}`, {
    method,
    headers: {
      authorization: `Bearer ${env.GITHUB_TOKEN}`,
      accept: 'application/vnd.github+json',
      'x-github-api-version': '2022-11-28',
      'user-agent': 'app-reports-worker',
      ...(body ? { 'content-type': 'application/json' } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  })
  return response
}

/** A product is enabled when the label `product:<id>` exists (cached briefly in KV). */
async function productEnabled(env, product) {
  const cacheKey = `label:${product}`
  const cached = await env.REPORTS_KV.get(cacheKey)
  if (cached !== null) return cached === '1'
  const response = await github(env, 'GET', `/repos/${env.REPORTS_REPO}/labels/${encodeURIComponent(`product:${product}`)}`)
  if (response.status !== 200 && response.status !== 404) throw new HttpError(502, { error: 'upstream' })
  const enabled = response.status === 200
  await env.REPORTS_KV.put(cacheKey, enabled ? '1' : '0', { expirationTtl: LABEL_CACHE_TTL })
  return enabled
}

/** A fence longer than any run of backticks in `text`, so the text can never close it. */
export function fence(text) {
  const longest = Math.max(0, ...[...text.matchAll(/`+/g)].map((m) => m[0].length))
  return '`'.repeat(Math.max(3, longest + 1))
}

/** A short value shown as inline code, with anything that could break out of it removed. */
const code = (value) => `\`${String(value).replace(/[`\r\n]/g, '')}\``

export function title(report) {
  const firstLine = report.description.split(/\r?\n/)[0].replace(/[\u0000-\u001f]/g, '').trim()
  const short = [...firstLine].length > 60 ? `${[...firstLine].slice(0, 59).join('')}…` : firstLine
  return `[${report.product}] ${report.kind}: ${short}`
}

export function body(report, images = [], failed = false) {
  const summary = [
    `${code(report.product)} ${code(report.app_version)}`,
    `${code(report.os)} ${code(report.os_version)}`,
    code(report.arch),
    report.locale ? code(report.locale) : null,
    report.category ? `category ${code(report.category)}` : null,
  ].filter(Boolean).join(' · ')
  const descriptionFence = fence(report.description)
  const parts = [summary, '', `${descriptionFence}text`, report.description, descriptionFence]
  if (report.diagnostics !== undefined) {
    const diagnostics = JSON.stringify(report.diagnostics, null, 2)
    const f = fence(diagnostics)
    parts.push('', '<details><summary>Diagnostics</summary>', '', `${f}json`, diagnostics, f, '', '</details>')
  }
  if (images.length) parts.push('', ...images.map((url, i) => `![attachment ${i + 1}](${url})`))
  if (failed) parts.push('', '> An attachment could not be stored; the report was filed without it.')
  return parts.join('\n')
}

async function fileReport(env, report) {
  const labels = [`product:${report.product}`, `kind:${report.kind}`, ...(report.test ? ['test'] : [])]
  const created = await github(env, 'POST', `/repos/${env.REPORTS_REPO}/issues`, { title: title(report), body: body(report), labels })
  if (created.status !== 201) throw new HttpError(502, { error: 'upstream' })
  const { number } = await created.json()
  if (!report.files.length) return number

  const images = []
  let failed = false
  for (const [index, file] of report.files.entries()) {
    const path = `reports/${report.product}/${number}/${index + 1}.${file.ext}`
    const stored = await github(env, 'PUT', `/repos/${env.REPORTS_REPO}/contents/${path}`, {
      message: `${report.product} report #${number}: attachment ${index + 1}`,
      content: file.base64,
    })
    if (stored.status === 201 || stored.status === 200) images.push(`https://github.com/${env.REPORTS_REPO}/blob/main/${path}?raw=true`)
    else failed = true
  }
  // The issue exists either way: its number is returned, and a failed attachment is labelled.
  await github(env, 'PATCH', `/repos/${env.REPORTS_REPO}/issues/${number}`, { body: body(report, images, failed) })
  if (failed) await github(env, 'POST', `/repos/${env.REPORTS_REPO}/issues/${number}/labels`, { labels: ['attachment-failed'] })
  return number
}
