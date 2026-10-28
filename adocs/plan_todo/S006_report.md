id:         S006
goal:       core produces a sync report: copied, skipped, ignored, failed with reason (incl. name collision), totals per source and type
accepts:    test: copied + skipped + failed equals the supported files found for every source (INV-6); ignored files and name collisions listed; a failed file (e.g. unreadable) is reported with a reason and the sync continues; scan failures (`SourceScan.failures`) are reported per entry; a failed folder is reported as "contents unknown" and its unknown files are outside the INV-6 count, which covers the files the scan listed; the INV-6 wording in specs is updated to say so; "duplicate in session" files count as skipped in the INV-6 sum (copied + skipped + failed = supported, per source), with their own reason; the INV-6 wording in specs is updated to say so; cancelled files (`CopyOutcome.cancelled`) are their own report bucket, and INV-6 becomes copied + skipped + failed + cancelled = supported, per source (S005 fast check); a test proves a plan cancelled before it starts sends no FileStarted or Bytes events
touches:    photosync-core
excludes:   formatting for display
closes:
paused_by:
author:
done:
