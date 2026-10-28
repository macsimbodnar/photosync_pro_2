//! Content hashes for already-synced detection and the copy check.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub(crate) use blake3::Hash;

use crate::progress::Cancel;

/// Large enough to keep a card reader streaming, small enough to hold. Also
/// the step between progress events and cancel checks.
pub(crate) const CHUNK: usize = 1 << 20;

/// The blake3 hash of the file at `path`. Streams the file, so an
/// uncompressed RAF is never held in memory whole.
pub(crate) fn hash_file(path: &Path) -> io::Result<Hash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(File::open(path)?)?;
    Ok(hasher.finalize())
}

/// `hash_file` one chunk at a time: `read` gets each chunk's length, and
/// `cancel` is checked before each read. `Ok(None)`: cancelled.
pub(crate) fn hash_file_with(
    path: &Path,
    cancel: &Cancel,
    mut read: impl FnMut(u64),
) -> io::Result<Option<Hash>> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0; CHUNK];
    loop {
        if cancel.is_cancelled() {
            return Ok(None);
        }
        let n = match file.read(&mut buf) {
            Ok(0) => return Ok(Some(hasher.finalize())),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        hasher.update(&buf[..n]);
        read(n as u64);
    }
}
