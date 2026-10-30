id:         S034
goal:       stale-temp cleanup leaves a temp whose writing process is alive
accepts:    failing test first: a temp named with a live pid survives cleanup and one with a dead pid goes; two processes on one library, reproduced as in the finding, both finish with no failure on an exFAT image; the temp's own pid counts as live only for an earlier run's file if the design needs it, recorded; a pid reused by another program only leaves a temp for later, which is safe, say so in specs; INV-2 wording stays true; suite and script pass
touches:    photosync-core copy.rs, specs, decisions
excludes:   library-wide locks across processes unless the test shows they are needed
closes:     2026-10-30_adversarial-F09
paused_by:
author:
done:
