id:         S013
goal:       copy engine hardening from the S004 fast check
accepts:    a session folder or `<TYPE>` folder that is a symlink is refused, never followed (INV-9/INV-10), with a test; the INV-3 flush test fails if `sync_all` on the file or the folder fsync is removed (prove it by removing each, then restore); a test injects a read failure partway through a copy and proves the partial temp is removed and only that file fails; when a file is in place but the final folder flush fails, the failure reason says that the file is in place; each item has a failing test before its fix; the exFAT fallback lock is keyed by the canonical target folder (not the library), so two paths into one folder share a lock, with a test (S012 fast check; amends DEC-019 point 1); scripts/test_exfat.sh reports a failed detach clearly and does not `rm -rf` through a still-mounted image
touches:    photosync-core copy engine, scripts/test_exfat.sh
excludes:   progress/cancel; parallel copying
closes:
paused_by:
author:
done:
