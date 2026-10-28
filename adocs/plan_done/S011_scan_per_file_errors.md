id:         S011
goal:       source scan reports unreadable entries per file instead of failing the whole source
accepts:    an unreadable root or DCIM is still an `Err`; an unreadable file or sub-folder under DCIM goes into the scan as a per-entry failure with its path and error, and every other file is still listed; tests simulate it (e.g. a sub-folder with no read permission, unix); existing source_scan tests stay green unchanged; INV-1 holds
touches:    photosync-core source scan
excludes:   report formatting (S006); duplicate fallback (S010)
closes:
paused_by:
author:     Claude agent (2026-10-28)
done:       2026-10-28 — `cargo test --workspace` green: 25 tests (6 new in crates/photosync-core/tests/source_scan.rs, written first and failing); clippy clean. `SourceScan` gained `failures: Vec<ScanError>`, sorted by path. Accepts proven by unreadable_subfolder_is_a_failure_and_the_rest_is_listed (mode 000 folder: one failure, PermissionDenied, every other sample_card file listed exactly), unreadable_files_are_failures_and_the_rest_is_listed (mode 444 folder: names listed, metadata denied, each file a failure), unreadable_dcim_is_an_error, unreadable_root_is_an_error, readable_source_has_no_failures, inv1_scan_with_failures_leaves_source_manifest_unchanged. The 9 existing source_scan tests are unchanged. `SourceScan` lost its Clone/PartialEq/Eq derives because io::Error has neither; no code used them. A broken listing mid-folder keeps what it gave so far and records the folder as failed. Specs Sources gained one bullet for the behaviour.
