//! Scanning the library: every file under the library folder, indexed by
//! name and size. In memory only, rebuilt each sync (INV-9, DEC-003), and
//! read-only like the source scan.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::source::ScanError;

/// Photosync's own copies in flight (INV-3). Not library files: a stale one
/// may be partial, so it must never count as a synced photo.
pub(crate) const TEMP_PREFIX: &[u8] = b".photosync-tmp-";

pub(crate) struct LibraryIndex {
    by_name_size: HashMap<(OsString, u64), Vec<PathBuf>>,
    /// Entries under the library that could not be read. Their files may be
    /// synced already; nothing here can tell.
    pub(crate) failures: Vec<ScanError>,
}

impl LibraryIndex {
    /// Library files with this name and size: the already-synced candidates.
    pub(crate) fn candidates(&self, name: &OsStr, size: u64) -> &[PathBuf] {
        self.by_name_size.get(&(name.to_os_string(), size)).map_or(&[], Vec::as_slice)
    }
}

/// Scans every folder under `root`. Only an unreadable `root` fails the scan;
/// an unreadable entry under it is recorded in `failures`. Symlinks are not
/// followed and are not library files.
pub(crate) fn scan_library(root: &Path) -> Result<LibraryIndex, ScanError> {
    let mut index = LibraryIndex { by_name_size: HashMap::new(), failures: Vec::new() };
    walk(root, &mut index).map_err(|source| ScanError { path: root.to_path_buf(), source })?;
    index.failures.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(index)
}

fn walk(dir: &Path, index: &mut LibraryIndex) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(source) => {
                index.failures.push(ScanError { path, source });
                continue;
            }
        };
        if meta.is_dir() {
            if let Err(source) = walk(&path, index) {
                index.failures.push(ScanError { path, source });
            }
        } else if meta.is_file() && !name.as_encoded_bytes().starts_with(TEMP_PREFIX) {
            index.by_name_size.entry((name, meta.len())).or_default().push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photosync_temp_files_are_not_library_files() {
        let dir = std::env::temp_dir().join(format!("photosync-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("2026-10-28/RAW")).unwrap();
        fs::write(dir.join("2026-10-28/RAW/.photosync-tmp-DSCF0001.RAF"), b"part").unwrap();
        fs::write(dir.join("2026-10-28/RAW/DSCF0002.RAF"), b"full").unwrap();

        let index = scan_library(&dir);
        let _ = fs::remove_dir_all(&dir);
        let index = index.unwrap();

        assert!(index.candidates(OsStr::new(".photosync-tmp-DSCF0001.RAF"), 4).is_empty());
        assert_eq!(index.by_name_size.len(), 1);
        assert_eq!(
            index.candidates(OsStr::new("DSCF0002.RAF"), 4),
            [dir.join("2026-10-28/RAW/DSCF0002.RAF")]
        );
        assert!(index.failures.is_empty());
    }
}
