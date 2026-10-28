id:         S006
goal:       core produces a sync report: copied, skipped, ignored, failed with reason (incl. name collision), totals per source and type
accepts:    test: copied + skipped + failed equals the supported files found for every source (INV-6); ignored files and name collisions listed; a failed file (e.g. unreadable) is reported with a reason and the sync continues; scan failures (`SourceScan.failures`) are reported per entry; a failed folder is reported as "contents unknown" and its unknown files are outside the INV-6 count, which covers the files the scan listed; the INV-6 wording in specs is updated to say so
touches:    photosync-core
excludes:   formatting for display
closes:
paused_by:
author:
done:
