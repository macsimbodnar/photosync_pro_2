id:         S063
goal:       the GUI's drive screen marks the drive Next will take, always
accepts:    failing tests first: choose A, go back from the library, click B, and the mark moves to B at once (a click picks a drive, as a radio button does; one mark only, no separate tick and bar) and Next takes B; with no click on a row, Next takes the marked drive and the library is reset only when the drive really changed; shared layer: App::volumes keeps the cursor on the chosen drive by path across a look, so drives [A,B] with B chosen, then a new drive that sorts first and Look again, leaves the mark on B and Next takes B, tested in crates/photosync's app tests, and the TUI gets the same rule with its tests unchanged or a recorded reason; a chosen drive that is gone after a look has no mark and Next is held with a hint; a double click on the cards' Next does not take a drive unseen on a first visit: the drive screen has no mark until one is picked when two or more drives are listed, or a recorded reason why not; every drive-screen picture is read again at 640x480 and 900x650; DEC-075 (5) amended by a new decision, the agent's choice under the user's standing instruction, not yet confirmed; suite and scripts/test_exfat.sh pass
touches:    crates/photosync-gui src/gui.rs, widgets, tests; crates/photosync src/app.rs and tests; decisions
excludes:   the cards screen's rule; the library screen (S053)
closes:
paused_by:
author:
done:
