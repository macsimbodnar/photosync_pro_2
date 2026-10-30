id:         S026
goal:       a file that reads shorter or longer than scanned is a failure, never copied
accepts:    failing test first (scan, shrink the source, plan, copy, Report): the file fails with a reason before the rename, no final file exists, the report is not complete and copied bytes count only what is on the drive; a source that grew is handled the same way, or a decision says why not; suite and scripts/test_exfat.sh pass
touches:    photosync-core copy.rs, report, text for any new reason, specs
excludes:   changing dedup
closes:     2026-10-30_adversarial-F01
paused_by:
author:
done:
