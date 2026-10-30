id:         S045
goal:       a_hung_hdiutil_ends_the_watchdog_and_it_says_so never fails by a race, and a killed hdiutil never counts as ended well
accepts:    failing test first, or the race shown by hand: an hdiutil the watchdog kills at its limit always counts as failed, so -force is tried; the test passes under load; existing tests unchanged; suite and scripts/test_exfat.sh pass
touches:    tests/common/images.rs (the watchdog's kill_tree or limited), test_images.rs if the fake is at fault
excludes:   the app; the time limits themselves
closes:
paused_by:
author:
done:

Found in S035. Failed in 2 of 6 `cargo test --workspace` runs, with no harness file changed,
and 0 of 25 runs alone, so load seems to matter. Calls seen: `detach <mnt> -quiet`, `info`,
`info`, and no `-force`: the first detach counted as a success.
Likely cause, read and not proven: `kill_tree` kills children first. The fake hdiutil's bare
`wait` then returns 0 when its `sleep` dies, so it can exit 0 before its own kill -9 comes, and
`limited` sees status 0. A real hdiutil with a helper could do the same: `-force` is skipped,
but the folder still goes only if `hdiutil info` and `mount` show nothing left.
