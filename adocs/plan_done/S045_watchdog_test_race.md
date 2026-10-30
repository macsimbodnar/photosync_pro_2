id:         S045
goal:       a_hung_hdiutil_ends_the_watchdog_and_it_says_so never fails by a race, and a killed hdiutil never counts as ended well
accepts:    failing test first, or the race shown by hand: an hdiutil the watchdog kills at its limit always counts as failed, so -force is tried; the test passes under load; existing tests unchanged; suite and scripts/test_exfat.sh pass
touches:    tests/common/images.rs (the watchdog's kill_tree or limited), test_images.rs if the fake is at fault
excludes:   the app; the time limits themselves
closes:
paused_by:
author:     Claude (subagent, session_01RTrRzTxLkBQpdFLoEc5hDW)
done:       2026-10-31 Claude. The race shown by a trace in the real gate: the watcher killed the fake's helper, the fake's `wait` returned 0 and it exited before its own kill -9, and `limited` saw status 0, so -force was skipped. CPU load alone did not show it: 0 failures in 60 parallel runs of the test and 8 runs of its binary with 20 busy loops; the gate did, 1 of 4 and then 1 of 6 with the trace in. Fix: `kill_tree` stops a process before its children and kills it last (DEC-059). Killing the parent first, tried, failed both tests all 3 runs with `lives on`. New test an_hdiutil_killed_at_its_limit_never_counts_as_ended_well seen failing 5 of 5 before the fix, with the same calls as the flake; existing tests unchanged. Trivial and in scope, outside `touches`: scripts/test_exfat.sh had the same order, fixed the same way (by hand: status 0 before, 137 after), and skips the new test. cargo test --workspace 18 runs: 17 passed, 335 passed and 28 ignored and run by their gates; 4 of those under 20 busy loops. a_hung_hdiutil_ends_the_watchdog_and_it_says_so ran in 17 and failed in none. Not green every time: run 8 of 14 failed in closed_stdout_never_panics_and_keeps_the_exit_code, before test_images ran: S046, independent. scripts/test_exfat.sh exit 0, 337 passed. `hdiutil info` shows no image, no load loop left. README checked, still true. Linux not run.

Found in S035. Failed in 2 of 6 `cargo test --workspace` runs, with no harness file changed,
and 0 of 25 runs alone, so load seems to matter. Calls seen: `detach <mnt> -quiet`, `info`,
`info`, and no `-force`: the first detach counted as a success.
Likely cause, read and not proven: `kill_tree` kills children first. The fake hdiutil's bare
`wait` then returns 0 when its `sleep` dies, so it can exit 0 before its own kill -9 comes, and
`limited` sees status 0. A real hdiutil with a helper could do the same: `-force` is skipped,
but the folder still goes only if `hdiutil info` and `mount` show nothing left.
