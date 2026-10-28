//! S002: scanning a source's DCIM tree, and INV-1 for the scan.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use photosync_core::{MediaType, scan_source};

mod common;
use common::{TempDir, at, manifest, write_file};
#[cfg(unix)]
use common::Mode;

/// A card as the X-T5 writes it, plus odd cases: mixed-case extensions, a
/// nested folder, a file with no extension, an unknown type.
fn sample_card(root: &Path) -> Vec<(PathBuf, Option<MediaType>, u64, SystemTime)> {
    let files: &[(&str, Option<MediaType>, usize, u64)] = &[
        ("DCIM/100_FUJI/DSCF0001.RAF", Some(MediaType::Raw), 30, 1_700_000_001),
        ("DCIM/100_FUJI/DSCF0001.JPG", Some(MediaType::Jpeg), 20, 1_700_000_002),
        ("DCIM/100_FUJI/dscf0002.raf", Some(MediaType::Raw), 31, 1_700_000_003),
        ("DCIM/100_FUJI/DSCF0002.jpeg", Some(MediaType::Jpeg), 21, 1_700_000_004),
        ("DCIM/100_FUJI/DSCF0003.HIF", Some(MediaType::Heif), 12, 1_700_000_005),
        ("DCIM/100_FUJI/DSCF0004.Heif", Some(MediaType::Heif), 13, 1_700_000_006),
        ("DCIM/100_FUJI/DSCF0005.MOV", Some(MediaType::Video), 50, 1_700_000_007),
        ("DCIM/101_FUJI/sub/DSCF0006.mp4", Some(MediaType::Video), 51, 1_700_000_008),
        ("DCIM/100_FUJI/DSCF0007.WAV", None, 5, 1_700_000_009),
        ("DCIM/100_FUJI/NOEXT", None, 4, 1_700_000_010),
        ("DCIM/100_FUJI/raf", None, 3, 1_700_000_011),
    ];
    files
        .iter()
        .map(|&(rel, t, len, secs)| {
            let path = write_file(root, rel, &vec![7u8; len], at(secs));
            (path, t, len as u64, at(secs))
        })
        .collect()
}

#[test]
fn scan_returns_every_file_with_type_size_and_mtime() {
    let dir = TempDir::new();
    let mut expected = sample_card(&dir.0);
    expected.sort_by(|a, b| a.0.cmp(&b.0));

    let scan = scan_source(&dir.0).unwrap();

    assert_eq!(scan.root, dir.0);
    let got: Vec<_> = scan
        .files
        .iter()
        .map(|f| (f.path.clone(), f.media_type, f.size, f.modified))
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn unknown_types_are_listed_as_ignored() {
    let dir = TempDir::new();
    sample_card(&dir.0);

    let scan = scan_source(&dir.0).unwrap();

    let names = |it: &mut dyn Iterator<Item = &photosync_core::SourceFile>| -> Vec<String> {
        it.map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned()).collect()
    };
    assert_eq!(names(&mut scan.ignored()), ["DSCF0007.WAV", "NOEXT", "raf"]);
    assert_eq!(scan.supported().count(), 8);
}

#[test]
fn files_outside_dcim_are_not_scanned() {
    let dir = TempDir::new();
    write_file(&dir.0, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1));
    write_file(&dir.0, "MISC/DSCF0009.RAF", b"raw", at(1));
    write_file(&dir.0, "DSCF0010.JPG", b"jpg", at(1));

    let scan = scan_source(&dir.0).unwrap();

    assert_eq!(scan.files.len(), 1);
    assert_eq!(scan.files[0].path, dir.0.join("DCIM/100_FUJI/DSCF0001.RAF"));
}

#[test]
fn empty_dcim_scans_to_no_files() {
    let dir = TempDir::new();
    fs::create_dir(dir.0.join("DCIM")).unwrap();
    assert!(scan_source(&dir.0).unwrap().files.is_empty());
}

#[test]
fn missing_dcim_is_an_error_naming_the_path() {
    let dir = TempDir::new();
    let err = scan_source(&dir.0).unwrap_err();
    assert_eq!(err.path, dir.0.join("DCIM"));
    assert_eq!(err.source.kind(), std::io::ErrorKind::NotFound);
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed() {
    let dir = TempDir::new();
    let outside = write_file(&dir.0, "elsewhere/DSCF0099.RAF", b"raw", at(1));
    fs::create_dir_all(dir.0.join("DCIM/100_FUJI")).unwrap();
    std::os::unix::fs::symlink(&outside, dir.0.join("DCIM/100_FUJI/LINK.RAF")).unwrap();
    std::os::unix::fs::symlink(&dir.0, dir.0.join("DCIM/loop")).unwrap();

    let scan = scan_source(&dir.0).unwrap();

    // The loop would recurse forever if followed; both links are ignored.
    assert_eq!(scan.files.len(), 2);
    assert!(scan.files.iter().all(|f| f.media_type.is_none()));
}

#[test]
fn media_type_maps_to_layout_folder_names() {
    let names: Vec<_> = [MediaType::Raw, MediaType::Jpeg, MediaType::Heif, MediaType::Video]
        .into_iter()
        .map(MediaType::dir_name)
        .collect();
    assert_eq!(names, ["RAW", "JPEG", "HEIF", "VIDEO"]);
}

#[test]
fn inv1_scan_leaves_source_manifest_unchanged() {
    let dir = TempDir::new();
    sample_card(&dir.0);
    write_file(&dir.0, "MISC/notes.txt", b"outside DCIM", at(1_600_000_000));
    let before = manifest(&dir.0);
    assert_eq!(before.values().filter(|(_, b)| b.is_some()).count(), 12);

    scan_source(&dir.0).unwrap();

    assert_eq!(manifest(&dir.0), before);
}

#[test]
fn dot_files_and_dot_folders_are_ignored() {
    let dir = TempDir::new();
    write_file(&dir.0, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1_700_000_001));
    write_file(&dir.0, "DCIM/100_FUJI/._DSCF0001.RAF", b"appledouble", at(1_700_000_002));
    write_file(&dir.0, "DCIM/.hidden/DSCF0002.JPG", b"jpg", at(1_700_000_003));

    let scan = scan_source(&dir.0).unwrap();

    let supported: Vec<_> = scan.supported().map(|f| f.path.clone()).collect();
    assert_eq!(supported, vec![dir.0.join("DCIM/100_FUJI/DSCF0001.RAF")]);
    // The dot folder is one ignored entry; its contents are not walked.
    let ignored: Vec<_> = scan.ignored().map(|f| f.path.clone()).collect();
    assert_eq!(
        ignored,
        vec![dir.0.join("DCIM/.hidden"), dir.0.join("DCIM/100_FUJI/._DSCF0001.RAF")]
    );
}

// S011: unreadable entries under DCIM are per-entry failures, not a failed scan.

#[cfg(unix)]
#[test]
fn unreadable_subfolder_is_a_failure_and_the_rest_is_listed() {
    let dir = TempDir::new();
    let mut expected = sample_card(&dir.0);
    write_file(&dir.0, "DCIM/102_FUJI/DSCF0100.RAF", b"raw", at(1));
    let locked = dir.0.join("DCIM/102_FUJI");
    let _mode = Mode::set(&locked, 0o000);
    expected.sort_by(|a, b| a.0.cmp(&b.0));

    let scan = scan_source(&dir.0).unwrap();

    let got: Vec<_> = scan
        .files
        .iter()
        .map(|f| (f.path.clone(), f.media_type, f.size, f.modified))
        .collect();
    assert_eq!(got, expected);
    assert_eq!(scan.failures.len(), 1);
    assert_eq!(scan.failures[0].path, locked);
    assert_eq!(scan.failures[0].source.kind(), std::io::ErrorKind::PermissionDenied);
}

/// A folder that can be listed but not searched: its entries' names are
/// known, their metadata is not. That is an unreadable file.
#[cfg(unix)]
#[test]
fn unreadable_files_are_failures_and_the_rest_is_listed() {
    let dir = TempDir::new();
    write_file(&dir.0, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1));
    write_file(&dir.0, "DCIM/101_FUJI/DSCF0002.RAF", b"raw", at(1));
    write_file(&dir.0, "DCIM/101_FUJI/DSCF0003.JPG", b"jpg", at(1));
    let _mode = Mode::set(&dir.0.join("DCIM/101_FUJI"), 0o444);

    let scan = scan_source(&dir.0).unwrap();

    let files: Vec<_> = scan.files.iter().map(|f| f.path.clone()).collect();
    assert_eq!(files, vec![dir.0.join("DCIM/100_FUJI/DSCF0001.RAF")]);
    let failed: Vec<_> = scan.failures.iter().map(|f| f.path.clone()).collect();
    assert_eq!(
        failed,
        vec![dir.0.join("DCIM/101_FUJI/DSCF0002.RAF"), dir.0.join("DCIM/101_FUJI/DSCF0003.JPG")]
    );
    assert!(scan
        .failures
        .iter()
        .all(|f| f.source.kind() == std::io::ErrorKind::PermissionDenied));
}

#[test]
fn readable_source_has_no_failures() {
    let dir = TempDir::new();
    sample_card(&dir.0);
    assert!(scan_source(&dir.0).unwrap().failures.is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_dcim_is_an_error() {
    let dir = TempDir::new();
    write_file(&dir.0, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1));
    let dcim = dir.0.join("DCIM");
    let _mode = Mode::set(&dcim, 0o000);

    let err = scan_source(&dir.0).unwrap_err();

    assert_eq!(err.path, dcim);
    assert_eq!(err.source.kind(), std::io::ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[test]
fn unreadable_root_is_an_error() {
    let dir = TempDir::new();
    write_file(&dir.0, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1));
    let _mode = Mode::set(&dir.0, 0o000);

    let err = scan_source(&dir.0).unwrap_err();

    assert_eq!(err.path, dir.0.join("DCIM"));
    assert_eq!(err.source.kind(), std::io::ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[test]
fn inv1_scan_with_failures_leaves_source_manifest_unchanged() {
    let dir = TempDir::new();
    sample_card(&dir.0);
    write_file(&dir.0, "DCIM/102_FUJI/DSCF0100.RAF", b"raw", at(1));
    write_file(&dir.0, "DCIM/103_FUJI/DSCF0101.RAF", b"raw", at(1));
    // The manifest needs read access, so it is taken with the locks off.
    let before = manifest(&dir.0);

    {
        let _a = Mode::set(&dir.0.join("DCIM/102_FUJI"), 0o000);
        let _b = Mode::set(&dir.0.join("DCIM/103_FUJI"), 0o444);
        let scan = scan_source(&dir.0).unwrap();
        assert_eq!(scan.failures.len(), 2);
    }

    assert_eq!(manifest(&dir.0), before);
}
