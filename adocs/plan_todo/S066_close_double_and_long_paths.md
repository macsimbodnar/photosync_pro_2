id:         S066
goal:       a double click on close cannot pass a warning; a long path cannot break the check screens' banner; Cmd-Q's exit code is stated
accepts:    failing tests first: two `M::CloseRequested` with no time between them, on the cards with code not 0: the second is ignored and "Before you quit" stays; while copying: the first cancels, the second within the guard does not kill the copy; on a report that a fast sync just raised: the second close does not quit; a close after the guard (about 500 ms, the clock injected in tests, no sleep) acts as before; find out why `the_banner_holds_on_the_library_name_and_check_screens` fails on a longer temp path (S057 moved the script's temp folder to /tmp to avoid it; DEC-084): if a long real path pushes a banner or a button out of sight, that is a screen defect and the screen is fixed so the test holds at any length, with a test using a 300-byte path; the test's assertion stays as strong; then the script's temp folder may stay in /tmp only for a recorded reason; docs: DEC-069 (6) says Cmd-Q and the Dock's Quit may return 0 through AppKit's terminate, from memory and unmeasured, and S060 measures `Contents/MacOS/photosync-gui; echo $?` after Cmd-Q in a copy, and tries a real double click on close; README and specs stay true; suite and scripts/test_exfat.sh pass
touches:    crates/photosync-gui src/gui.rs, tests; crates/photosync src/app.rs if the cancelling close lives there, with the TUI's Ctrl-C rule unchanged; S060 file; decisions; specs
excludes:   objc2; catching Cmd-Q
closes:
paused_by:
author:
done:
