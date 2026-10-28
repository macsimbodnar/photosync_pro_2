id:         S025
goal:       first run on Linux: build, tests, and hand checks with a real external drive, SD cards and the X-T5 over USB
accepts:    on the user's Linux machine (distro, kernel and Rust version recorded in the stamp): `cargo build` and `cargo test --workspace` pass, or each failure becomes its own step; `photosync sources` lists an X-T5 SD card mounted under /media/$USER or /run/media/$USER, and `photosync drives` lists the external drive and not the card; a sync from the card to `<drive>/Photos` (exFAT and, if available, ext4) exits 0 with files under `<date> - <name>/<TYPE>/`, and a second run copies 0; the scan line's device differs from the drive's (`stat -c %d`); the X-T5 over a USB-C data cable in USB CARD READER mode is listed and syncs the same way; the no-replace rename works on the drive (renameat2 RENAME_NOREPLACE, or the EINVAL fallback on exFAT, DEC-018/DEC-019) with no "unsupported" failures; two card readers at once copy in parallel (S019); `sudo photosync drives` refuses with exit 2; the TUI (`photosync`) completes a sync; outputs recorded in the stamp
touches:    nothing in code unless a check fails (then a new step per failure)
excludes:   Windows; packaging
closes:
paused_by:
author:
done:
