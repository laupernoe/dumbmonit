#!/bin/sh
# Assembles a DSM 7 package (.spk) around an already built agent binary.
#
#   packaging/synology/build-spk.sh <x86_64|aarch64> <path-to-agent-binary> <version>
#
# Output: dist/dumbmonit-agent-<version>-<arch>.spk (relative to the current directory).
# Icons: resized from web/static/icon-192.png / icon-512.png with ImageMagick
# (magick or convert) when available; otherwise the originals are copied as is
# (DSM scales them, but 72 and 256 px files are preferred).
set -eu

[ "$#" -eq 3 ] || { echo "usage: $0 <x86_64|aarch64> <agent-binary> <version>" >&2; exit 1; }
ARCH="$1"
BINARY="$2"
VERSION="$3"

case "$ARCH" in
    x86_64) SPK_ARCH="x86_64" ;;
    aarch64) SPK_ARCH="aarch64" ;;
    *) echo "unsupported arch: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BINARY" ] || { echo "binary not found: $BINARY" >&2; exit 1; }
case "$VERSION" in
    *[!0-9A-Za-z.+-]*|"") echo "invalid version: $VERSION" >&2; exit 1 ;;
esac

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="$(pwd)/dist"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# package.tgz: what ends up in /var/packages/dumbmonit-agent/target
mkdir -p "$WORK/pkg/bin" "$WORK/spk/scripts" "$WORK/spk/conf"
cp "$BINARY" "$WORK/pkg/bin/dumbmonit-agent"
chmod 0755 "$WORK/pkg/bin/dumbmonit-agent"
tar -C "$WORK/pkg" -czf "$WORK/spk/package.tgz" .

# INFO, scripts, privileges, wizard
sed -e "s/@VERSION@/$VERSION/" -e "s/@SPK_ARCH@/$SPK_ARCH/" "$HERE/INFO.in" > "$WORK/spk/INFO"
cp "$HERE"/scripts/* "$WORK/spk/scripts/"
chmod 0755 "$WORK"/spk/scripts/*
cp "$HERE/conf/privilege" "$WORK/spk/conf/privilege"
cp -R "$HERE/WIZARD_UIFILES" "$WORK/spk/WIZARD_UIFILES"

# Icons
if command -v magick >/dev/null 2>&1; then
    RESIZE="magick"
elif command -v convert >/dev/null 2>&1; then
    RESIZE="convert"
else
    RESIZE=""
fi
if [ -n "$RESIZE" ]; then
    $RESIZE "$ROOT/web/static/icon-192.png" -resize 72x72 "$WORK/spk/PACKAGE_ICON.PNG"
    $RESIZE "$ROOT/web/static/icon-512.png" -resize 256x256 "$WORK/spk/PACKAGE_ICON_256.PNG"
else
    echo "warning: no ImageMagick found, copying unresized icons" >&2
    cp "$ROOT/web/static/icon-192.png" "$WORK/spk/PACKAGE_ICON.PNG"
    cp "$ROOT/web/static/icon-512.png" "$WORK/spk/PACKAGE_ICON_256.PNG"
fi

mkdir -p "$OUT"
SPK="$OUT/dumbmonit-agent-$VERSION-$ARCH.spk"
(cd "$WORK/spk" && tar -cf "$SPK" INFO package.tgz scripts conf WIZARD_UIFILES PACKAGE_ICON.PNG PACKAGE_ICON_256.PNG)
echo "built $SPK"
