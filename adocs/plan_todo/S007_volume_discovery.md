id:         S007
goal:       core lists candidate sources (mounted volumes with DCIM) and destination drives, Mac then Linux
accepts:    on Mac, volumes under /Volumes; on Linux, under /media/$USER and /run/media/$USER; volumes with a DCIM folder are sources; the root search path can be injected, so tests run on temp dirs; checked by hand with a real card on Mac, recorded in the stamp; the X-T5 in USB card reader mode over cable is listed as a source and a sync from it succeeds, checked by hand on Mac and recorded in the stamp
touches:    photosync-core
excludes:   PTP; Windows; hot-plug notifications
closes:     2026-10-28_adversarial-F09
paused_by:
author:
done:
