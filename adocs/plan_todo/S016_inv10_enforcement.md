id:         S016
goal:       enforce INV-10 for real: end-to-end card test, read-only source device, re-check before copying (S015 fast check)
accepts:    an end-to-end `plan_sync` test with the real volume lookup refuses a library on a volume with DCIM at its top: scripts/test_exfat.sh mounts a second small image with a DCIM folder as the library volume and runs it (the test is skipped or reports clearly when that image is absent, never passes silently: e.g. env var set by the script, test fails if the var is set and the refusal does not happen); replacing the real lookup with `|_| Ok(None)` makes that run fail (prove it, then restore); `SourceScan::device` is read-only (private field + getter) and tests set a fake device only through a cfg(test)/test-support seam that a normal build cannot reach from outside the crate; `SyncPlan` records the library's st_dev and volume root at planning, and `copy_plan` re-checks both (and that no source device equals the library's) before creating any folder or file, refusing with a clear error, tested by simulating a changed device; `inv10_library_on_the_device_of_a_source_volume_is_refused` is renamed or merged so its name matches what it tests, assertions not weakened; each item gets a failing test first
touches:    photosync-core plan/source/copy/volume, tests, scripts/test_exfat.sh, README.md
excludes:   CLI (S008)
closes:
paused_by:
author:
done:
