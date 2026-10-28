id:         S021
goal:       when a read fails on one card, copy the same photo from its twin on another card (F07)
accepts:    tests: a planned file whose read fails is copied from the first readable twin (`Planned::twins`), checked against the planning hash with no new hashing; the twin that was read is reported copied and the unread file as "duplicate in session" with its read error shown; a twin whose read fails or whose content changed is skipped for the next; a drive-side failure is not retried on a twin; with every twin unreadable the file fails with its own read error and its twins are failed duplicates; INV-6 holds and `Report::new` refuses a twin the plan does not list; README and specs updated; split from S010 (DEC-032). Needs the user first: `report.rs` test `duplicate_of_an_original_that_failed_to_read_is_failed_not_skipped` makes the original unreadable while its twin reads fine and expects both failed, which is the F07 defect; the user says whether its twin becomes unreadable too
touches:    photosync-core (copy, report), crates/photosync (text), README.md, adocs/specs.md
excludes:   GUI
closes:     2026-10-28_adversarial-F07
paused_by:
author:
done:
