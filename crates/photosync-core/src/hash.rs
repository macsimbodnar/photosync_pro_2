//! Content hashes for already-synced detection (and later the copy check).

use std::fs::File;
use std::io;
use std::path::Path;

pub(crate) use blake3::Hash;

/// The blake3 hash of the file at `path`. Streams the file, so an
/// uncompressed RAF is never held in memory whole.
pub(crate) fn hash_file(path: &Path) -> io::Result<Hash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(File::open(path)?)?;
    Ok(hasher.finalize())
}
