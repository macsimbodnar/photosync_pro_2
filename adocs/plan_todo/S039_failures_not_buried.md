id:         S039
goal:       long duplicate lists never bury a failure: all duplicate lines print after every source's failed and unreadable lines and after the library section
accepts:    failing test first: two sources, the first with many duplicates and the second with a failure, and a library read problem: every failed, unreadable and library line comes before the first duplicate line, in CLI and TUI; each duplicate line still names its source's card; DEC-043 (1) is amended by a new decision, the agent's choice under the user's standing instruction, not yet confirmed; specs and README updated if they name the place; suite and scripts/test_exfat.sh pass
touches:    crates/photosync text.rs, tests, decisions
excludes:   whether a library read failure should change the result line (if it should, a new step)
closes:
paused_by:
author:
done:
