id:         S065
goal:       Start copying never freezes the window, a late key cannot land in the other field, and Esc in a field goes back
accepts:    failing tests first: the check before a session (flow::start_found, canonicalize, symlink_metadata) runs on its own thread in the GUI, with a blocked injected check: the window stays live, shows a "checking" state, a close is heard, Start is held, a second press starts nothing, and an answer that is out of date is dropped, as for looks (DEC-073); the TUI's order and tests are unchanged; a field's message names its screen, so `M::Field` for Library arriving on Name and the reverse are dropped, tested by sending both directly; Esc on Library or Name acts as Back and keeps the typed text as Back does, and typing and Enter keep working on the next screen with no click, tested in the Emulator where it can and named in S060 for the real window; pictures of the checking state read at 640x480 and 900x650; suite and scripts/test_exfat.sh pass
touches:    crates/photosync-gui src/gui.rs, widgets.rs, tests; S060 file; decisions; specs if they name it
excludes:   the wording of refusals; the session's rules
closes:
paused_by:
author:
done:
