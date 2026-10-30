id:         S042
goal:       the TUI's refused screen offers only keys that work
accepts:    failing test first: a refusal before the session starts (fresh drive, bad name) shows no "n next card" in the footer, and the footer's keys all act; inside a session the footer still offers it; suite passes
touches:    crates/photosync tui/view.rs, tests
excludes:   the rule of what `n` does
closes:
paused_by:
author:
done:
