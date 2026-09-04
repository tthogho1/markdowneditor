#!/bin/bash
set -e

APP="MarkdownEditor.app"
DEST="$HOME/Desktop/$APP"
BINARY="target/release/markdown_editor_rust"

echo "Building release binary..."
cargo build --release

echo "Creating $DEST ..."
rm -rf "$DEST"
mkdir -p "$DEST/Contents/MacOS"
mkdir -p "$DEST/Contents/Resources"

cp "$BINARY" "$DEST/Contents/MacOS/"

cat > "$DEST/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>markdown_editor_rust</string>
  <key>CFBundleIdentifier</key>
  <string>com.tthogho1.markdowneditor</string>
  <key>CFBundleName</key>
  <string>Markdown Editor</string>
  <key>CFBundleDisplayName</key>
  <string>Markdown Editor</string>
  <key>CFBundleVersion</key>
  <string>1.0</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>
</dict>
</plist>
EOF

echo "Done: $DEST"
echo "Open with: open \"$DEST\""
