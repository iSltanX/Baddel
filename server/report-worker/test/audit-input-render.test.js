// Audit (Copilot, 2026): input validation edges and issue rendering of the Worker.
// Tests named "BAD-xxx" are FAILING BY DESIGN (see BADDEL-COPILOT-AUDIT.md).
import assert from 'node:assert/strict'
import { beforeEach, test } from 'node:test'

import worker, { body, title, validate } from '../src/index.js'

function kv() {
  const store = new Map()
  return { store, get: async (k) => (store.has(k) ? store.get(k) : null), put: async (k, v) => void store.set(k, v) }
}
let env, calls
beforeEach(() => {
  env = { GITHUB_TOKEN: 'token', RATE_LIMIT_SALT: 'salt', REPORTS_REPO: 'owner/app-reports', REPORTS_KV: kv() }
  calls = []
  let issue = 41
  globalThis.fetch = async (url, init = {}) => {
    const path = new URL(url).pathname
    const method = init.method ?? 'GET'
    calls.push({ method, path, body: init.body ? JSON.parse(init.body) : undefined })
    if (method === 'GET') return new Response('{}', { status: path.endsWith(encodeURIComponent('product:baddel')) ? 200 : 404 })
    if (method === 'POST' && path.endsWith('/issues')) return new Response(JSON.stringify({ number: ++issue }), { status: 201 })
    return new Response('{}', { status: 201 })
  }
})
const report = (extra = {}) => ({
  product: 'baddel', app_version: '1.1.0', os: 'macos', os_version: '27.0.1', arch: 'arm64', kind: 'bug', description: 'audit', ...extra,
})
const req = (raw, headers = {}) => new Request('https://w.example/v1/reports', {
  method: 'POST', headers: { 'content-type': 'application/json', 'cf-connecting-ip': '198.51.100.1', ...headers }, body: raw,
})
const status = async (raw, headers) => (await worker.fetch(req(raw, headers), env)).status
const code = (fn) => { try { fn(); return 'ok' } catch (e) { return `${e.status}:${e.body.error}${e.body.field ? `:${e.body.field}` : ''}` } }

// ── Transport ────────────────────────────────────────────────────────────────

test('content-type variants: charset suffix and case are accepted, others 415', async () => {
  assert.equal(await status(JSON.stringify(report()), { 'content-type': 'Application/JSON; charset=utf-8' }), 201)
  assert.equal(await status(JSON.stringify(report()), { 'content-type': 'text/plain' }), 415)
})

test('a body over 16 MB is 413 from the streaming cap', async () => {
  const big = 'x'.repeat(16 * 1024 * 1024 + 1)
  // Request recomputes content-length from the body; the streaming cap is what actually protects.
  assert.equal(await status(big), 413)
})

test('malformed JSON, arrays, null and __proto__ keys are 400', async () => {
  assert.equal(await status('{'), 400)
  assert.equal(await status('[]'), 400)
  assert.equal(await status('null'), 400)
  assert.equal(await status(JSON.stringify(report()).replace('{', '{"__proto__":{"x":1},')), 400)
})

// ── Fields ───────────────────────────────────────────────────────────────────

test('description: limits count trimmed code points (2000), not UTF-16 units', () => {
  assert.equal(code(() => validate(report({ description: '😀'.repeat(2000) }))), 'ok')
  assert.equal(code(() => validate(report({ description: '😀'.repeat(2001) }))), '400:invalid_field:description')
  assert.equal(code(() => validate(report({ description: '  \n\t ' }))), '400:invalid_field:description')
  assert.equal(code(() => validate(report({ description: ` ${'a'.repeat(2000)} ` }))), 'ok')
})

test('characterisation: short fields count UTF-16 units while description counts code points', () => {
  assert.equal(code(() => validate(report({ arch: '😀'.repeat(8) }))), 'ok') // 16 units
  assert.equal(code(() => validate(report({ arch: '😀'.repeat(9) }))), '400:invalid_field:arch')
})

test('diagnostics: 16384 compact UTF-8 bytes is the limit', () => {
  const at = (n) => ({ d: 'x'.repeat(n - 8) }) // {"d":""} is 8 bytes
  assert.equal(code(() => validate(report({ diagnostics: at(16384) }))), 'ok')
  assert.equal(code(() => validate(report({ diagnostics: at(16385) }))), '413:too_large')
  assert.equal(code(() => validate(report({ diagnostics: [] }))), '400:invalid_field:diagnostics')
})

test('attachments: count, keys, padding and signature', () => {
  const png = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0]).toString('base64')
  const jpg = Buffer.from([0xff, 0xd8, 0xff, 0]).toString('base64')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: png }] }))), 'ok')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/jpeg', data: jpg }] }))), 'ok')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: jpg }] }))), '415:unsupported_media_type')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/gif', data: png }] }))), '415:unsupported_media_type')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: png, name: 'x' }] }))), '400:invalid_field:attachments')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: png.slice(1) }] }))), '400:invalid_field:attachments')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: `A=${png.slice(2)}` }] }))), '400:invalid_field:attachments')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: '' }] }))), '415:unsupported_media_type')
  assert.equal(code(() => validate(report({ attachments: Array(4).fill({ type: 'image/png', data: png }) }))), '400:invalid_field:attachments')
})

test('attachment size: exactly 3 MiB decoded passes, one byte more is 413', () => {
  const mk = (n) => { const b = Buffer.alloc(n); b.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]); return b.toString('base64') }
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: mk(3 * 1024 * 1024) }] }))), 'ok')
  assert.equal(code(() => validate(report({ attachments: [{ type: 'image/png', data: mk(3 * 1024 * 1024 + 1) }] }))), '413:too_large')
})

// ── Rendering ────────────────────────────────────────────────────────────────

test('description cannot escape its fence or render Markdown/HTML', () => {
  const evil = '```\n</details><img src=x onerror=alert(1)>\n````\n@iSltanX #1'
  const b = body(validate(report({ description: evil, diagnostics: { k: '```` </details>' } })))
  const f = '`````'
  assert.ok(b.includes(`${f}text\n${evil}\n${f}`))
})

test('inline fields cannot break out of their code spans', () => {
  const b = body(validate(report({ app_version: '1`<b>x</b>\n# h', os_version: '``' })))
  assert.ok(b.startsWith('`baddel` `1<b>x</b># h`'))
})

test('BAD-105: bidi control characters are removed from the issue title', () => {
  const t = title(validate(report({ description: 'abc\u202Efdp.exe\u2066x\u2069' })))
  assert.ok(!/[\u202A-\u202E\u2066-\u2069\u200E\u200F]/.test(t), `title keeps bidi controls: ${JSON.stringify(t)}`)
})

test('characterisation: the title is cut at 59 code points, which can split a grapheme', () => {
  const t = title(validate(report({ description: `${'a'.repeat(58)}👩‍👩‍👧 tail` })))
  assert.equal(t, `[baddel] bug: ${'a'.repeat(58)}👩…`)
})

test('title uses only the first line and strips C0 controls', () => {
  assert.equal(title(validate(report({ description: 'one\u0007two\r\nthree' }))), '[baddel] bug: onetwo')
})
