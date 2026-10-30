id:         S037
goal:       the TUI's report shows every line whole, so a failed file's reason is never cut off
accepts:    failing test first: at 80 columns a long "failed" line shows its reason on the report screen, and a duplicate line with a long Linux card path such as /run/media/max/NO NAME shows the card whole; scrolling still reaches the end; suite passes
touches:    crates/photosync tui/view.rs, app.rs (scroll), tests
excludes:   the wording of the report's lines
closes:
paused_by:
author:
done:
