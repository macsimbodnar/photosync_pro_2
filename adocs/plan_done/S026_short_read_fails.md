id:         S026
goal:       a file that reads shorter or longer than scanned is a failure, never copied
accepts:    failing test first (scan, shrink the source, plan, copy, Report): the file fails with a reason before the rename, no final file exists, the report is not complete and copied bytes count only what is on the drive; a source that grew is handled the same way, or a decision says why not; suite and scripts/test_exfat.sh pass
touches:    photosync-core copy.rs, report, text for any new reason, specs
excludes:   changing dedup
closes:     2026-10-30_adversarial-F01
paused_by:
author:     Claude agent (2026-10-30)
done:       2026-10-30, Claude agent. The copy compares bytes read with the scanned size before the hash check and the rename; any difference fails with the new FailReason::SizeChanged (DEC-042, agent's pick, not yet confirmed). A file that grew fails the same way. Six tests failed first, then passed: copy_engine a_file_that_reads_shorter_than_scanned_fails (no temp, only the healthy file on the drive, not complete, copied bytes = the healthy file's), a_file_that_reads_empty_fails, a_file_that_reads_longer_than_scanned_fails, a_short_read_that_matches_the_planning_hash_still_fails, a_short_read_after_planning_does_not_fall_back_to_the_twin; unit f07_a_twin_that_reads_short_is_passed_for_the_next (also fails with SizeChanged taken out of the twin arm). cargo test --workspace green, clippy clean, scripts/test_exfat.sh exit 0, no image left attached. Specs INV-3 and README updated. Trigger on real hardware still unconfirmed, as the audit says.
