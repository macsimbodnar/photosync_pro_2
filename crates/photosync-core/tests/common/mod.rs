//! Temp-dir and file helpers shared by the integration tests.

// Each test file is its own crate and uses only some of these.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use photosync_core::{SourceScan, scan_source};

#[cfg(unix)]
pub mod images;

/// A device id no real volume has: not a sign-extended 32-bit macOS
/// `dev_t`, and a Linux major number far beyond any real one.
pub const OTHER_DEVICE: u64 = 0xdead_beef_dead_beef;

/// `scan_source(root)`, with the source declared to be on another device
/// than the library. Temp dirs put both on one device, which INV-10 refuses
/// (DEC-025), so every test that is not about the device rule scans with
/// this. The device rule's own tests use `scan_source` as is.
pub fn scan_elsewhere(root: &Path) -> SourceScan {
    let mut scan = scan_source(root).unwrap();
    assert_ne!(scan.device(), OTHER_DEVICE, "a real device has the fake id");
    scan.set_device_for_test(OTHER_DEVICE);
    scan
}

/// `scan_elsewhere`, but on fake device `card` of its own (1, 2, ...).
/// `scan_elsewhere` puts every source on one device, and a twin must be on
/// another device than the file it stands in for (DEC-038), so a test of
/// twins on two cards scans each with its own number.
pub fn scan_card(root: &Path, card: u64) -> SourceScan {
    let mut scan = scan_elsewhere(root);
    scan.set_device_for_test(OTHER_DEVICE + card);
    scan
}

/// A fresh directory under the system temp dir, removed on drop.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> TempDir {
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

pub fn write_file(root: &Path, rel: &str, bytes: &[u8], modified: SystemTime) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    File::options().write(true).open(&path).unwrap().set_modified(modified).unwrap();
    path
}

pub fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

/// The local `YYYY-MM-DD` that `date(1)` prints for `secs` after the epoch:
/// an independent check of the core's time zone handling.
pub fn date_command(secs: u64) -> String {
    let mut cmd = std::process::Command::new("date");
    if cfg!(target_vendor = "apple") {
        cmd.args(["-r", &secs.to_string()]);
    } else {
        cmd.arg(format!("-d@{secs}"));
    }
    let out = cmd.arg("+%Y-%m-%d").output().unwrap();
    assert!(out.status.success(), "date failed: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Every path under `root` (files and folders) with its mtime, and full
/// contents for files. Contents stand in for INV-1's hash: stricter, and
/// needs no hashing crate.
pub fn manifest(root: &Path) -> BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)> {
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

/// Sets `path`'s permission bits for the guard's lifetime. Declared after the
/// `TempDir`, so it drops first and `remove_dir_all` can clean up.
#[cfg(unix)]
pub struct Mode(PathBuf);

#[cfg(unix)]
impl Mode {
    pub fn set(path: &Path, mode: u32) -> Mode {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        Mode(path.to_path_buf())
    }
}

#[cfg(unix)]
impl Drop for Mode {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}
