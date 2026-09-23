#!/usr/bin/env bash
# Renders design/brand/cover.html and readme.html into the repository images, with a headless
# Chrome: no window opens and nothing takes focus.
#
#   ./design/brand/render.sh
#
# docs/assets/social-preview.png                 GitHub social preview, 1280×640
# docs/assets/header-{ar,en}-{light,dark}.png     README header, 1280×440 @2x   (readme.html)
# docs/assets/steps-{ar,en}-{light,dark}.png      README "how it works", 1280×420 @2x
set -euo pipefail
cd "$(dirname "$0")/../.."

chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
[ -x "$chrome" ] || { echo "Google Chrome is required" >&2; exit 1; }

brand="file://$PWD/design/brand"
page="$brand/cover.html"
out="docs/assets"
profile=$(mktemp -d -t baddel-render)
trap 'rm -rf "$profile"' EXIT
mkdir -p "$out"

render() { # render <query> <width> <height> <scale> <file>
  # Headless Chrome writes the file and then may linger, so wait for the file and
  # stop it ourselves.
  rm -f "$5"
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --no-first-run \
    --user-data-dir="$profile" --allow-file-access-from-files \
    --force-device-scale-factor="$4" --window-size="$2,$3" \
    --screenshot="$5" "$page?$1" >/dev/null 2>&1 &
  local pid=$!
  for _ in $(seq 100); do
    [ -s "$5" ] && break
    sleep 0.2
  done
  sleep 0.5
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  pkill -f "$profile" 2>/dev/null || true
  [ -s "$5" ] || { echo "✗ $5" >&2; return 1; }
  echo "✓ $5 ($(sips -g pixelWidth -g pixelHeight "$5" | awk '/pixel/ {printf "%s ", $2}'))"
}

render "theme=light&kind=social" 1280 640 1 "$out/social-preview.png"

page="$brand/readme.html"
for lang in ar en; do
  for theme in light dark; do
    render "theme=$theme&kind=header&lang=$lang" 1280 440 2 "$out/header-$lang-$theme.png"
    render "theme=$theme&kind=steps&lang=$lang" 1280 420 2 "$out/steps-$lang-$theme.png"
  done
done
