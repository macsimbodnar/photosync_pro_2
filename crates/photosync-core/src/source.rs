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

/// Every readable file under `<root>/DCIM`, and every entry that could not
/// be read, both sorted by path.
#[derive(Debug)]
pub struct SourceScan {
    pub root: PathBuf,
    /// `root` with symlinks, `.` and `..` resolved, taken while the card was
    /// there, so two spellings of one source compare equal even after it is
    /// removed.
    pub resolved_root: PathBuf,
    pub files: Vec<SourceFile>,
    /// Files and folders under `DCIM` that could not be read. A folder whose
    /// listing broke partway keeps the files it gave before the break in
    /// `files`; the rest of its contents are unknown and in neither list.
    /// Only metadata is read here: a file whose data is unreadable shows up
    /// later, when it is hashed or copied.
    pub failures: Vec<ScanError>,
    /// The paths in `failures` whose contents are unknown, sorted: a folder
    /// whose listing failed, or an entry whose type could not be read, as it
    /// may be a folder. What such a folder holds is in neither list.
    pub failed_folders: Vec<PathBuf>,
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

/// Scans `<root>/DCIM`. `root` is the source volume. Only an unreadable
/// `DCIM` (or a missing one, which is not an empty source) fails the scan.
/// An unreadable entry under it is recorded in `failures` and the walk goes
/// on, so one bad folder does not hide the rest of the card.
///
/// Symlinks are not followed, so the walk cannot loop or leave the source;
/// a symlink is listed as an ignored file.
pub fn scan_source(root: &Path) -> Result<SourceScan, ScanError> {
    let dcim = root.join("DCIM");
    let mut files = Vec::new();
    let mut failures = Failures::default();
    walk(&dcim, &mut files, &mut failures).map_err(|source| ScanError { path: dcim, source })?;
    let resolved_root = fs::canonicalize(root)
        .map_err(|source| ScanError { path: root.to_path_buf(), source })?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let Failures { mut errors, mut folders } = failures;
    errors.sort_by(|a, b| a.path.cmp(&b.path));
    folders.sort();
    Ok(SourceScan {
        root: root.to_path_buf(),
        resolved_root,
        files,
        failures: errors,
        failed_folders: folders,
    })
}

#[derive(Default)]
struct Failures {
    errors: Vec<ScanError>,
    folders: Vec<PathBuf>,
}

impl Failures {
    fn push(&mut self, path: PathBuf, source: io::Error, folder: bool) {
        if folder {
            self.folders.push(path.clone());
        }
        self.errors.push(ScanError { path, source });
    }
}

/// Walks `dir`. An `Err` means `dir`'s own listing failed; the caller
/// decides whether that fails the scan or is one more failure.
fn walk(dir: &Path, files: &mut Vec<SourceFile>, failures: &mut Failures) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        // A broken listing cannot be trusted to go on; what it gave so far stays.
        let entry = entry?;
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(source) => {
                // The listing usually gives the type without a stat; with no
                // type, it may be a folder, so its contents count as unknown.
                let folder = entry.file_type().map_or(true, |t| t.is_dir());
                failures.push(path, source, folder);
                continue;
            }
        };
        // macOS leaves `._*` AppleDouble files on FAT cards; they and any other
        // dot entry are metadata, never photos (DEC-014).
        let hidden = entry.file_name().as_encoded_bytes().starts_with(b".");
        if meta.is_dir() && !hidden {
            if let Err(source) = walk(&path, files, failures) {
                failures.push(path, source, true);
            }
            continue;
        }
        let modified = match meta.modified() {
            Ok(modified) => modified,
            Err(source) => {
                failures.push(path, source, false);
                continue;
            }
        };
        let media_type =
            if meta.is_file() && !hidden { MediaType::from_path(&path) } else { None };
        files.push(SourceFile { media_type, size: meta.len(), modified, path });
    }
    Ok(())
}
