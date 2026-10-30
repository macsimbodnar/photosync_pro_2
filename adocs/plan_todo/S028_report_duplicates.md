id:         S028
goal:       the report shows 'duplicate in session' apart from 'already on drive' and names the card copied from
accepts:    failing test first; CLI and TUI text show both kinds apart per source and name the card whose file was copied (FileOutcome::DuplicateInSession { of }), within the line-width rules; the text test that pins the omission (text.rs problem_line, asserted None) is the one test that changes, because the specs say otherwise: note this in the stamp; exFAT e2e asserts the sentence with two cards; suite and script pass
touches:    crates/photosync text.rs, tui/view.rs, tests, README
excludes:   core changes unless the data is missing
closes:     2026-10-30_adversarial-F03
paused_by:
author:
done:
