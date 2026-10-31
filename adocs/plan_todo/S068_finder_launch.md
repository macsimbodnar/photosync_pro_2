id:         S068
goal:       a Finder launch always opens the window, and what a Finder launch cannot see is said
accepts:    failing test first: args.rs ignores an argument that starts `-psn_` and still refuses other unknown arguments; test_package_mac.sh launches the packaged binary once with no arguments of its own beyond its test seam's environment, or the stamp says why it cannot; README's Mac app section says: launched from Finder, stdout and stderr go nowhere, so the report is what the screen shows, and the sentence "The window closed with no report... Do not format the card." and the exit code are not seen there; specs say the same in a clause; S060 adds: a double click from Finder, on a copy with the quarantine flag if the user downloads one, opens the window; suite passes
touches:    crates/photosync-gui src/args.rs, tests; scripts/test_package_mac.sh; README; specs; S060 file
excludes:   a log file; a native dialog after the window ends (a new step if the user wants one)
closes:
paused_by:
author:
done:
