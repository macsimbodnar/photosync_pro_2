id:         S031
goal:       a name that cannot be a folder name is refused before any card is read, including one too long
accepts:    failing test first: '<date> - <name>' over 255 bytes is refused, exit 2 in the CLI, fixable in the TUI before the session starts; the characters refused per platform are decided and recorded (':' on macOS and FAT-family characters on exFAT, checked on a real image); the 'Folder ...' report line never names a folder that does not exist in this path; suite and script pass
touches:    photosync-core copy.rs check_name, crates/photosync app.rs, text.rs, specs, decisions
excludes:   renaming or sanitising a name silently
closes:     2026-10-30_adversarial-F06
paused_by:
author:     Claude (subagent)
done:       2026-10-30. Failing tests first, seen red. `cargo test --workspace`: 309 passed, 0 failed. `scripts/test_exfat.sh`: exit 0, 310 passed, 8 of them on HFS+; no image left attached. Tests: copy_engine name limit, characters, kept names, `there`; `a_name_the_filesystem_finds_too_long_is_refused` fails on HFS+ with the lookup removed (mutant run, exit 1); cli exit 2 with no card read; app fix in the TUI; exfat_e2e Folder line. Characters measured on APFS, exFAT, HFS+, FAT32 images, recorded in DEC-053; Linux exFAT not measured, left to S025. Length refused in `Session::new`, not `sync_folder`: an existing test holds that behaviour, unchanged but for its comment. Audit Status lines left alone.
