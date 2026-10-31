id:         S048
goal:       a large report stays quick to scroll: wrapping does not repeat on every draw
accepts:    failing test first: a report of 5000 long lines (two paths each, with some non-ASCII), and a count of wraps per draw or a generous draw-time bound that does not flake under load, shows the whole report is not wrapped again while width, height and text are unchanged; the wrapped rows are kept until one of those changes, and a resize or a new outcome rewraps, tested; grapheme width is measured without building a Line per grapheme; a key press is handled within one draw on that report; scroll positions still reach the end; the other TUI screens that use `wrapped` (S047) cost no more than before; suite and scripts/test_exfat.sh pass
touches:    crates/photosync tui/view.rs, tui/mod.rs, app.rs if the cache sits there, tests
excludes:   changing what is drawn
closes:
paused_by:
author:
done:
