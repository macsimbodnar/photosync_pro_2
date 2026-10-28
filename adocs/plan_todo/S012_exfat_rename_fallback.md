id:         S012
goal:       copies work on exFAT: when the no-replace rename is unsupported, fall back to a locked check + rename (DEC-018)
accepts:    an in-app lock per library serialises the fallback's check + rename, so photosync's own threads never race on it; the fallback is used only on ENOTSUP (macOS) / EINVAL (Linux renameat2), never for any other error; a unit test forces the "unsupported" path and proves that an existing target is never replaced and that two racing threads lose no file; a script `scripts/test_exfat.sh` (macOS, hdiutil) creates and mounts a temporary exFAT image, runs `cargo test -p photosync-core` with TMPDIR on it, and detaches it again, and it passes, with only tests that need Unix permission bits skipped or tolerated and named in the script; the folder fsync on exFAT is checked by that run; README documents the script; INV-2, INV-3 and INV-7 tests still hold
touches:    photosync-core copy engine, scripts/, README.md
excludes:   Linux exFAT run (later Linux verification); progress/cancel
closes:
paused_by:
author:
done:
