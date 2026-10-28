id:         S010
goal:       several sources at once, and one source after another in one session with a combined report
accepts:    tests: two sources copy concurrently with per-source progress; a session accepts a new source after the first finishes; the combined report satisfies INV-6 per source; a file on both sources is copied once; CLI and TUI both expose it; README updated; when a read fails on one source, the duplicate copy on another source is tried; extra copies reported as "duplicate in session"; the INV-1 manifest test holds for every source in a session
touches:    photosync-core, crates/photosync, README.md
excludes:   GUI
closes:     2026-10-28_adversarial-F07
paused_by:
author:
done:
