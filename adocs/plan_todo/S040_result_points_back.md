id:         S040
goal:       when duplicate lines sit between a problem and the result, the end of a CLI report still points at the problem
accepts:    failing tests first: a report with a problem line (failed, unreadable, library section) and held duplicates ends, near the result, with a line that names how many problem lines sit above the duplicates, or the problems repeat there: pick the smaller; `| tail` shows it; a report with no problem has no such line; a library-only problem (result says "all on the drive", exit 0) is visible in the last lines; a unit test on session_report covers `hold` for RunEnd::Refused and RunEnd::Crashed with duplicates-only syncs, and flipping that arm fails it under plain `cargo test --workspace`; README's duplicates sentence says that in CLI one-by-one each report prints as its sync ends; suite and scripts/test_exfat.sh pass
touches:    crates/photosync text.rs, tests, README, specs, decisions
excludes:   changing exit codes; changing the safety wording of the result line
closes:
paused_by:
author:
done:
