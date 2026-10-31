id:         S064
goal:       the harness can tell what stays fixed from what scrolls, and Fails cannot pass on words that are gone
accepts:    self-tests first, each failing on today's harness: a new check, say `fixed_in_sight`, fails for a sentence inside any scrollable and for one out of sight, and passes for one in the fixed part; `in_sight`'s doc stops calling it the check for what may never scroll; S053 to S056's accepts name the new check for "Do not format", Cancelling, the second-close warning, the banner and the buttons, and any such sentence already drawn in S052's screens uses it; `Step::Fails` also runs `expect` on the words, so a click on words that have vanished fails the script; gui_split.rs rejects `column![.., "x"]`, `row![.., "x"]` and every other macro item that is a literal, and `center_x`, `center_y`, `right`, `bottom`, `pin`, `opaque`, `stack!`, `grid!` with a literal, each checked by a scratch line in the test, with DEC-072 (4)'s "what it cannot see" corrected; README's GUI tests section stays true; suite and scripts/test_exfat.sh pass
touches:    crates/photosync-gui src/tests/harness.rs, tests/mod.rs, tests/gui_split.rs, README, plan_todo S053 to S056, decisions
excludes:   new screens
closes:
paused_by:
author:
done:
