#!/bin/bash

# Bundle script for RustInSynth VST3 plugin

PLUGIN_NAME="RustInSynth"
DYLIB_PATH="target/release/lib${PLUGIN_NAME}.dylib"
VST3_DIR="$HOME/Library/Audio/Plug-Ins/VST3/${PLUGIN_NAME}.vst3"
CONTENTS_DIR="${VST3_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"

echo "Bundling ${PLUGIN_NAME} VST3 plugin..."

# Check if dylib exists
if [ ! -f "$DYLIB_PATH" ]; then
    echo "Error: $DYLIB_PATH not found. Please run 'cargo build --release' first."
    exit 1
fi

# Create VST3 bundle structure
echo "Creating bundle structure..."
mkdir -p "$MACOS_DIR"

# Copy the dylib
echo "Copying plugin binary..."
cp "$DYLIB_PATH" "${MACOS_DIR}/${PLUGIN_NAME}"

# Create Info.plist
echo "Creating Info.plist..."
cat > "${CONTENTS_DIR}/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>${PLUGIN_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.nasker.rustinsynth</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${PLUGIN_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleVersion</key>
    <string>1.0.1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.1</string>
</dict>
</plist>
EOF

# Create PkgInfo
echo "BNDL????" > "${CONTENTS_DIR}/PkgInfo"

echo "✅ VST3 bundle created successfully at: $VST3_DIR"
echo ""
echo "You can now load the plugin in Ableton Live."
