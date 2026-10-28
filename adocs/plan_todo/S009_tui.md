id:         S009
goal:       TUI (ratatui): pick sources, drive and library, an optional name; show progress with ETA; show the report
accepts:    `photosync` with no arguments opens the TUI; the TUI calls only the core's public API (no sync logic in the bin crate); state/update logic unit-tested without a terminal; a full sync from a real card checked by hand, recorded in the stamp; README updated
touches:    crates/photosync, README.md
excludes:   GUI; multi-source concurrency (S010)
closes:
paused_by:
author:
done:
