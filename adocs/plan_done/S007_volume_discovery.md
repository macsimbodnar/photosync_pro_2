id:         S007
goal:       core lists candidate sources (mounted volumes with DCIM) and destination drives, Mac then Linux
accepts:    on Mac, volumes under /Volumes; on Linux, under /media/$USER and /run/media/$USER; volumes with a DCIM folder are sources; the root search path can be injected, so tests run on temp dirs; INV-10 also refuses a library on the same device (st_dev) as a source, so two paths to one card volume cannot bypass the path-prefix check; hand checks moved to S008, which has the CLI to run them
touches:    photosync-core
excludes:   PTP; Windows; hot-plug notifications
closes:
paused_by:
author:     Claude agent (2026-10-28)
done:       2026-10-28, Claude agent. `discover_volumes(roots)` + `default_search_roots()` (/Volumes; /media/$USER, /run/media/$USER), sources = mounted volumes with DCIM, destinations = other writable mounted volumes; boot-volume symlink, leftover folders and dot entries excluded; `SourceScan::device` + INV-10 device check (DEC-024). Proven by volume::tests (11, roots injected, mount test injected) and library_plan inv10_library_on_the_device_of_a_source_volume_is_refused / inv10_library_on_another_device_than_the_source_volume_is_planned; mutation checks: following symlinks fails the boot test, dropping the device clause fails the device test. `cargo test --workspace` green (114), clippy clean, scripts/test_exfat.sh exit 0, no image left mounted (2 new permission tests added to its skip list). In scope, also: unit tests share one `common` module (lib.rs), as two copies of `TempDir`'s counter collided; `copy::c_path` made pub(crate). Linux code paths compiled and tested on macOS only.
