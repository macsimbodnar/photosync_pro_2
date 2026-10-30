id:         S046
goal:       closed_stdout_never_panics_and_keeps_the_exit_code never fails by a race
accepts:    the cause shown by hand or by a failing test first; the test still proves a closed stdout loses only the text and keeps the exit code; existing assertions unchanged; suite and scripts/test_exfat.sh pass
touches:    crates/photosync/tests/exfat_e2e.rs (closed_run); main.rs only if the app proves at fault
excludes:   the images' watchdog (S045); the time limits
closes:
paused_by:
author:
done:

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
