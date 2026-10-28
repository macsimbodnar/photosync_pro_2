id:         S005
goal:       core emits progress events (files and bytes per source and overall, ETA) and supports cancel
accepts:    tests: events arrive in order and their totals match the plan; ETA is present once some bytes are copied; cancel stops at a file boundary or removes the temp file, satisfying INV-7; the core prints nothing (INV-8)
touches:    photosync-core
excludes:   rendering of progress
closes:
paused_by:
author:
done:
