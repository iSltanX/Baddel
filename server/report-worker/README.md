# app-reports Worker

The shared report Worker for the apps published under iSltanX. An app posts a problem report here; the Worker files it as an issue in the private repository `iSltanX/app-reports` and stores its attachments under `reports/<product>/<id>/`.

**The contract is that repository's README (v1).** This Worker implements it as written, with nothing specific to one app: a product is enabled by creating its `product:<id>` label there. Baddel hosts the source only because it is the first product.

## What it does

1. Accepts `POST /v1/reports` with `Content-Type: application/json` and a body up to 16 MB; anything else gets `404`, `405`, `413` or `415`.
2. Validates every field against the contract and rejects unknown ones (`400 invalid_field`).
3. Replays a known `Idempotency-Key` with `200 { "id": … }` instead of filing twice (keys kept 24 hours). A request whose key is still being filed waits for that filing and gets the same number.
4. Applies the limits before anything costs a GitHub call: 5 reports per IP per hour, 100 per day across all products (`429` with `Retry-After`). The IP is kept only as a salted SHA-256 hash, in a key that expires with its window.
5. Requires the label `product:<id>` (`400 unknown_product`; the answer is cached in KV for 10 minutes).
6. Opens the issue (`[<product>] <kind>: <first line>`, with control characters and bidi overrides removed from that line; labels `product:<id>`, `kind:<kind>` and `test` when set), with the description in a fence longer than any backtick run in it so user text never renders as Markdown, and `diagnostics` as JSON in `<details>`.
7. Checks each attachment's signature, stores it at `reports/<product>/<id>/<n>.<png|jpg>`, embeds it in the issue, and returns `201 { "id": <issue number> }`. Once the issue exists its number is always returned and its key kept: a failed attachment, or a failed update that leaves the issue without its image links, adds the `attachment-failed` label, and no later network error turns the number into a `502`.

**Limits of the guarantees in 3 and 4.** KV has no atomic read-and-write and is eventually consistent, so the Worker serialises the idempotency check and the rate-limit count within one isolate. Two requests that reach different isolates or locations at the same instant can still both pass; a strict guarantee would need a Durable Object.

No CORS headers: reports come from apps, not web pages. Nothing is logged by the Worker.

## Configuration

| Name | Kind | Where | Purpose |
|---|---|---|---|
| `GITHUB_TOKEN` | secret | `wrangler secret put` (done by the deploy script) | fine-grained token with access to **only** `iSltanX/app-reports`: Issues and Contents, read and write |
| `RATE_LIMIT_SALT` | secret | generated once by the deploy script | salts the IP hash; never changed afterwards |
| `REPORTS_REPO` | variable | `wrangler.toml` | `iSltanX/app-reports` |
| `REPORTS_KV` | KV binding | `wrangler.toml` (id written by the first deploy) | rate-limit counters, idempotency keys, product-label cache — all with TTLs |

Deploy credentials come from the environment or the repository root's git-ignored `.env`: `CLOUDFLARE_API_TOKEN` (template "Edit Cloudflare Workers"), `CLOUDFLARE_ACCOUNT_ID`, and `REPORTS_GITHUB_TOKEN` (becomes the secret `GITHUB_TOKEN`). No value belongs in this folder or in any app.

## Deploy

```sh
cd server/report-worker
npm install
npm test
./scripts/deploy.sh
```

The first run creates the KV namespace and writes its id into `wrangler.toml`, deploys, generates `RATE_LIMIT_SALT`, stores `GITHUB_TOKEN`, and prints the endpoint (`https://app-reports.<account-subdomain>.workers.dev/v1/reports`). Later runs redeploy and refresh the token. When the GitHub token expires, every report fails with `502`: renew it in `.env` and run the script again.

## Test

`npm test` runs the contract tests in `test/` against an in-memory KV and a stand-in GitHub API, without network or a Cloudflare account.
