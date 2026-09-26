#!/usr/bin/env sh
# gen-icons.sh - regenerate all Tauri app icons from icon-master.svg.
#
# `cargo tauri icon` accepts a squared SVG with transparency directly and
# renders every platform size itself (macOS .icns, Windows .ico, iOS,
# Android, Store logos) into apps/panya-app/icons/.
#
# Prerequisite: tauri-cli (`cargo install tauri-cli --locked`).
#
# Usage:
#   sh script/gen-icons.sh          # from the repository root
#
# Rebuild the app afterwards to apply the new icons:
#   cargo tauri build

set -eu

cd "$(dirname "$0")/.."

if [ ! -f icon-master.svg ]; then
  echo "error: icon-master.svg not found at the repository root" >&2
  exit 1
fi

cargo tauri icon icon-master.svg -o apps/panya-app/icons

echo "All icons generated in apps/panya-app/icons/."
