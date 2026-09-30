#!/usr/bin/env bash
# Deploys the shared report Worker with the owner's Cloudflare account.
#
#   ./scripts/deploy.sh
#
# Reads from the environment, or from the repository root's git-ignored .env:
#   CLOUDFLARE_API_TOKEN   token from the "Edit Cloudflare Workers" template
#   CLOUDFLARE_ACCOUNT_ID
#   REPORTS_GITHUB_TOKEN   fine-grained token for iSltanX/app-reports only (Issues + Contents: write);
#                          stored as the Worker secret GITHUB_TOKEN
#   REPORTS_REPO           iSltanX/app-reports (optional; wrangler.toml has it)
#
# First run: creates the KV namespace and writes its id into wrangler.toml, and generates
# RATE_LIMIT_SALT. Later runs redeploy the code and refresh GITHUB_TOKEN. No value is printed.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$(git rev-parse --show-toplevel)"

if [ -f "$ROOT/.env" ]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
fi
: "${CLOUDFLARE_API_TOKEN:?missing — see the header of this script}"
: "${CLOUDFLARE_ACCOUNT_ID:?missing — see the header of this script}"
: "${REPORTS_GITHUB_TOKEN:?missing — see the header of this script}"
export CLOUDFLARE_API_TOKEN CLOUDFLARE_ACCOUNT_ID
# No usage telemetry from wrangler.
export WRANGLER_SEND_METRICS=false

[ -d node_modules ] || npm install --no-audit --no-fund
WRANGLER="npx --no-install wrangler"

# 1. KV namespace (once).
if grep -q 'id = "REPLACED_BY_DEPLOY"' wrangler.toml; then
  ID=$($WRANGLER kv namespace list | node -e '
    let s = ""; process.stdin.on("data", d => s += d).on("end", () => {
      const list = JSON.parse(s.slice(s.indexOf("[")));
      const found = list.find(n => n.title === "app-reports-REPORTS_KV" || n.title === "REPORTS_KV");
      process.stdout.write(found ? found.id : "");
    });')
  if [ -z "$ID" ]; then
    ID=$($WRANGLER kv namespace create REPORTS_KV 2>&1 | sed -nE 's/.*id = "([0-9a-f]{32})".*/\1/p' | head -1)
  fi
  [ -n "$ID" ] || { echo "could not create or find the KV namespace" >&2; exit 1; }
  sed -i '' "s/id = \"REPLACED_BY_DEPLOY\"/id = \"$ID\"/" wrangler.toml
  echo "KV namespace: $ID (written to wrangler.toml)"
fi

# 2. Code.
$WRANGLER deploy

# 3. Secrets. RATE_LIMIT_SALT is generated once and never changed (it would reset the limits).
if ! $WRANGLER secret list 2>/dev/null | grep -q '"RATE_LIMIT_SALT"'; then
  openssl rand -hex 32 | $WRANGLER secret put RATE_LIMIT_SALT >/dev/null
  echo "RATE_LIMIT_SALT: generated"
fi
printf '%s' "$REPORTS_GITHUB_TOKEN" | $WRANGLER secret put GITHUB_TOKEN >/dev/null
echo "GITHUB_TOKEN: updated"

SUBDOMAIN=$(curl -fsS -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
  "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/workers/subdomain" |
  node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>process.stdout.write(JSON.parse(s).result.subdomain||""))')
echo "endpoint: https://app-reports.$SUBDOMAIN.workers.dev/v1/reports"
