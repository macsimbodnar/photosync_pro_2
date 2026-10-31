id:         S049
goal:       notes never squeeze the list's own sentence off the screen, and the Cancelling sentence is always in sight
accepts:    failing tests first, on drawn screens: no card found with scan-failure notes filling the body (80x12, 80x24, 40 columns): the whole empty-list sentence shows; a selected list row that wraps to 2 rows shows whole; list_areas reserves those rows before giving the rest to notes; on the copying screen scrolled down, pressing c or Ctrl-C brings the "Cancelling" sentence into sight (scroll reset in App::cancel); the test scan_lines_taller_than_the_screen_are_read_to_their_end either asserts what its comment says about the progress line or the comment is corrected; specs' "No screen cuts what it says" names every exception DEC-062 (6) allows, with the empty-list exception removed if this step closes it; suite and scripts/test_exfat.sh pass
touches:    crates/photosync tui/view.rs, app.rs, tui/tests.rs, specs, decisions
excludes:   new screens; the wording
closes:
paused_by:
author:
done:
