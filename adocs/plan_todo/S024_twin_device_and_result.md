id:         S024
goal:       twins only on another device, and the one-sync result names an unread card even when not complete (S023 fast check)
accepts:    a twin must be on another device (st_dev) than the planned file's source, not just another source index, tested in core with the device seam (two sources on one device give no twin); the exFAT tests that need real twins use separate card images (the script mounts as many as needed), with their assertions unchanged; the one-sync `result_line` names every card with an unread file, by name, in both the complete and the NOT COMPLETE result, tested; each item gets a failing test first; scripts/test_exfat.sh passes; DEC-038 records the device rule
touches:    photosync-core plan.rs, crates/photosync text.rs, scripts/test_exfat.sh, tests
excludes:   parallel copying (S019); GUI
closes:
paused_by:
author:
done:
