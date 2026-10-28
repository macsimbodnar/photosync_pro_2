//! S003: scanning the library into memory and planning which source files
//! are already synced; INV-9 and INV-10 for planning.

use std::fs;
use std::path::{Path, PathBuf};

use photosync_core::{Duplicate, PlanError, Skipped, SourceScan, SyncPlan, plan_sync, scan_source};

mod common;
use common::{TempDir, at, manifest, write_file};
#[cfg(unix)]
use common::Mode;

/// Writes `files` (path under `root`, contents) with a fixed mtime.
fn put(root: &Path, files: &[(&str, &[u8])]) {
    for (rel, bytes) in files {
        write_file(root, rel, bytes, at(1_700_000_000));
    }
}

fn scan(root: &Path) -> SourceScan {
    scan_source(root).unwrap()
}

fn paths(files: &[photosync_core::SourceFile]) -> Vec<PathBuf> {
    files.iter().map(|f| f.path.clone()).collect()
}

/// INV-6 holds for the plan: each supported file is in exactly one list.
fn assert_complete(plan: &SyncPlan, sources: &[SourceScan]) {
    let supported: usize = sources.iter().map(|s| s.supported().count()).sum();
    let planned =
        plan.to_copy.len() + plan.skipped.len() + plan.duplicates.len() + plan.failures.len();
    assert_eq!(planned, supported);
}

#[test]
fn same_name_size_and_hash_anywhere_in_library_is_skipped() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"raw one"),
        ("DCIM/100_FUJI/DSCF0002.RAF", b"raw two"),
    ]);
    // Synced earlier into another session's folder: still counts.
    put(&lib, &[("2025-01-01 - Old/RAW/DSCF0001.RAF", b"raw one")]);
    let sources = [scan(&card)];

    let plan = plan_sync(&lib, &sources).unwrap();

    assert_eq!(
        plan.skipped,
        [Skipped {
            file: sources[0].files[0].clone(),
            library_file: lib.join("2025-01-01 - Old/RAW/DSCF0001.RAF"),
        }]
    );
    assert_eq!(paths(&plan.to_copy), [card.join("DCIM/100_FUJI/DSCF0002.RAF")]);
    assert!(plan.failures.is_empty() && plan.library_failures.is_empty());
    assert_complete(&plan, &sources);
}

#[test]
fn same_name_and_size_with_different_content_is_not_skipped() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"new shot")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"old shot")]);
    let sources = [scan(&card)];

    let plan = plan_sync(&lib, &sources).unwrap();

    assert!(plan.skipped.is_empty());
    assert_eq!(paths(&plan.to_copy), [card.join("DCIM/100_FUJI/DSCF0001.RAF")]);
}

/// Larger than any read buffer, differing only in the last byte: the whole
/// file is hashed, not a prefix.
#[test]
fn large_file_differing_in_last_byte_is_not_skipped() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    let mut bytes = vec![0x5a_u8; 3 * 1024 * 1024 + 1];
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", &bytes)]);
    *bytes.last_mut().unwrap() ^= 1;
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", &bytes)]);

    let plan = plan_sync(&lib, &[scan(&card)]).unwrap();

    assert!(plan.skipped.is_empty());
    assert_eq!(plan.to_copy.len(), 1);
}

#[test]
fn same_content_under_another_name_is_not_skipped() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("2025-01-01/RAW/DSCF9999.RAF", b"raw")]);

    let plan = plan_sync(&lib, &[scan(&card)]).unwrap();

    assert!(plan.skipped.is_empty());
    assert_eq!(plan.to_copy.len(), 1);
}

#[test]
fn file_on_two_sources_is_planned_once() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    // Backup slots: same photo on both cards. Plus a same-name, same-size,
    // different photo (counter reset), which is not a duplicate.
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"same"), ("DCIM/100_FUJI/DSCF0002.RAF", b"on a")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"same"), ("DCIM/100_FUJI/DSCF0002.RAF", b"on b")]);
    fs::create_dir(&lib).unwrap();
    let sources = [scan(&a), scan(&b)];

    let plan = plan_sync(&lib, &sources).unwrap();

    assert_eq!(
        paths(&plan.to_copy),
        [
            a.join("DCIM/100_FUJI/DSCF0001.RAF"),
            a.join("DCIM/100_FUJI/DSCF0002.RAF"),
            b.join("DCIM/100_FUJI/DSCF0002.RAF"),
        ]
    );
    assert_eq!(
        plan.duplicates,
        [Duplicate { file: sources[1].files[0].clone(), of: a.join("DCIM/100_FUJI/DSCF0001.RAF") }]
    );
    assert_complete(&plan, &sources);
}

#[test]
fn file_on_two_sources_already_in_library_is_skipped_once() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"same")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"same")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"same")]);
    let sources = [scan(&a), scan(&b)];

    let plan = plan_sync(&lib, &sources).unwrap();

    assert!(plan.to_copy.is_empty());
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.skipped[0].file.path, a.join("DCIM/100_FUJI/DSCF0001.RAF"));
    assert_eq!(plan.duplicates.len(), 1);
    assert_eq!(plan.duplicates[0].file.path, b.join("DCIM/100_FUJI/DSCF0001.RAF"));
    assert_complete(&plan, &sources);
}

#[test]
fn photosync_temp_file_does_not_count_as_synced() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("2026-10-28/RAW/.photosync-tmp-DSCF0001.RAF", b"raw")]);

    let plan = plan_sync(&lib, &[scan(&card)]).unwrap();

    assert!(plan.skipped.is_empty());
    assert_eq!(plan.to_copy.len(), 1);
}

#[test]
fn inv9_planning_writes_nothing() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    // Every path through the planner: skipped, duplicate, collision, new.
    put(&a, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"synced"),
        ("DCIM/100_FUJI/DSCF0002.RAF", b"on both"),
        ("DCIM/100_FUJI/DSCF0003.RAF", b"newer"),
        ("DCIM/100_FUJI/DSCF0004.JPG", b"unique"),
    ]);
    put(&b, &[("DCIM/100_FUJI/DSCF0002.RAF", b"on both")]);
    put(&lib, &[
        ("2025-01-01/RAW/DSCF0001.RAF", b"synced"),
        ("2025-01-01/RAW/DSCF0003.RAF", b"older"),
        ("2025-01-01/RAW/.photosync-tmp-DSCF0005.RAF", b"stale"),
    ]);
    let sources = [scan(&a), scan(&b)];
    // Library and sources share one parent, so a file created anywhere
    // under it, even one deleted again, changes a folder mtime or a path.
    let before = manifest(&dir.0);

    let plan = plan_sync(&lib, &sources).unwrap();

    assert_eq!((plan.skipped.len(), plan.duplicates.len(), plan.to_copy.len()), (1, 1, 3));
    assert_eq!(manifest(&dir.0), before);
}

fn refused(library: &Path, sources: &[SourceScan]) -> PlanError {
    plan_sync(library, sources).unwrap_err()
}

#[test]
fn inv10_library_on_a_source_is_refused() {
    let dir = TempDir::new();
    let card = dir.0.join("card");
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir(card.join("Photos")).unwrap();
    let sources = [scan(&card)];

    for library in [card.clone(), card.join("Photos")] {
        match refused(&library, &sources) {
            PlanError::LibraryOnSource { library: l, source_root } => {
                assert_eq!((l, source_root), (library, card.clone()));
            }
            other => panic!("expected LibraryOnSource, got {other:?}"),
        }
    }
}

#[test]
fn inv10_library_inside_a_dcim_tree_is_refused() {
    let dir = TempDir::new();
    let card = dir.0.join("card");
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    let sources = [scan(&card)];
    // A drive that is not a source but holds a DCIM tree; cards ignore case.
    let inside =
        [dir.0.join("drive/DCIM"), dir.0.join("drive/DCIM/lib"), dir.0.join("drive/dcim/lib")];

    for library in inside {
        fs::create_dir_all(&library).unwrap();
        match refused(&library, &sources) {
            PlanError::LibraryInDcim { library: l } => assert_eq!(l, library),
            other => panic!("expected LibraryInDcim, got {other:?}"),
        }
    }
}

#[test]
fn inv10_library_containing_a_source_is_refused() {
    let dir = TempDir::new();
    let card = dir.0.join("lib/imports/card");
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    let library = dir.0.join("lib");
    let sources = [scan(&card)];

    match refused(&library, &sources) {
        PlanError::LibraryContainsSource { library: l, source_root } => {
            assert_eq!((l, source_root), (library, card));
        }
        other => panic!("expected LibraryContainsSource, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn inv10_symlink_cannot_hide_a_library_on_a_source() {
    let dir = TempDir::new();
    let card = dir.0.join("card");
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir(card.join("Photos")).unwrap();
    let link = dir.0.join("lib");
    std::os::unix::fs::symlink(card.join("Photos"), &link).unwrap();

    let err = refused(&link, &[scan(&card)]);

    assert!(matches!(err, PlanError::LibraryOnSource { .. }), "got {err:?}");
}

#[test]
fn missing_library_is_an_error_naming_the_path() {
    let dir = TempDir::new();
    let card = dir.0.join("card");
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    let library = dir.0.join("nope");

    match refused(&library, &[scan(&card)]) {
        PlanError::Library(e) => {
            assert_eq!(e.path, library);
            assert_eq!(e.source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected Library, got {other:?}"),
    }
}

/// An unreadable file with no name + size candidate still lands in
/// `to_copy`: proof it was never read. Only candidates are hashed.
#[cfg(unix)]
#[test]
fn only_name_and_size_candidates_are_hashed() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"unique")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0999.RAF", b"unrelated")]);
    let _src = Mode::set(&card.join("DCIM/100_FUJI/DSCF0001.RAF"), 0o000);
    let _lib = Mode::set(&lib.join("2025-01-01/RAW/DSCF0999.RAF"), 0o000);

    let plan = plan_sync(&lib, &[scan(&card)]).unwrap();

    assert_eq!(paths(&plan.to_copy), [card.join("DCIM/100_FUJI/DSCF0001.RAF")]);
    assert!(plan.failures.is_empty() && plan.library_failures.is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_source_file_fails_that_file_only() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw"), ("DCIM/100_FUJI/DSCF0002.RAF", b"two")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"raw")]);
    let locked = card.join("DCIM/100_FUJI/DSCF0001.RAF");
    let _mode = Mode::set(&locked, 0o000);
    let sources = [scan(&card)];

    let plan = plan_sync(&lib, &sources).unwrap();

    assert_eq!(plan.failures.len(), 1);
    assert_eq!(plan.failures[0].path, locked);
    assert_eq!(plan.failures[0].source.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(paths(&plan.to_copy), [card.join("DCIM/100_FUJI/DSCF0002.RAF")]);
    assert_complete(&plan, &sources);
}

/// An unreadable library copy is not a safe copy: the photo is planned.
#[cfg(unix)]
#[test]
fn unreadable_library_file_is_a_library_failure_and_the_file_is_planned() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"raw")]);
    let locked = lib.join("2025-01-01/RAW/DSCF0001.RAF");
    let _mode = Mode::set(&locked, 0o000);

    let plan = plan_sync(&lib, &[scan(&card)]).unwrap();

    assert_eq!(plan.library_failures.len(), 1);
    assert_eq!(plan.library_failures[0].path, locked);
    assert!(plan.skipped.is_empty() && plan.failures.is_empty());
    assert_eq!(paths(&plan.to_copy), [card.join("DCIM/100_FUJI/DSCF0001.RAF")]);
}

#[cfg(unix)]
#[test]
fn unreadable_library_folder_is_a_library_failure() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"raw")]);
    let locked = lib.join("2025-01-01");
    let _mode = Mode::set(&locked, 0o000);

    let plan = plan_sync(&lib, &[scan(&card)]).unwrap();

    assert_eq!(plan.library_failures.len(), 1);
    assert_eq!(plan.library_failures[0].path, locked);
    assert_eq!(plan.to_copy.len(), 1);
}

#[test]
fn trashed_and_hidden_library_files_do_not_count_as_synced() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"raw one"),
        ("DCIM/100_FUJI/DSCF0002.RAF", b"raw two"),
        ("DCIM/100_FUJI/DSCF0003.RAF", b"raw three"),
        ("DCIM/100_FUJI/DSCF0004.RAF", b"raw four"),
    ]);
    // Deleted in Finder / Explorer, or hidden: the photo may vanish when the
    // trash is emptied, so these must never stand in for a synced copy.
    put(&lib, &[
        (".Trashes/501/DSCF0001.RAF", b"raw one"),
        ("$RECYCLE.BIN/S-1-5-21/DSCF0002.RAF", b"raw two"),
        ("System Volume Information/DSCF0003.RAF", b"raw three"),
        ("2026-10-28/RAW/.DSCF0004.RAF", b"raw four"),
    ]);
    let sources = [scan(&card)];

    let plan = plan_sync(&lib, &sources).unwrap();

    assert!(plan.skipped.is_empty(), "skipped: {:?}", plan.skipped);
    assert_eq!(plan.to_copy.len(), 4);
    assert_complete(&plan, &sources);
}
