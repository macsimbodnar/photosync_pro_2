id:         S033
goal:       after a name collision, the result names the way out: sync again under another name
accepts:    failing test first: a result with a collision among the failures says to sync again under another name (DEC-013), and a result without one keeps today's sentence; the TUI's wording fits its own rule that the name stays for the session; suite passes
touches:    crates/photosync text.rs, tests
excludes:   changing collision handling
closes:     2026-10-30_adversarial-F08
paused_by:
author:
done:
