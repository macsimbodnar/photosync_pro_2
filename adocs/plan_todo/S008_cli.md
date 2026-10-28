id:         S008
goal:       CLI commands: `photosync sources`, `photosync drives`, `photosync sync --source ... --dest ... [--name ...]` with plain progress and report
accepts:    integration test runs the binary on temp dirs and checks the exit code, the files on the destination, and the report text; `--help` documents every flag; README updated; a repeated `--source` (same root, any spelling) is refused with a clear error
touches:    crates/photosync, README.md
excludes:   TUI
closes:
paused_by:
author:
done:
