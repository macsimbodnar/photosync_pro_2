id:         S008
goal:       CLI commands: `photosync sources`, `photosync drives`, `photosync sync --source ... --dest ... [--name ...]` with plain progress and report
accepts:    integration test runs the binary on temp dirs and checks the exit code, the files on the destination, and the report text; `--help` documents every flag; README updated; a repeated `--source` (same root, any spelling) is refused with a clear error; hand checks moved to S017 (DEC-029); the CLI refuses to run as root (euid 0), since root can write to system volumes such as Recovery (S007 fast check)
touches:    crates/photosync, README.md
excludes:   TUI
closes:
paused_by:
author:     Claude agent (2026-10-28)
done:       2026-10-28: `cargo test --workspace` 153 passed; clippy clean; `scripts/test_exfat.sh` exit 0, including tests/exfat_e2e.rs: a real mounted exFAT card image synced to a drive image (files, card unchanged, device = card st_dev, report text, re-sync 0 copied, named-folder reuse, collision exit 1, library on card exit 2). S008 fast check: nothing dangerous; two cosmetic notes moved to S009. Hardware hand checks not run (the user was unavailable): moved to S017, DEC-029.
