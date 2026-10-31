id:         S046
goal:       closed_stdout_never_panics_and_keeps_the_exit_code never fails by a race
accepts:    the cause shown by hand or by a failing test first; the test still proves a closed stdout loses only the text and keeps the exit code; existing assertions unchanged; suite and scripts/test_exfat.sh pass
touches:    crates/photosync/tests/exfat_e2e.rs (closed_run); main.rs only if the app proves at fault
excludes:   the images' watchdog (S045); the time limits
closes:
paused_by:
author:     Claude (subagent)
done:       2026-10-31, the test's fault, the binary was right (DEC-060). Reproduced with the real binary: 1064 of 307109 runs had a read end alive before the spawn, 448 of them gave code 0 and no sentence; 0 of 306045 with the read end gone missed it. closed_pipe proves no reader by a failed write; 236390 runs all noticed. Assertions unchanged. 12 of 12 `cargo test --workspace` (335 passed each last run), scripts/test_exfat.sh passes, hdiutil info shows no images. The gate's own failure did not come back in 80 gates before the fix: its rate stays unexplained. README checked: no change. Fixed nothing in main.rs.

Found in S045. Failed in 1 of 18 `cargo test --workspace` runs (2026-10-31, run 8 of 14), inside
`ignored_image_tests_pass_on_real_mounts` of exfat_e2e. Not seen before in the 10 runs on the old
harness, too few to say it is new. S045's change does not run on this path: the watchdog's
`kill_tree` runs only after the binary dies or a command hits its limit.

Seen: code 0, the first assert passed, and at exfat_e2e.rs:348 stderr had no "Cannot write the
report to stdout": it stopped after `copying 1/1 files`. So the report's write returned Ok, on a
pipe whose reader the test had dropped.

Likely cause, not proven, and std's source was not read (rust-src is not installed): on macOS
there is no `pipe2`, so from memory std makes the pipe and sets close-on-exec in two calls. Another
test thread that spawns between them hands the read end to its child, whose life keeps the pipe
readable, and the write lands in the buffer. Check that first. If so, the test's premise needs a
pipe no other process can hold, not a retry.
