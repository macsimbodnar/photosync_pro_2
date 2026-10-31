#!/usr/bin/env bash
# Makes `target/package/phoosync pro 2.app` and a zip of it from a release build (DEC-070).
# Only tools this Mac has, no network, nothing written outside target/. The signature is
# ad hoc: it runs on this Mac, and the README says what another Mac does with it.
# scripts/test_package_mac.sh checks the result.
set -euo pipefail

if [[ "$(uname)" != Darwin ]]; then
    echo "package_mac.sh: macOS only" >&2
    exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package="$repo/target/package"
name="phoosync pro 2"
app="$package/$name.app"
zip="$package/$name.zip"

# A CARGO_TARGET_DIR from outside would put the build outside target/.
export CARGO_TARGET_DIR="$repo/target"
cargo build --manifest-path "$repo/Cargo.toml" --release --locked --offline -p photosync-gui

# `cargo pkgid` ends in `#photosync-gui@<version>` or `#<version>`, by the path's name.
pkgid="$(cargo pkgid --manifest-path "$repo/Cargo.toml" --offline -p photosync-gui)"
version="${pkgid##*[#@]}"

# All of it, so no picture from an older build can pass for this one's.
rm -rf "$package"
mkdir -p "$app/Contents/MacOS"
cp "$repo/target/release/photosync-gui" "$app/Contents/MacOS/photosync-gui"

cat >"$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>$name</string>
    <key>CFBundleExecutable</key>
    <string>photosync-gui</string>
    <key>CFBundleIdentifier</key>
    <string>local.photosync.gui</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$name</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSRemovableVolumesUsageDescription</key>
    <string>phoosync pro 2 reads photos from your camera card and copies them to the drive you pick.</string>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist"

# The linker signed the binary alone; this one also seals the Info.plist to it.
codesign --force --sign - "$app"

ditto -c -k --keepParent "$app" "$zip"

echo "made $app ($(du -sh "$app" | cut -f1 | tr -d ' ')) and $zip ($(du -sh "$zip" | cut -f1 | tr -d ' '))"
