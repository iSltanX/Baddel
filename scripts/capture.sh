#!/usr/bin/env bash
# Captures the documentation screenshots from the built debug app: every screen,
# in both languages and both appearances. Needs a debug build, because the window,
# pane and appearance are chosen through the debug-only BADDEL_* variables.
#
#   ./scripts/dev-build.sh && ./scripts/capture.sh
set -euo pipefail
cd "$(dirname "$0")/.."

app="target/debug/bundle/macos/Baddel.app/Contents/MacOS/baddel"
store="$HOME/Library/Application Support/com.isltanx.baddel/settings.json"
out="docs/screenshots"
windows=(
  settings:general settings:shortcuts settings:layouts settings:exceptions settings:about
  onboarding:1 onboarding:2 onboarding:3
)
# The notice is dark in both appearances by design, so it is captured once per
# language. It also draws itself to file: macOS will not let `screencapture`
# photograph a floating, non-activating panel.
notices=(success undone blocked too-long no-text)

[ -x "$app" ] || { echo "build first: ./scripts/dev-build.sh" >&2; exit 1; }
mkdir -p "$out" "$(dirname "$store")"

# The captures rewrite the settings file and stop any running Baddel. Put both back
# afterwards: the user's own settings, and their Baddel relaunched in the background.
running=$(pgrep -x baddel | head -1 | xargs -r ps -o comm= -p || true)
backup=$(mktemp -t baddel-settings)
[ -f "$store" ] && cp "$store" "$backup"
restore() {
  pkill -x baddel 2>/dev/null || true
  if [ -s "$backup" ]; then cp "$backup" "$store"; else rm -f "$store"; fi
  rm -f "$backup"
  if [ -n "$running" ]; then
    for _ in $(seq 40); do pgrep -x baddel >/dev/null || break; sleep 0.25; done
    open -g "${running%/Contents/MacOS/*}"
    echo "relaunched ${running%/Contents/MacOS/*}"
  fi
}
trap restore EXIT

launch() { # launch <appearance> <window> [capture-path]
  # Wait for the previous run to be gone: its window lingers for a moment after the
  # signal, and the capture would otherwise photograph the wrong screen.
  pkill -x baddel 2>/dev/null || true
  for _ in $(seq 40); do
    pgrep -x baddel >/dev/null || break
    sleep 0.25
  done
  sleep 0.5
  # A pointer resting over the window would photograph a row or button in its hover state.
  swift scripts/park-cursor.swift
  BADDEL_THEME="$1" BADDEL_WINDOW="$2" BADDEL_HUD_CAPTURE="${3:-}" "$app" >/dev/null 2>&1 &
  sleep 3
}

for language in ar en; do
  printf '{ "settings": { "language": "%s", "welcomed": true } }\n' "$language" > "$store"

  for appearance in light dark; do
    for window in "${windows[@]}"; do
      name="${window//:/-}-$language-$appearance"
      launch "$appearance" "$window"
      if ./scripts/screenshot.sh "$out/$name.png" >/dev/null 2>&1; then
        echo "✓ $name"
      else
        echo "✗ $name"
      fi
    done
  done

  for notice in "${notices[@]}"; do
    name="hud-$notice-$language"
    launch dark "hud:$notice" "$PWD/$out/$name.png"
    [ -s "$out/$name.png" ] && echo "✓ $name" || echo "✗ $name"
  done
done

echo "screenshots in $out"
