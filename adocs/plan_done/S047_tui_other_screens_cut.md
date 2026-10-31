id:         S047
goal:       the TUI's other screens never cut what the user must read
accepts:    failing test first, on a drawn screen: a refusal's sentence, with the session's result after it, can be read to its end on a small terminal; a long note on the card and drive lists, and a long scan line on the copying screen, show whole at 80 columns; suite passes
touches:    crates/photosync tui/view.rs, tests
excludes:   the report (S037); the wording; the Library and Name fields' typing line
closes:
paused_by:
author:     Claude
done:       2026-10-31, Claude. cargo test --workspace 360 passed, 0 failed; scripts/test_exfat.sh exit 0, no image left attached. 12 tests seen failing first, on drawn screens: in tui/view.rs a_long_note_shows_whole_on_the_card_and_drive_lists_at_80_columns, a_long_path_in_a_list_shows_whole_at_80_columns, a_long_scan_line_shows_whole_on_the_copying_screen_at_80_columns, the_progress_lines_of_long_card_names_show_whole_at_80_columns, a_long_hint_shows_whole_at_80_columns, the_keys_show_whole_on_a_narrow_terminal; in tui/tests.rs a_refusal_and_the_result_after_it_are_read_to_their_end, the_refused_screen_offers_scrolling_only_when_it_is_needed, notes_taller_than_the_screen_are_read_to_their_end, a_check_screen_taller_than_the_terminal_is_read_to_its_end, the_help_of_a_field_is_read_to_its_end, scan_lines_taller_than_the_screen_are_read_to_their_end. Added with the fix: a_scrolled_check_screen_starts_at_its_top_when_come_back_to. No existing test changed. Touches app.rs too, past the step's touches: the scroll keys are the model's. Beyond the accepts, in the goal: list paths, progress lines, hint and keys wrap; the check and a field's help scroll. Wording unchanged, but for the keys row's scroll offer. The copying screen's gauge moved on top. Specs TUI and README updated. DEC-062, not yet confirmed by the user, with what can still be cut on a tiny terminal.

Found in S037, by reading `view.rs`, not run. `outcome`'s refused box wraps but cannot scroll, so a sentence
taller than the screen loses its end; the exit message still prints it after quitting. `list_screen`'s notes
and `running`'s scanned lines are one row each and do not wrap. `view::wrapped` is there to use.
