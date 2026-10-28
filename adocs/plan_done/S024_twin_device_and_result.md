id:         S024
goal:       twins only on another device, and the one-sync result names an unread card even when not complete (S023 fast check)
accepts:    a twin must be on another device (st_dev) than the planned file's source, not just another source index, tested in core with the device seam (two sources on one device give no twin); the exFAT tests that need real twins use separate card images (the script mounts as many as needed), with their assertions unchanged; the one-sync `result_line` names every card with an unread file, by name, in both the complete and the NOT COMPLETE result, tested; each item gets a failing test first; scripts/test_exfat.sh passes; DEC-038 records the device rule
touches:    photosync-core plan.rs, crates/photosync text.rs, scripts/test_exfat.sh, tests
excludes:   parallel copying (S019); GUI
closes:
paused_by:
author:     Claude agent (2026-10-28)
done:       2026-10-28, Claude agent. Planning compares source devices (DEC-038, DEC-039 (1)): library_plan two_sources_on_one_device_give_no_twin failed first, then passed. One-sync result_line names each unread card, complete or NOT COMPLETE (DEC-039 (2)): text the_one_sync_result_names_every_unread_card_complete_or_not failed first; the old "on its card" assertions now name the card. Core twin tests scan each card on its own fake device (scan_card); the binary's two exFAT twin tests put cards a and b on two new 16 MB images (PHOTOSYNC_TEST_TWIN_A/B), assertions otherwise unchanged. cargo test --workspace green, clippy clean, scripts/test_exfat.sh exit 0, no image left attached.
