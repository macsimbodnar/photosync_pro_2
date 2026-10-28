id:         S019
goal:       copy several cards of one sync at once, each on its own thread, safely, with per-card progress
accepts:    tests: two sources copy concurrently (both cards are being read at the same time, shown by a test hook) with per-source progress in one event stream; renames into one folder stay serialised (DEC-008) and two cards' same-name different photos give one copy and one name collision, never an overwrite; the stale-temp cleanup still runs once, before any copy thread starts; the INV-10 re-check still runs before each folder is made, from every thread; a cancel stops every thread at its next check and the outcome lists each file once (INV-6); INV-1, INV-2, INV-7 hold under concurrency; scripts/test_exfat.sh passes; split from S010 (DEC-032)
touches:    photosync-core (copy, progress, session), crates/photosync (progress line), README.md
excludes:   GUI; copying one card's files in parallel
closes:
paused_by:
author:
done:
