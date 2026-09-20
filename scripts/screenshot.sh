#!/usr/bin/env bash
# Captures one window of the running Baddel build into a file.
#   scripts/screenshot.sh <output.png> [--panel]
# Only Baddel's own window is captured, so nothing of the desktop behind it
# ends up in the documentation.
set -euo pipefail
cd "$(dirname "$0")/.."

out="$1"
shift || true
mkdir -p "$(dirname "$out")"

# The notice is a floating non-activating panel, which `screencapture -l` will not
# photograph, and it is translucent besides. Cut it out of a full screen capture.
if [[ " $* " == *" --panel "* ]]; then
  if ! bounds=$(swift scripts/window-id.swift --panel --bounds); then
    echo "no Baddel notice on screen" >&2
    exit 1
  fi
  full=$(mktemp -t baddel-screen).png
  trap 'rm -f "$full"' EXIT
  screencapture -x "$full"
  swift scripts/crop.swift "$full" "$out" $bounds
  echo "$out"
  exit 0
fi

if ! id=$(swift scripts/window-id.swift "$@"); then
  echo "no Baddel window on screen" >&2
  exit 1
fi

screencapture -x -o -l "$id" "$out"
echo "$out"
