#!/bin/bash
# Builds shop.lauden.dev - Launcher.app from the release binary.
# Run from the launcher/ repo root: bash packaging/macos/build-app.sh
set -euo pipefail

APP_NAME="shop.lauden.dev - Launcher"
BUNDLE="${APP_NAME}.app"

echo "Building release binary..."
cargo build --release

echo "Generating iconset..."
rm -rf AppIcon.iconset
mkdir -p AppIcon.iconset
sips -z 16 16     assets/icon_256.png --out AppIcon.iconset/icon_16x16.png
sips -z 32 32     assets/icon_256.png --out AppIcon.iconset/icon_16x16@2x.png
sips -z 32 32     assets/icon_256.png --out AppIcon.iconset/icon_32x32.png
sips -z 64 64     assets/icon_256.png --out AppIcon.iconset/icon_32x32@2x.png
sips -z 128 128   assets/icon_256.png --out AppIcon.iconset/icon_128x128.png
sips -z 256 256   assets/icon_256.png --out AppIcon.iconset/icon_128x128@2x.png
sips -z 256 256   assets/icon_256.png --out AppIcon.iconset/icon_256x256.png
sips -z 512 512   assets/icon_256.png --out AppIcon.iconset/icon_256x256@2x.png
sips -z 512 512   assets/icon_256.png --out AppIcon.iconset/icon_512x512.png
iconutil -c icns AppIcon.iconset -o AppIcon.icns

echo "Building app bundle..."
rm -rf "${BUNDLE}"
mkdir -p "${BUNDLE}/Contents/MacOS"
mkdir -p "${BUNDLE}/Contents/Resources"
cp target/release/lauden-dev-launcher "${BUNDLE}/Contents/MacOS/"
cp AppIcon.icns "${BUNDLE}/Contents/Resources/"

cat > "${BUNDLE}/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>shop.lauden.dev - Launcher</string>
	<key>CFBundleDisplayName</key>
	<string>shop.lauden.dev - Launcher</string>
	<key>CFBundleIdentifier</key>
	<string>dev.lauden.launcher</string>
	<key>CFBundleVersion</key>
	<string>0.1.0</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleExecutable</key>
	<string>lauden-dev-launcher</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon.icns</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
EOF

echo "Done: ${BUNDLE}"
