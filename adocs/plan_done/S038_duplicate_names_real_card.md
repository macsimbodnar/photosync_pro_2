id:         S038
goal:       a duplicate's report line names the card whose file is on the drive, also when the original was found in an earlier sync of the session
accepts:    failing core test first: one session, card A with X, then card B with X in both 100_FUJI and 101_FUJI: both of B's lines name A, never B (specs Sources, "the card whose file is on the drive now"); the same for A, then B+C together; `of` is resolved through `seen_earlier` in the plan or in Report::build; a_twin_on_a_later_card and the NoOriginal check still hold; suite and scripts/test_exfat.sh pass
touches:    photosync-core plan.rs or report.rs, tests
excludes:   the wording of the line
closes:
paused_by:
author:     Claude
done:       2026-10-30. Failing first: three tests in tests/session.rs (later card with two copies, later cards B+C, [B, A] again) and a_duplicate_of_a_file_found_in_an_earlier_sync_is_refused in session.rs. Then green: cargo test --workspace 274 passed, 0 failed; scripts/test_exfat.sh exit 0, 259 passed, 0 failed, no image left attached. a_twin_on_a_later_card and the NoOriginal checks unchanged and pass. Also found: A given again after B was a duplicate of B; now already on drive (DEC-045). README checked, no change: it says "on drive from". Specs: refused plans wording.
