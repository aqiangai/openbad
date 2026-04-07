#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICONS_DIR="$ROOT/src-tauri/icons"
TMP_DIR="$(mktemp -d)"
ICONSET_DIR="$TMP_DIR/OpenBad.iconset"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

render_svg_png() {
  local svg_path="$1"
  local size="$2"
  local output_path="$3"
  qlmanage -t -s "$size" -o "$TMP_DIR" "$svg_path" >/dev/null 2>&1
  local rendered="$TMP_DIR/$(basename "$svg_path").png"
  mv "$rendered" "$output_path"
}

resize_png() {
  local input_path="$1"
  local size="$2"
  local output_path="$3"
  cp "$input_path" "$output_path"
  sips -z "$size" "$size" "$output_path" >/dev/null
}

mkdir -p "$ICONSET_DIR"

MASTER_PNG="$TMP_DIR/master-1024.png"
render_svg_png "$ICONS_DIR/logo-source.svg" 1024 "$MASTER_PNG"

resize_png "$MASTER_PNG" 32 "$ICONS_DIR/32x32.png"
resize_png "$MASTER_PNG" 128 "$ICONS_DIR/128x128.png"
resize_png "$MASTER_PNG" 256 "$ICONS_DIR/128x128@2x.png"
resize_png "$MASTER_PNG" 512 "$ICONS_DIR/icon.png"

resize_png "$MASTER_PNG" 30 "$ICONS_DIR/Square30x30Logo.png"
resize_png "$MASTER_PNG" 44 "$ICONS_DIR/Square44x44Logo.png"
resize_png "$MASTER_PNG" 71 "$ICONS_DIR/Square71x71Logo.png"
resize_png "$MASTER_PNG" 89 "$ICONS_DIR/Square89x89Logo.png"
resize_png "$MASTER_PNG" 107 "$ICONS_DIR/Square107x107Logo.png"
resize_png "$MASTER_PNG" 142 "$ICONS_DIR/Square142x142Logo.png"
resize_png "$MASTER_PNG" 150 "$ICONS_DIR/Square150x150Logo.png"
resize_png "$MASTER_PNG" 284 "$ICONS_DIR/Square284x284Logo.png"
resize_png "$MASTER_PNG" 310 "$ICONS_DIR/Square310x310Logo.png"
resize_png "$MASTER_PNG" 50 "$ICONS_DIR/StoreLogo.png"

resize_png "$MASTER_PNG" 16 "$ICONSET_DIR/icon_16x16.png"
resize_png "$MASTER_PNG" 32 "$ICONSET_DIR/icon_16x16@2x.png"
resize_png "$MASTER_PNG" 32 "$ICONSET_DIR/icon_32x32.png"
resize_png "$MASTER_PNG" 64 "$ICONSET_DIR/icon_32x32@2x.png"
resize_png "$MASTER_PNG" 128 "$ICONSET_DIR/icon_128x128.png"
resize_png "$MASTER_PNG" 256 "$ICONSET_DIR/icon_128x128@2x.png"
resize_png "$MASTER_PNG" 256 "$ICONSET_DIR/icon_256x256.png"
resize_png "$MASTER_PNG" 512 "$ICONSET_DIR/icon_256x256@2x.png"
resize_png "$MASTER_PNG" 512 "$ICONSET_DIR/icon_512x512.png"
cp "$MASTER_PNG" "$ICONSET_DIR/icon_512x512@2x.png"

iconutil -c icns "$ICONSET_DIR" -o "$ICONS_DIR/icon.icns"
ICO_MASTER="$TMP_DIR/icon-256.png"
resize_png "$MASTER_PNG" 256 "$ICO_MASTER"
ffmpeg -loglevel error -y -i "$ICO_MASTER" "$ICONS_DIR/icon.ico"

TRAY_PNG="$TMP_DIR/tray-template.png"
render_svg_png "$ICONS_DIR/tray-template-source.svg" 64 "$TRAY_PNG"
cp "$TRAY_PNG" "$ICONS_DIR/trayTemplate.png"

cp "$ROOT/src/assets/openbad-mark.svg" "$ROOT/public/favicon.svg"

echo "Updated icon set in $ICONS_DIR"
