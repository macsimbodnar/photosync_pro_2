//! Finding the volumes a front-end offers: mounted volumes with a `DCIM`
//! folder are sources, the other mounted volumes the user can write to are
//! destinations. Read-only: only listings and metadata calls (INV-1).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::source::ScanError;

/// The candidate volumes under the search roots. Each list is sorted by
/// path; a volume is in at most one of them.
#[derive(Debug, Default)]
pub struct Volumes {
    /// Mounted volumes holding a `DCIM` folder.
    pub sources: Vec<PathBuf>,
    /// The other mounted volumes the current user can write to.
    pub destinations: Vec<PathBuf>,
    /// Search roots and entries that could not be read. A volume whose
    /// `DCIM` could not be checked is here only: it may be a card, so it is
    /// not offered as a destination.
    pub failures: Vec<ScanError>,
}

/// Where this platform mounts removable volumes: `/Volumes` on macOS;
/// `/media/$USER` and `/run/media/$USER` on Linux, as distributions differ.
/// Fails if Linux has no `$USER`, or on any other platform.
pub fn default_search_roots() -> io::Result<Vec<PathBuf>> {
    #[cfg(target_os = "macos")]
    return Ok(vec![PathBuf::from("/Volumes")]);
    #[cfg(target_os = "linux")]
    {
        let user = std::env::var_os("USER")
            .filter(|u| !u.is_empty())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "USER is not set"))?;
        Ok(vec![Path::new("/media").join(&user), Path::new("/run/media").join(&user)])
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Err(io::Error::new(io::ErrorKind::Unsupported, "no volume search roots for this platform"))
}

/// Lists the volumes mounted directly under `roots` (see
/// `default_search_roots`). A root that does not exist holds no volumes;
/// Linux usually has only one of its two.
///
/// Only an entry that is a folder and a mount point is a volume. So the
/// macOS boot volume, which `/Volumes` shows as a symlink to `/`, is never
/// offered, and neither is a folder a volume left behind when it was not
/// cleanly ejected: writing there would fill the boot disk. Dot entries
/// such as `/Volumes/.timemachine` are OS internals and are skipped too
/// (DEC-014).
pub fn discover_volumes(roots: &[PathBuf]) -> Volumes {
    discover(roots, |path| Ok(mount_device(path)?.is_some()))
}

/// `discover_volumes` with the mount point test injected, so unit tests can
/// treat plain temp folders as volumes.
fn discover(roots: &[PathBuf], mounted: impl Fn(&Path) -> io::Result<bool>) -> Volumes {
    let mut sources = Vec::new();
    let mut destinations = Vec::new();
    let mut failures = Vec::new();
    let mut fail = |path: PathBuf, source| failures.push(ScanError { path, source });
    for root in roots {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => {
                fail(root.clone(), source);
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(source) => {
                    // A broken listing cannot be trusted to go on.
                    fail(root.clone(), source);
                    break;
                }
            };
            if entry.file_name().as_encoded_bytes().starts_with(b".") {
                continue;
            }
            let path = entry.path();
            // Not followed: a symlink is never a mount, whatever it points to.
            match fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => continue,
                Err(source) => {
                    fail(path, source);
                    continue;
                }
            }
            match mounted(&path) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(source) => {
                    fail(path, source);
                    continue;
                }
            }
            // Followed, as `scan_source` follows it.
            let dcim = path.join("DCIM");
            match fs::metadata(&dcim) {
                Ok(meta) if meta.is_dir() => {
                    sources.push(path);
                    continue;
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(source) => {
                    fail(dcim, source);
                    continue;
                }
            }
            match writable(&path) {
                Ok(true) => destinations.push(path),
                Ok(false) => {}
                Err(source) => fail(path, source),
            }
        }
    }
    sources.sort();
    destinations.sort();
    failures.sort_by(|a: &ScanError, b| a.path.cmp(&b.path));
    Volumes { sources, destinations, failures }
}

/// The device id (`st_dev`) of `path` if it is a mount point, else `None`.
/// A mount point is on another device than its parent, or is `/`. Symlinks
/// in `path` are followed.
pub(crate) fn mount_device(path: &Path) -> io::Result<Option<u64>> {
    let own = fs::metadata(path)?;
    let parent = fs::metadata(path.join(".."))?;
    let (dev, parent_dev) = (device_id(&own)?, device_id(&parent)?);
    Ok((dev != parent_dev || same_inode(&own, &parent)).then_some(dev))
}

/// The mount point of the volume that holds `path`, an absolute path with
/// symlinks resolved: its topmost folder on the same device. `None` when
/// that is `/`: the boot disk is never a card (DEC-026).
pub(crate) fn volume_root(path: &Path) -> io::Result<Option<PathBuf>> {
    volume_root_with(path, |p| device_id(&fs::metadata(p)?))
}

/// `volume_root` with the device lookup injected, so unit tests can place
/// temp folders on fake devices.
fn volume_root_with(
    path: &Path,
    device: impl Fn(&Path) -> io::Result<u64>,
) -> io::Result<Option<PathBuf>> {
    let own = device(path)?;
    let mut top = path;
    while let Some(parent) = top.parent() {
        if device(parent)? != own {
            return Ok(Some(top.to_path_buf()));
        }
        top = parent;
    }
    Ok(None)
}

#[cfg(unix)]
pub(crate) fn device_id(meta: &fs::Metadata) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(meta.dev())
}

#[cfg(not(unix))]
pub(crate) fn device_id(_: &fs::Metadata) -> io::Result<u64> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "no device ids on this platform"))
}

#[cfg(unix)]
pub(crate) fn same_inode(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.ino() == b.ino()
}

#[cfg(not(unix))]
pub(crate) fn same_inode(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    false
}

/// Whether the current user may create files in `path`: checked with the
/// effective ids, the ones a copy writes with. A read-only mount (a card's
/// lock switch, a macOS NTFS drive) and a folder without write permission
/// are both `false`.
#[cfg(unix)]
fn writable(path: &Path) -> io::Result<bool> {
    let c = crate::copy::c_path(path)?;
    // SAFETY: `c` is a NUL-terminated string that outlives the call.
    let rc =
        unsafe { libc::faccessat(libc::AT_FDCWD, c.as_ptr(), libc::W_OK, libc::AT_EACCESS) };
    if rc == 0 {
        return Ok(true);
    }
    let err = io::Error::last_os_error();
    match err.raw_os_error() {
        Some(libc::EACCES | libc::EROFS | libc::EPERM) => Ok(false),
        _ => Err(err),
    }
}

#[cfg(not(unix))]
fn writable(_: &Path) -> io::Result<bool> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "no write check on this platform"))
}

#[cfg(test)]
mod tests {
    use crate::common::{Mode, TempDir};
    use super::*;

    /// Every folder counts as mounted, so temp folders stand in for volumes.
    fn all_mounted(roots: &[PathBuf]) -> Volumes {
        discover(roots, |_| Ok(true))
    }

    fn card(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(path.join("DCIM/100_FUJI")).unwrap();
        path
    }

    fn drive(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn volumes_with_dcim_are_sources_and_the_rest_are_destinations() {
        let dir = TempDir::new();
        let media = drive(&dir.0, "media");
        let run_media = drive(&dir.0, "run_media");
        let a = card(&media, "CARD_A");
        let b = card(&run_media, "CARD_B");
        let ssd = drive(&media, "SSD");
        fs::write(media.join("not_a_volume.txt"), b"x").unwrap();

        let found = all_mounted(&[media, run_media]);

        assert_eq!(found.sources, [a, b]);
        assert_eq!(found.destinations, [ssd]);
        assert!(found.failures.is_empty(), "{:?}", found.failures);
    }

    #[test]
    fn missing_search_root_holds_no_volumes() {
        let dir = TempDir::new();
        let media = drive(&dir.0, "media");
        let a = card(&media, "CARD");

        let found = all_mounted(&[dir.0.join("absent"), media]);

        assert_eq!(found.sources, [a]);
        assert!(found.destinations.is_empty());
        assert!(found.failures.is_empty(), "{:?}", found.failures);
    }

    #[test]
    fn search_root_that_is_a_file_is_a_failure_and_other_roots_are_listed() {
        let dir = TempDir::new();
        let file = dir.0.join("file");
        fs::write(&file, b"x").unwrap();
        let media = drive(&dir.0, "media");
        let a = card(&media, "CARD");

        let found = all_mounted(&[file.clone(), media]);

        assert_eq!(found.sources, [a]);
        let failed: Vec<&Path> = found.failures.iter().map(|f| f.path.as_path()).collect();
        assert_eq!(failed, [file.as_path()]);
    }

    /// On macOS the boot volume sits in `/Volumes` as `Macintosh HD -> /`.
    /// Even with a `DCIM` behind the link, and with every folder counted as
    /// mounted, it is neither a source nor a destination.
    #[cfg(unix)]
    #[test]
    fn boot_volume_symlink_is_never_offered() {
        let dir = TempDir::new();
        let volumes = drive(&dir.0, "Volumes");
        let boot = card(&dir.0, "boot");
        let plain = drive(&dir.0, "plain");
        std::os::unix::fs::symlink(&boot, volumes.join("Macintosh HD")).unwrap();
        std::os::unix::fs::symlink(&plain, volumes.join("Other HD")).unwrap();
        std::os::unix::fs::symlink("/", volumes.join("Root")).unwrap();

        let found = all_mounted(&[volumes]);

        assert!(found.sources.is_empty(), "{:?}", found.sources);
        assert!(found.destinations.is_empty(), "{:?}", found.destinations);
        assert!(found.failures.is_empty(), "{:?}", found.failures);
    }

    #[test]
    fn dot_entries_are_skipped() {
        let dir = TempDir::new();
        let volumes = drive(&dir.0, "Volumes");
        card(&volumes, ".hidden_card");
        drive(&volumes, ".timemachine");

        let found = all_mounted(&[volumes]);

        assert!(found.sources.is_empty(), "{:?}", found.sources);
        assert!(found.destinations.is_empty(), "{:?}", found.destinations);
    }

    #[test]
    fn dcim_file_is_not_a_source() {
        let dir = TempDir::new();
        let volumes = drive(&dir.0, "Volumes");
        let odd = drive(&volumes, "ODD");
        fs::write(odd.join("DCIM"), b"x").unwrap();

        let found = all_mounted(&[volumes]);

        assert!(found.sources.is_empty());
        assert_eq!(found.destinations, [odd]);
    }

    /// Needs Unix permission bits: skipped by `scripts/test_exfat.sh`.
    #[cfg(unix)]
    #[test]
    fn read_only_volume_is_not_a_destination() {
        let dir = TempDir::new();
        let volumes = drive(&dir.0, "Volumes");
        let locked = drive(&volumes, "LOCKED");
        let open = drive(&volumes, "OPEN");
        let _mode = Mode::set(&locked, 0o555);

        let found = all_mounted(&[volumes]);

        assert_eq!(found.destinations, [open]);
    }

    /// Needs Unix permission bits: skipped by `scripts/test_exfat.sh`.
    #[cfg(unix)]
    #[test]
    fn volume_whose_dcim_cannot_be_checked_is_a_failure_not_a_destination() {
        let dir = TempDir::new();
        let volumes = drive(&dir.0, "Volumes");
        let shut = drive(&volumes, "SHUT");
        let _mode = Mode::set(&shut, 0o000);

        let found = all_mounted(&[volumes]);

        assert!(found.sources.is_empty());
        assert!(found.destinations.is_empty(), "{:?}", found.destinations);
        let failed: Vec<&Path> = found.failures.iter().map(|f| f.path.as_path()).collect();
        assert_eq!(failed, [shut.join("DCIM").as_path()]);
    }

    /// With the real mount test, folders that are not mount points (here,
    /// all of them: one temp device) are never offered, `DCIM` or not.
    #[test]
    fn folder_that_is_not_a_mount_point_is_not_a_volume() {
        let dir = TempDir::new();
        let volumes = drive(&dir.0, "Volumes");
        card(&volumes, "STALE_CARD");
        drive(&volumes, "STALE_DRIVE");

        let found = discover_volumes(&[volumes]);

        assert!(found.sources.is_empty(), "{:?}", found.sources);
        assert!(found.destinations.is_empty(), "{:?}", found.destinations);
        assert!(found.failures.is_empty(), "{:?}", found.failures);
    }

    #[cfg(unix)]
    #[test]
    fn mount_points_are_told_from_plain_folders() {
        let dir = TempDir::new();
        let plain = drive(&dir.0, "plain");
        let root_dev = device_id(&fs::metadata("/").unwrap()).unwrap();
        let dev_dev = device_id(&fs::metadata("/dev").unwrap()).unwrap();

        // `/` is its own parent; `/dev` is devfs or devtmpfs on both platforms.
        assert_eq!(mount_device(Path::new("/")).unwrap(), Some(root_dev));
        assert_eq!(mount_device(Path::new("/dev")).unwrap(), Some(dev_dev));
        assert_eq!(mount_device(&plain).unwrap(), None);
    }

    /// Paths under `vol` are on device 2, everything else on device 1.
    fn fake_devices(vol: &Path) -> impl Fn(&Path) -> io::Result<u64> + '_ {
        move |p| Ok(if p.starts_with(vol) { 2 } else { 1 })
    }

    #[test]
    fn volume_root_is_where_the_device_changes() {
        let vol = Path::new("/mnt/CARD");
        let found = |p: &str| volume_root_with(Path::new(p), fake_devices(vol)).unwrap();

        assert_eq!(found("/mnt/CARD/Photos/2026"), Some(vol.to_path_buf()));
        assert_eq!(found("/mnt/CARD"), Some(vol.to_path_buf()));
    }

    /// No device change up to `/`: the library is on the boot disk, which
    /// is never a card, so no `/DCIM` is looked at (DEC-026).
    #[test]
    fn volume_root_on_the_boot_disk_is_none() {
        let found = |p: &str| volume_root_with(Path::new(p), fake_devices(Path::new("/mnt/CARD")));

        assert_eq!(found("/Users/max/Photos").unwrap(), None);
        assert_eq!(found("/mnt").unwrap(), None);
        assert_eq!(found("/").unwrap(), None);
    }

    #[test]
    fn volume_root_fails_if_a_device_cannot_be_read() {
        let err = volume_root_with(Path::new("/mnt/CARD/Photos"), |p| {
            if p == Path::new("/mnt") { Err(io::Error::other("gone")) } else { Ok(2) }
        })
        .unwrap_err();

        assert_eq!(err.to_string(), "gone");
    }

    #[cfg(unix)]
    #[test]
    fn volume_root_on_this_machine() {
        let dir = TempDir::new();
        let dev = fs::canonicalize("/dev").unwrap();

        assert_eq!(volume_root(Path::new("/")).unwrap(), None);
        assert_eq!(volume_root(&dev).unwrap(), Some(dev.clone()));
        // Wherever the temp dir is mounted, its volume holds it.
        let temp = fs::canonicalize(&dir.0).unwrap();
        if let Some(root) = volume_root(&temp).unwrap() {
            assert!(temp.starts_with(&root), "{root:?}");
            let temp_dev = device_id(&fs::metadata(&temp).unwrap()).unwrap();
            assert_eq!(mount_device(&root).unwrap(), Some(temp_dev));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn default_search_root_on_macos_is_volumes() {
        assert_eq!(default_search_roots().unwrap(), [PathBuf::from("/Volumes")]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn default_search_roots_on_linux_are_media_and_run_media_of_the_user() {
        let user = std::env::var_os("USER").unwrap();
        assert_eq!(
            default_search_roots().unwrap(),
            [Path::new("/media").join(&user), Path::new("/run/media").join(&user)]
        );
    }
}
