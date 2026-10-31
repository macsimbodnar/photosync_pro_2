id:         S047
goal:       the TUI's other screens never cut what the user must read
accepts:    failing test first, on a drawn screen: a refusal's sentence, with the session's result after it, can be read to its end on a small terminal; a long note on the card and drive lists, and a long scan line on the copying screen, show whole at 80 columns; suite passes
touches:    crates/photosync tui/view.rs, tests
excludes:   the report (S037); the wording; the Library and Name fields' typing line
closes:
paused_by:
author:
done:

Found in S037, by reading `view.rs`, not run. `outcome`'s refused box wraps but cannot scroll, so a sentence
taller than the screen loses its end; the exit message still prints it after quitting. `list_screen`'s notes
and `running`'s scanned lines are one row each and do not wrap. `view::wrapped` is there to use.
