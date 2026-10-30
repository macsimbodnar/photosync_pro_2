id:         S034
goal:       stale-temp cleanup leaves a temp whose writing process is alive
accepts:    failing test first: a temp named with a live pid survives cleanup and one with a dead pid goes; two processes on one library, reproduced as in the finding, both finish with no failure on an exFAT image; the temp's own pid counts as live only for an earlier run's file if the design needs it, recorded; a pid reused by another program only leaves a temp for later, which is safe, say so in specs; INV-2 wording stays true; suite and script pass
touches:    photosync-core copy.rs, specs, decisions
excludes:   library-wide locks across processes unless the test shows they are needed
closes:     2026-10-30_adversarial-F09
paused_by:
author:     Claude (Opus 5.5)
done:       2026-10-30. Stale means the temp's pid is gone or is this process; a name with no pid stays (DEC-054). Failing first: cleanup_keeps_the_temp_of_a_live_process_and_removes_that_of_a_dead_one and cleanup_leaves_a_temp_whose_name_holds_no_pid; a_second_process_leaves_the_live_temp_of_the_first on exFAT images printed F09's false "Removed 1 temp file" before the fix and again with the check turned off. That test stops the first process with a temp on the drive, so the second always meets it; both then exit 0 with every photo in place. Passed 5 runs of 5. a_temp_of_a_killed_process_goes_with_the_next_sync proves a real SIGKILL through the binary. cargo test --workspace: 312 passed, 26 ignored (run by the gate tests on images). scripts/test_exfat.sh: 315 passed. No image left attached. No new dependency. Fixtures naming pids 1, 5, 12, 77 now name the test's own pid; assertions unchanged. Specs and README updated.
