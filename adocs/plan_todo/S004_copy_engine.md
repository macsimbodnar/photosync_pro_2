id:         S004
goal:       core executes a sync plan: target folder naming, temp copy, blake3 verify, atomic rename, collision suffix
accepts:    tests for INV-2, INV-3, INV-4, INV-5 and INV-7 (simulated interruption leaves no file under a final name; re-run completes); folder is `<earliest new-file date>[ - <name>]/<TYPE>/`; a same-name different-content file gets `_1`, `_2`, ...; running the same sync twice copies zero files the second time
touches:    photosync-core
excludes:   progress events; UI
closes:
paused_by:
author:
done:
