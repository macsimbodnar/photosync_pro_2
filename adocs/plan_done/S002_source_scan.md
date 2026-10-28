id:         S002
goal:       core scans a source's DCIM tree and sorts files into RAW/JPEG/HEIF/VIDEO/ignored
accepts:    given a temp DCIM tree, the scan returns every file with its type (case-insensitive extensions per specs Layout), size, and modification time; unknown types listed as ignored; a test proves the scan leaves the source manifest unchanged (INV-1)
touches:    photosync-core
excludes:   copying; volume discovery
closes:
paused_by:
author:     Claude agent (2026-10-28)
done:       2026-10-28 — `cargo test --workspace` green: 18 tests (8 new in crates/photosync-core/tests/source_scan.rs). Accepts proven by scan_returns_every_file_with_type_size_and_mtime (all four types, mixed-case extensions, nested folders, exact size and mtime), unknown_types_are_listed_as_ignored, inv1_scan_leaves_source_manifest_unchanged (path + mtime + full contents of every file and folder under the source root; shown to fail when a write was put into scan_source, then reverted). Also: files outside DCIM not scanned, missing DCIM is an error, empty DCIM, symlinks not followed (listed as ignored), MediaType::dir_name. Contents stand in for the INV-1 hash, so no dependency was added.
