#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
VERSION="$(node -p "require('./package.json').version")"
ARTIFACTS="$ROOT/artifacts/linux"
RELEASE="$ROOT/src-tauri/target/release"

echo '=== GitFlic Contour Sync Linux Release ==='
echo "Root: $ROOT"
for cmd in node npm cargo rustc; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "Не найден $cmd" >&2; exit 1; }
done
node --version
npm --version
rustc --version
cargo --version

echo '[1/3] Production build...'
npm run tauri:build

echo '[2/3] Collecting artifacts...'
rm -rf "$ARTIFACTS"
mkdir -p "$ARTIFACTS"
BIN="$RELEASE/gitflic-contour-sync"
[ -f "$BIN" ] || { echo "Не найден Linux binary: $BIN" >&2; exit 1; }
cp "$BIN" "$ARTIFACTS/GitFlic-Contour-Sync"
chmod +x "$ARTIFACTS/GitFlic-Contour-Sync"
find "$RELEASE/bundle" -type f \( -name '*.deb' -o -name '*.AppImage' \) -exec cp {} "$ARTIFACTS/" \; 2>/dev/null || true
(
  cd "$ARTIFACTS"
  sha256sum * > SHA256SUMS.txt
)
mkdir -p "$ROOT/artifacts"
ZIP="$ROOT/artifacts/GitFlic-Contour-Sync-v${VERSION}-linux.zip"
rm -f "$ZIP"
if command -v zip >/dev/null 2>&1; then
  (cd "$ARTIFACTS" && zip -9 -q "$ZIP" ./*)
else
  TAR="$ROOT/artifacts/GitFlic-Contour-Sync-v${VERSION}-linux.tar.gz"
  rm -f "$TAR"
  tar -C "$ARTIFACTS" -czf "$TAR" .
fi

echo '[3/3] Result'
ls -lh "$ARTIFACTS"
echo "Artifacts: $ARTIFACTS"
