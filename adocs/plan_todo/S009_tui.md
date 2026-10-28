id:         S009
goal:       TUI (ratatui): pick sources, drive and library, an optional name; show progress with ETA; show the report
accepts:    `photosync` with no arguments opens the TUI; the TUI calls only the core's public API (no sync logic in the bin crate); state/update logic unit-tested without a terminal; a full sync from a real card checked by hand, recorded in the stamp; README updated; CLI fixes from the S008 fast check: the in-place progress line is cut to the terminal width, and a closed stdout (e.g. `| head`) never panics: the sync still finishes, and the exit code stays within the documented 0/1/2
touches:    crates/photosync, README.md
excludes:   GUI; multi-source concurrency (S010)
closes:
paused_by:
author:
done:
