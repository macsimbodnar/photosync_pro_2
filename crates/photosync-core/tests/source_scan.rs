//! S002: scanning a source's DCIM tree, and INV-1 for the scan.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use photosync_core::{MediaType, scan_source};

/// A fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> TempDir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("photosync-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path); // left over from a killed run
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_file(root: &Path, rel: &str, bytes: &[u8], modified: SystemTime) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    File::options().write(true).open(&path).unwrap().set_modified(modified).unwrap();
    path
}

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

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

/// Every path under `root` (files and folders) with its mtime, and full
/// contents for files. Contents stand in for INV-1's hash: stricter, and
/// needs no hashing crate.
fn manifest(root: &Path) -> BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)> {
    fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let bytes = meta.is_file().then(|| fs::read(&path).unwrap());
            out.insert(path.clone(), (meta.modified().unwrap(), bytes));
            if meta.is_dir() {
                walk(&path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &mut out);
    out
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
