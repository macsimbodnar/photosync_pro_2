id:         S058
goal:       package the GUI as a Mac .app, provable on this Mac (DEC-070). Signing for other Macs needs the user and is left out
accepts:    `scripts/package_mac.sh` makes `target/package/phoosync pro 2.app` from a release build: `Contents/MacOS/photosync-gui`, `Contents/Info.plist` with `CFBundleExecutable`, `CFBundleIdentifier` `local.photosync.gui`, `CFBundleName` and `CFBundleDisplayName` "phoosync pro 2", `CFBundlePackageType` `APPL`, the version from the workspace, `NSHighResolutionCapable`; `codesign --force --sign -`; then a zip by `ditto -c -k --keepParent`; no new crate, no network, nothing outside `target/`; `scripts/test_package_mac.sh` checks `plutil -lint`, `codesign --verify --strict`, `otool -L` lists only `/System` and `/usr/lib`, the zip unzips to the same files, and runs `open -n --stderr <file> <app> --args --smoke-png <path>`, waits with a time limit, and compares that PNG with the headless picture through the ignored test of S051, 0 pixels off, or says the scale is not 2; root and no terminal are not asked of it: launched from `open` it has no tty and draws; a window shows for about a second, so neither script is in the gate, and it is said in the README; Screen check as DEC-066 (5) for the packaged window: `target/package/smoke.png` read and described in the stamp, as what the real `.app` drew; the script's output is in the stamp; cargo test --workspace passes. NEEDS THE USER, not done here: a Developer ID and notarization for other Macs, as this Mac has 0 signing identities (`security find-identity -p codesigning -v`); an icon, if wanted
touches:    scripts, README
excludes:   Developer ID signing, notarization, a DMG, an icon; any product code but what the script needs
closes:     
paused_by:
author:
done:
