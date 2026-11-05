#!/usr/bin/env bash
# Builds dist/FigTime-<version>-mac.dmg containing a universal (Apple chip + Intel) FigTime.app.
# Run on a Mac after:
#   cargo build --release --target aarch64-apple-darwin
#   cargo build --release --target x86_64-apple-darwin
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP=dist/FigTime.app
rm -rf dist && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# One binary for both Apple chip and Intel Macs
lipo -create \
  target/aarch64-apple-darwin/release/figtime \
  target/x86_64-apple-darwin/release/figtime \
  -output "$APP/Contents/MacOS/figtime"

# App icon
ICONSET=$(mktemp -d)/AppIcon.iconset
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  if [ $double -le 512 ]; then
    sips -z $double $double assets/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
  fi
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>FigTime</string>
  <key>CFBundleDisplayName</key><string>FigTime</string>
  <key>CFBundleIdentifier</key><string>com.infinitylinkage.figtime</string>
  <key>CFBundleExecutable</key><string>figtime</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# Ad-hoc signature: required for Apple chip Macs to launch it (not an Apple-issued certificate)
codesign --force --deep --sign - "$APP"

# .dmg with an Applications shortcut to drag into
STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname FigTime -srcfolder "$STAGE" -ov -format UDZO "dist/FigTime-${VERSION}-mac.dmg"
rm -rf "$APP"
ls -la dist
