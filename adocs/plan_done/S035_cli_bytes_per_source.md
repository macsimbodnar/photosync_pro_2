id:         S035
goal:       CLI progress shows bytes per source, as specs Progress says
accepts:    failing test first: the line gives files and bytes per source within the line-width cut, the cut dropping per-source bytes before anything else; suite passes
touches:    crates/photosync text.rs, tests
excludes:   TUI changes
closes:     2026-10-30_adversarial-F10
paused_by:
author:     Claude
done:       2026-10-30 Claude. Five tests seen failing first, then fixed. With several sources the CLI's line gives each one's files and bytes; past the width all of those bytes go at once, before ", cancelled" and all else (boundary held column by column); a log and an unknown width keep them; one source is as before; progress_line and so the TUI unchanged (DEC-058). Two existing tests' expected text extended for the format, each holding more than before: progress_of_two_cards_at_once_shows_each_card, two_cards_on_two_devices_copy_at_once. One test's comment reworded (cli.rs). Trivial fix, in scope: the probe's comment and DEC-056 (2) said ASCII of one length fits every drive, and so the path is at fault; false on a shorter NAME_MAX (eCryptfs, 143 bytes, general knowledge, not run). Comment, DEC-056, PathTooLong's doc and Display, specs and README corrected; the refusal now names both causes and sends only a long path to move the library (new test, seen failing). cargo test --workspace 334 passed, 28 ignored and run by their gates; scripts/test_exfat.sh 337 passed, no image left. Not green every time: a_hung_hdiutil_ends_the_watchdog_and_it_says_so failed in 2 of 6 workspace runs, with no harness file changed, and passed 25 of 25 alone: S045, independent. Linux not run.
