// Contract tests for the Worker, against an in-memory KV and a stand-in for the GitHub API.
// Run with `npm test` (node:test; no network, no Cloudflare account).
import assert from 'node:assert/strict'
import { beforeEach, test } from 'node:test'

import worker, { body, fence, title, validate } from '../src/index.js'

const PNG = Buffer.from('89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d4944415478da63f8cfc0f01f0005000201a3e2a8a10000000049454e44ae426082', 'hex').toString('base64')

function kv() {
  const store = new Map()
  return { store, get: async (k) => (store.has(k) ? store.get(k) : null), put: async (k, v) => void store.set(k, v) }
}

let env, calls, labels
beforeEach(() => {
  env = { GITHUB_TOKEN: 'token', RATE_LIMIT_SALT: 'salt', REPORTS_REPO: 'owner/app-reports', REPORTS_KV: kv() }
  calls = []
  labels = new Set(['product:baddel'])
  let issue = 41
  globalThis.fetch = async (url, init = {}) => {
    const path = new URL(url).pathname
    const method = init.method ?? 'GET'
    calls.push({ method, path, body: init.body ? JSON.parse(init.body) : undefined })
    if (method === 'GET' && path.startsWith('/repos/owner/app-reports/labels/')) {
      return new Response('{}', { status: labels.has(decodeURIComponent(path.split('/labels/')[1])) ? 200 : 404 })
    }
    if (method === 'POST' && path === '/repos/owner/app-reports/issues') return new Response(JSON.stringify({ number: ++issue }), { status: 201 })
    if (method === 'PUT' && path.includes('/contents/reports/')) return new Response('{}', { status: 201 })
    return new Response('{}', { status: 200 })
  }
})

const report = (extra = {}) => ({
  product: 'baddel', app_version: '1.1.0', os: 'macos', os_version: '27.0.1', arch: 'arm64', locale: 'ar',
  kind: 'bug', category: 'conversion', description: 'فتحوّلت الكلمات الأولى فقط', diagnostics: { accessibility: true }, ...extra,
})

function post(payload, headers = {}) {
  return new Request('https://app-reports.example.workers.dev/v1/reports', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'cf-connecting-ip': '203.0.113.9', ...headers },
    body: typeof payload === 'string' ? payload : JSON.stringify(payload),
  })
}

test('files an issue and returns its number', async () => {
  const response = await worker.fetch(post(report({ test: true })), env)
  assert.equal(response.status, 201)
  assert.deepEqual(await response.json(), { id: 42 })
  const created = calls.find((c) => c.method === 'POST' && c.path.endsWith('/issues'))
  assert.deepEqual(created.body.labels, ['product:baddel', 'kind:bug', 'test'])
  assert.match(created.body.title, /^\[baddel\] bug: فتحوّلت/)
  assert.equal(response.headers.get('access-control-allow-origin'), null)
})

test('stores the attachment under reports/<product>/<id>/ and embeds it', async () => {
  const response = await worker.fetch(post(report({ attachments: [{ type: 'image/png', data: PNG }] })), env)
  assert.equal(response.status, 201)
  const put = calls.find((c) => c.method === 'PUT')
  assert.equal(put.path, '/repos/owner/app-reports/contents/reports/baddel/42/1.png')
  const patch = calls.find((c) => c.method === 'PATCH')
  assert.match(patch.body.body, /!\[attachment 1\]\(https:\/\/github\.com\/owner\/app-reports\/blob\/main\/reports\/baddel\/42\/1\.png\?raw=true\)/)
})

test('rejects what the contract does not allow', async () => {
  const cases = [
    [report({ extra: 1 }), 400, { error: 'invalid_field', field: 'extra' }],
    [report({ product: 'Baddel' }), 400, { error: 'invalid_field', field: 'product' }],
    [report({ kind: 'feature' }), 400, { error: 'invalid_field', field: 'kind' }],
    [report({ description: '   ' }), 400, { error: 'invalid_field', field: 'description' }],
    [report({ description: 'x'.repeat(2001) }), 400, { error: 'invalid_field', field: 'description' }],
    [report({ attachments: [{ type: 'image/gif', data: PNG }] }), 415, { error: 'unsupported_media_type' }],
    [report({ attachments: [{ type: 'image/jpeg', data: PNG }] }), 415, { error: 'unsupported_media_type' }],
    [report({ diagnostics: { blob: 'x'.repeat(17000) } }), 413, { error: 'too_large' }],
    [report({ product: 'rasd' }), 400, { error: 'unknown_product' }],
  ]
  for (const [payload, status, error] of cases) {
    const response = await worker.fetch(post(payload), env)
    assert.equal(response.status, status, JSON.stringify(error))
    assert.deepEqual(await response.json(), error)
  }
  assert.equal((await worker.fetch(post('{nope'), env)).status, 400)
  assert.equal((await worker.fetch(post(report(), { 'content-type': 'text/plain' }), env)).status, 415)
  assert.equal((await worker.fetch(new Request('https://x.dev/v1/reports'), env)).status, 405)
  assert.equal((await worker.fetch(new Request('https://x.dev/'), env)).status, 404)
  assert.equal(calls.filter((c) => c.method === 'POST').length, 0)
})

test('replays an idempotency key instead of filing twice', async () => {
  const headers = { 'idempotency-key': '0f8b2c5e-2f7a-4a3e-9d7b-1c2d3e4f5a6b' }
  const first = await worker.fetch(post(report(), headers), env)
  const second = await worker.fetch(post(report(), headers), env)
  assert.equal(first.status, 201)
  assert.equal(second.status, 200)
  assert.deepEqual(await second.json(), { id: 42 })
  assert.equal(calls.filter((c) => c.method === 'POST' && c.path.endsWith('/issues')).length, 1)
})

test('limits reports per IP, keyed by a salted hash', async () => {
  for (let i = 0; i < 5; i++) assert.equal((await worker.fetch(post(report()), env)).status, 201)
  const limited = await worker.fetch(post(report()), env)
  assert.equal(limited.status, 429)
  assert.ok(Number(limited.headers.get('retry-after')) > 0)
  const keys = [...env.REPORTS_KV.store.keys()].join(' ')
  assert.ok(!keys.includes('203.0.113.9'), 'the IP itself is never stored')
  // Another network still gets through.
  assert.equal((await worker.fetch(post(report(), { 'cf-connecting-ip': '198.51.100.7' }), env)).status, 201)
})

test('a failed attachment still returns the number, labelled', async () => {
  const original = globalThis.fetch
  globalThis.fetch = async (url, init = {}) => (init.method === 'PUT' ? new Response('{}', { status: 500 }) : original(url, init))
  const response = await worker.fetch(post(report({ attachments: [{ type: 'image/png', data: PNG }] })), env)
  assert.equal(response.status, 201)
  const labelled = calls.find((c) => c.path.endsWith('/issues/42/labels'))
  assert.deepEqual(labelled.body.labels, ['attachment-failed'])
})

test('user text never renders as Markdown', () => {
  const text = 'hi @owner ``` <img src=x> [link](https://evil)'
  const r = { ...validate(report({ description: text })), diagnostics: { a: 1 } }
  const rendered = body(r)
  assert.equal(fence(text), '````')
  assert.ok(rendered.includes(`\`\`\`\`text\n${text}\n\`\`\`\``))
  assert.equal(title({ product: 'baddel', kind: 'bug', description: 'a'.repeat(80) }).length, '[baddel] bug: '.length + 60)
})
