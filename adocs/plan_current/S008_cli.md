id:         S008
goal:       CLI commands: `photosync sources`, `photosync drives`, `photosync sync --source ... --dest ... [--name ...]` with plain progress and report
accepts:    integration test runs the binary on temp dirs and checks the exit code, the files on the destination, and the report text; `--help` documents every flag; README updated; a repeated `--source` (same root, any spelling) is refused with a clear error; hand checks on Mac, recorded in the stamp: a real X-T5 SD card in a card reader is listed by `photosync sources` and a sync from it succeeds; the X-T5 over USB cable in "USB card reader" mode is listed and a sync from it succeeds (moved from S007; closes audit F09); the CLI refuses to run as root (euid 0), since root can write to system volumes such as Recovery (S007 fast check)
touches:    crates/photosync, README.md
excludes:   TUI
closes:     2026-10-28_adversarial-F09
paused_by:
author:     Claude agent (2026-10-28)
done:
