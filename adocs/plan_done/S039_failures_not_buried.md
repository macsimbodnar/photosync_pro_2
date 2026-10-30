id:         S039
goal:       long duplicate lists never bury a failure: all duplicate lines print after every source's failed and unreadable lines and after the library section
accepts:    failing test first: two sources, the first with many duplicates and the second with a failure, and a library read problem: every failed, unreadable and library line comes before the first duplicate line, in CLI and TUI; each duplicate line still names its source's card; DEC-043 (1) is amended by a new decision, the agent's choice under the user's standing instruction, not yet confirmed; specs and README updated if they name the place; suite and scripts/test_exfat.sh pass
touches:    crates/photosync text.rs, tests, decisions
excludes:   whether a library read failure should change the result line (if it should, a new step)
closes:
paused_by:
author:     Claude
done:       2026-10-30. Failing first: duplicates_come_after_every_failure_of_every_source_and_the_library and duplicates_stay_in_their_section_when_there_is_nothing_to_bury in text.rs, duplicates_never_bury_a_failure_or_a_library_problem_in_the_cli in exfat_e2e.rs and ..._in_the_tui in app.rs, each failed on the old order. Then green: cargo test --workspace 278 passed, 0 failed; scripts/test_exfat.sh exit 0, 263 passed, 0 failed, no image left attached. No existing test edited. DEC-046 amends DEC-043 (1): duplicates wait only when there is something to bury, a heading names the source. Also did: across syncs in the TUI's session report, as a backup card's sync would bury a later sync's failure; the CLI prints per sync, so not there. Unreadable scan entries are covered by the unit test only: chmod does nothing on exFAT. Specs and README updated.
