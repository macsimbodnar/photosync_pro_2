id:         S038
goal:       a duplicate's report line names the card whose file is on the drive, also when the original was found in an earlier sync of the session
accepts:    failing core test first: one session, card A with X, then card B with X in both 100_FUJI and 101_FUJI: both of B's lines name A, never B (specs Sources, "the card whose file is on the drive now"); the same for A, then B+C together; `of` is resolved through `seen_earlier` in the plan or in Report::build; a_twin_on_a_later_card and the NoOriginal check still hold; suite and scripts/test_exfat.sh pass
touches:    photosync-core plan.rs or report.rs, tests
excludes:   the wording of the line
closes:
paused_by:
author:
done:
