//! Scanning a source: walk its `DCIM` tree and sort files by type. Read-only
//! by construction (INV-1): only `read_dir` and metadata calls touch the
//! source.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The supported file types, per specs Layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MediaType {
    Raw,
    Jpeg,
    Heif,
    Video,
}

impl MediaType {
    /// The type for a file name's extension, case-insensitive. `None` means
    /// the file is ignored.
    pub fn from_path(path: &Path) -> Option<MediaType> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "raf" => Some(MediaType::Raw),
            "jpg" | "jpeg" => Some(MediaType::Jpeg),
            "hif" | "heif" => Some(MediaType::Heif),
            "mov" | "mp4" => Some(MediaType::Video),
            _ => None,
        }
    }

    /// The `<TYPE>` folder name in the library layout.
    pub fn dir_name(self) -> &'static str {
        match self {
            MediaType::Raw => "RAW",
            MediaType::Jpeg => "JPEG",
            MediaType::Heif => "HEIF",
            MediaType::Video => "VIDEO",
        }
    }
}

/// One file found under a source's `DCIM` tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Full path: the source root joined with the path under it.
    pub path: PathBuf,
    /// `None`: not a supported type; the file is ignored and only counted.
    pub media_type: Option<MediaType>,
    pub size: u64,
    /// As the camera wrote it; the capture date is derived from it.
    pub modified: SystemTime,
}

/// Every file under `<root>/DCIM`, sorted by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceScan {
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
}

impl SourceScan {
    pub fn supported(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter().filter(|f| f.media_type.is_some())
    }

    pub fn ignored(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter().filter(|f| f.media_type.is_none())
    }
}

/// An I/O error during a scan, with the path it happened on.
#[derive(Debug)]
pub struct ScanError {
    pub path: PathBuf,
    pub source: io::Error,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot scan {}: {}", self.path.display(), self.source)
    }
}

impl Error for ScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Scans `<root>/DCIM`. `root` is the source volume. Fails on the first
/// unreadable entry, so a scan is never silently partial. A missing `DCIM`
/// is an error, not an empty source.
///
/// Symlinks are not followed, so the walk cannot loop or leave the source;
/// a symlink is listed as an ignored file.
pub fn scan_source(root: &Path) -> Result<SourceScan, ScanError> {
    let mut files = Vec::new();
    walk(&root.join("DCIM"), &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(SourceScan { root: root.to_path_buf(), files })
}

fn walk(dir: &Path, out: &mut Vec<SourceFile>) -> Result<(), ScanError> {
    let err = |path: &Path| {
        let path = path.to_path_buf();
        move |source| ScanError { path, source }
    };
    for entry in fs::read_dir(dir).map_err(err(dir))? {
        let entry = entry.map_err(err(dir))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(err(&path))?;
        // macOS leaves `._*` AppleDouble files on FAT cards; they and any other
        // dot entry are metadata, never photos (DEC-014).
        let hidden = entry.file_name().as_encoded_bytes().starts_with(b".");
        if meta.is_dir() && !hidden {
            walk(&path, out)?;
            continue;
        }
        let media_type =
            if meta.is_file() && !hidden { MediaType::from_path(&path) } else { None };
        out.push(SourceFile {
            media_type,
            size: meta.len(),
            modified: meta.modified().map_err(err(&path))?,
            path,
        });
    }
    Ok(())
}
