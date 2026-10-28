id:         S007
goal:       core lists candidate sources (mounted volumes with DCIM) and destination drives, Mac then Linux
accepts:    on Mac, volumes under /Volumes; on Linux, under /media/$USER and /run/media/$USER; volumes with a DCIM folder are sources; the root search path can be injected, so tests run on temp dirs; INV-10 also refuses a library on the same device (st_dev) as a source, so two paths to one card volume cannot bypass the path-prefix check; hand checks moved to S008, which has the CLI to run them
touches:    photosync-core
excludes:   PTP; Windows; hot-plug notifications
closes:
paused_by:
author:
done:
