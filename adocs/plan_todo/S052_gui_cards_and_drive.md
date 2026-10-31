id:         S052
goal:       the cards and drive screens by the TUI's rules: pick several cards, pick a drive, look again
accepts:    by clicks through the Simulator, as DEC-068 (2) maps them: ticking two rows picks both and the check screen later lists both; unticking drops one; Next with none ticked takes the highlighted row; Next with no card shows the empty sentence, and the hint with no key name in it; Look again runs the injected discovery again, keeps what is still there, and a picked card that went away is unticked; the drive screen marks the chosen drive, Next with no drive says why, Back returns to the cards, and a drive picked again keeps an edited library while a new one sets `<drive>/Photos`; a card or drive path of 300 bytes and a label `写真カード` show whole; many notes scroll and leave the rows' sentence in sight; no key name (`Esc`, `Space`, `press`, `Ctrl-C`) in any words shown, by a test over every hint this reaches (DEC-068 (3)); an Emulator `play` clicks from the cards to the library screen; Screen check as DEC-066 (5): `S052_cards_two_picked.png`, `S052_cards_long_paths.png`, `S052_cards_notes.png`, `S052_drive.png`, `S052_drive_empty.png`, read and described in the stamp; suite and scripts/test_exfat.sh pass
touches:    crates/photosync-gui
excludes:   crates/photosync; a folder picker; watching for mounts
closes:     
paused_by:
author:
done:
