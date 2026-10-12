// Audit (Copilot, 2026): idempotency, rate-limit and upstream-failure behaviour of the Worker.
// In-memory KV and a stand-in GitHub API, as in worker.test.js; no network.
// Tests named "BAD-xxx" are FAILING BY DESIGN: each demonstrates a defect listed in
// BADDEL-COPILOT-AUDIT.md and should pass once that defect is fixed.
import assert from 'node:assert/strict'
import { beforeEach, test } from 'node:test'

import worker from '../src/index.js'

const PNG = Buffer.from('89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d4944415478da63f8cfc0f01f0005000201a3e2a8a10000000049454e44ae426082', 'hex').toString('base64')

/** KV whose operations yield to the event loop, like the real (remote) KV. */
function kv() {
  const store = new Map()
  const tick = () => new Promise((r) => setTimeout(r, 1))
  return {
    store,
    get: async (k) => { await tick(); return store.has(k) ? store.get(k) : null },
    put: async (k, v) => { await tick(); store.set(k, v) },
  }
}

let env, calls, fail
beforeEach(() => {
  env = { GITHUB_TOKEN: 'token', RATE_LIMIT_SALT: 'salt', REPORTS_REPO: 'owner/app-reports', REPORTS_KV: kv() }
  calls = []
  fail = {} // method -> 'throw' | status
  let issue = 41
  globalThis.fetch = async (url, init = {}) => {
    const path = new URL(url).pathname
    const method = init.method ?? 'GET'
    calls.push({ method, path })
    await new Promise((r) => setTimeout(r, 2))
    const isLabelPost = method === 'POST' && path.endsWith('/labels')
    const mode = fail[isLabelPost ? 'LABELS' : method]
    if (mode === 'throw') throw new TypeError('network connection lost')
    if (typeof mode === 'number') return new Response('{}', { status: mode })
    if (method === 'GET' && path.includes('/labels/')) {
      return new Response('{}', { status: decodeURIComponent(path.split('/labels/')[1]) === 'product:baddel' ? 200 : 404 })
    }
    if (method === 'POST' && path === '/repos/owner/app-reports/issues') return new Response(JSON.stringify({ number: ++issue }), { status: 201 })
    if (method === 'PUT') return new Response('{}', { status: 201 })
    return new Response('{}', { status: 200 })
  }
})

const report = (extra = {}) => ({
  product: 'baddel', app_version: '1.1.0', os: 'macos', os_version: '27.0.1', arch: 'arm64',
  kind: 'bug', description: 'audit', ...extra,
})
const post = (payload, headers = {}) => new Request('https://w.example/v1/reports', {
  method: 'POST',
  headers: { 'content-type': 'application/json', 'cf-connecting-ip': '203.0.113.9', ...headers },
  body: JSON.stringify(payload),
})
const issuesFiled = () => calls.filter((c) => c.method === 'POST' && c.path === '/repos/owner/app-reports/issues').length

// ── Idempotency ──────────────────────────────────────────────────────────────

test('sequential retry with the same key replays the id and files once', async () => {
  const h = { 'idempotency-key': 'aaaaaaaa-1111' }
  const a = await worker.fetch(post(report(), h), env)
  const b = await worker.fetch(post(report(), h), env)
  assert.equal(a.status, 201)
  assert.equal(b.status, 200)
  assert.deepEqual(await b.json(), { id: 42 })
  assert.equal(issuesFiled(), 1)
})

test('BAD-101: two concurrent requests with the same Idempotency-Key file only one issue', async () => {
  const h = { 'idempotency-key': 'bbbbbbbb-2222' }
  const [a, b] = await Promise.all([worker.fetch(post(report(), h), env), worker.fetch(post(report(), h), env)])
  const ids = [(await a.json()).id, (await b.json()).id]
  assert.equal(issuesFiled(), 1, `filed ${issuesFiled()} issues, ids ${ids}`)
  assert.equal(ids[0], ids[1])
})

test('characterisation: the key is not bound to the payload — a different report replays the old id', async () => {
  const h = { 'idempotency-key': 'cccccccc-3333' }
  await worker.fetch(post(report({ description: 'first' }), h), env)
  const r = await worker.fetch(post(report({ description: 'entirely different' }), h), env)
  assert.equal(r.status, 200)
  assert.deepEqual(await r.json(), { id: 42 })
  assert.equal(issuesFiled(), 1)
})

test('a failed issue POST stores no key, so a retry can file it', async () => {
  const h = { 'idempotency-key': 'dddddddd-4444' }
  fail.POST = 500
  assert.equal((await worker.fetch(post(report(), h), env)).status, 502)
  delete fail.POST
  const r = await worker.fetch(post(report(), h), env)
  assert.equal(r.status, 201)
})

test('BAD-102: a network error on the PATCH after the issue exists still returns its number (no 502, no duplicate on retry)', async () => {
  const h = { 'idempotency-key': 'eeeeeeee-5555' }
  fail.PATCH = 'throw'
  const first = await worker.fetch(post(report({ attachments: [{ type: 'image/png', data: PNG }] }), h), env)
  delete fail.PATCH
  const retry = await worker.fetch(post(report({ attachments: [{ type: 'image/png', data: PNG }] }), h), env)
  assert.equal(issuesFiled(), 1, `first=${first.status}, retry=${retry.status}, issues filed=${issuesFiled()}`)
  assert.equal(first.status, 201)
})

test('BAD-102 (variant): a network error adding the attachment-failed label also loses the number', async () => {
  fail.PUT = 500
  fail.LABELS = 'throw'
  const r = await worker.fetch(post(report({ attachments: [{ type: 'image/png', data: PNG }] })), env)
  assert.equal(issuesFiled(), 1)
  assert.equal(r.status, 201, 'README: "A failed attachment still returns the number"')
})

test('characterisation: a 5xx on the PATCH is ignored — 201 is returned and the issue body keeps no image links', async () => {
  fail.PATCH = 503
  const r = await worker.fetch(post(report({ attachments: [{ type: 'image/png', data: PNG }] })), env)
  assert.equal(r.status, 201)
  assert.ok(!calls.some((c) => c.path.endsWith('/labels') && c.method === 'POST'), 'no attachment-failed label either')
})

test('a replayed key is answered before the rate limit (retries are never 429)', async () => {
  const h = { 'idempotency-key': 'ffffffff-6666' }
  await worker.fetch(post(report(), h), env)
  for (let i = 0; i < 6; i++) await worker.fetch(post(report()), env)
  assert.equal((await worker.fetch(post(report(), h), env)).status, 200)
})

// ── Rate limit ───────────────────────────────────────────────────────────────

test('sequential: the sixth report from one IP within the hour is 429 with Retry-After', async () => {
  for (let i = 0; i < 5; i++) assert.equal((await worker.fetch(post(report()), env)).status, 201)
  const r = await worker.fetch(post(report()), env)
  assert.equal(r.status, 429)
  assert.ok(Number(r.headers.get('retry-after')) >= 1)
})

test('BAD-103: a concurrent burst from one IP is held to 5 issues per hour', async () => {
  const rs = await Promise.all(Array.from({ length: 20 }, () => worker.fetch(post(report()), env)))
  const ok = rs.filter((r) => r.status === 201).length
  assert.ok(ok <= 5, `${ok} of 20 concurrent reports were filed (limit 5/hour)`)
})

test('characterisation: requests without cf-connecting-ip share one "unknown" bucket', async () => {
  const noIp = (p) => new Request('https://w.example/v1/reports', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(p) })
  for (let i = 0; i < 5; i++) await worker.fetch(noIp(report()), env)
  assert.equal((await worker.fetch(noIp(report()), env)).status, 429)
})

test('BAD-104: unknown products are not rate-limited and each costs an authenticated GitHub API call', async () => {
  const n = 30
  const rs = []
  for (let i = 0; i < n; i++) rs.push(await worker.fetch(post(report({ product: `nope-${i}` })), env))
  const ghCalls = calls.filter((c) => c.method === 'GET').length
  // Approved change (owner, fix/audit-findings): once the IP is over the limit the answer is 429,
  // since the limit now comes before the label lookup — which is the fix this test asks for.
  assert.ok(rs.every((r) => r.status === 400 || r.status === 429))
  assert.ok(ghCalls <= 5, `${ghCalls} GitHub label lookups for ${n} requests from one IP; none was rate-limited`)
})
