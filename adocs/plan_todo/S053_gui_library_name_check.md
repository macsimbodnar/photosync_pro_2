id:         S053
goal:       the library, name and check screens, and the refusals before a session starts
accepts:    text fields by `on_input` as DEC-068 (2), with Enter in a field same as Next; the library shows the words of `field_help`'s two sentences with no key named, the name screen shows `<library>/<folder_preview>/RAW, JPEG, ...` updating as typed; empty library: Next stays and says why; the check screen lists every picked card, the library, the folder preview with its "(the date of the earliest new photo)", "The cards are only read. Photos already in the library are skipped.", Start copying, Back, Quit; a test on real temp folders: a library that is not there is refused at Start copying with `text::refusal`'s sentence, no session made (`in_session()` false), no folder made, no Next card button, Back returns to the check and Back, Back to the library, the library is fixed, and the next Start goes on; a bad name (`a/b`, and one over 242 bytes) is refused the same, with no card read; the source check holds that non-test code names `checking_library` (DEC-050 (7)); no sync started with no card picked: dropping the only card by Look again keeps the check screen with its hint, no key named; Screen check as DEC-066 (5): `S053_library.png`, `S053_name.png`, `S053_check_two_cards.png`, `S053_refused_no_library.png`, `S053_refused_bad_name.png`, read and described in the stamp; suite and scripts/test_exfat.sh pass
touches:    crates/photosync-gui
excludes:   crates/photosync; the offer to make the folder (DEC-050 (6)); starting a real copy, which is S054
closes:     
paused_by:
author:
done:
