//! Planning a sync: which source files are already in the library, which are
//! extra copies within the session, and which must be copied. Planning only
//! reads; it writes nothing anywhere (INV-9).

use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::hash::{Hash, hash_file_with};
use crate::library::scan_library;
use crate::progress::{Monitor, Phase, Tracker};
use crate::source::{ScanError, SourceFile, SourceScan};

/// What a sync of `sources` into a library would do. Every supported source
/// file is in exactly one list (INV-6). Ignored files and scan failures stay
/// in the `SourceScan`s.
#[derive(Debug)]
pub struct SyncPlan {
    /// The library the plan is for, as given to `plan_sync`, which checked
    /// it against the sources (INV-10).
    pub library: PathBuf,
    /// The roots of the planned sources, in the order given. Progress is
    /// reported per source in this order.
    pub sources: Vec<PathBuf>,
    /// New to the library, in source order, then path order.
    pub to_copy: Vec<SourceFile>,
    /// Already synced: same name, size and hash as a library file.
    pub skipped: Vec<Skipped>,
    /// Same content as a file earlier in the session, so it is planned once.
    pub duplicates: Vec<Duplicate>,
    /// Source files that could not be read while hashing. Not planned.
    pub failures: Vec<ScanError>,
    /// Library entries that could not be read or hashed. A source file whose
    /// only candidates are here is planned for copy: a second copy in the
    /// library is safe, a photo left uncopied is not.
    pub library_failures: Vec<ScanError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub file: SourceFile,
    /// The library file with the same content.
    pub library_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicate {
    pub file: SourceFile,
    /// The path of the planned (or skipped) file it duplicates.
    pub of: PathBuf,
}

/// Why a sync cannot be planned at all.
#[derive(Debug)]
pub enum PlanError {
    /// INV-10: the library is a source root or under one.
    LibraryOnSource { library: PathBuf, source_root: PathBuf },
    /// INV-10: the library is inside a `DCIM` tree.
    LibraryInDcim { library: PathBuf },
    /// INV-10: a source root is under the library.
    LibraryContainsSource { library: PathBuf, source_root: PathBuf },
    /// The library folder itself cannot be read (or does not exist).
    Library(ScanError),
    /// A source root cannot be resolved, e.g. the card was removed.
    Source(ScanError),
    /// The monitor's cancel was set. Nothing was written.
    Cancelled,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::LibraryOnSource { library, source_root } => write!(
                f,
                "library {} is on source {}",
                library.display(),
                source_root.display()
            ),
            PlanError::LibraryInDcim { library } => {
                write!(f, "library {} is inside a DCIM folder", library.display())
            }
            PlanError::LibraryContainsSource { library, source_root } => write!(
                f,
                "library {} contains source {}",
                library.display(),
                source_root.display()
            ),
            PlanError::Library(e) | PlanError::Source(e) => e.fmt(f),
            PlanError::Cancelled => write!(f, "planning cancelled"),
        }
    }
}

impl Error for PlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PlanError::Library(e) | PlanError::Source(e) => Some(e),
            _ => None,
        }
    }
}

/// Plans a sync of the supported files of `sources` into `library`, an
/// existing folder. Refuses a library that could overlap a source (INV-10)
/// before reading anything else.
///
/// Only name + size candidates are hashed: files with a candidate in the
/// library, or a same name and size file elsewhere in the session. A file
/// unique by name and size is planned without being read.
///
/// Progress counts the source files to hash (`Phase::Planning`). Library
/// candidates are hashed too, uncounted, while their source file is
/// current. A cancel returns `PlanError::Cancelled`; planning never writes,
/// so nothing is left behind.
pub fn plan_sync(
    library: &Path,
    sources: &[SourceScan],
    monitor: &mut Monitor,
) -> Result<SyncPlan, PlanError> {
    check_library(library, sources)?;
    let mut index = scan_library(library).map_err(PlanError::Library)?;
    let library_failures = std::mem::take(&mut index.failures);

    let supported: Vec<(usize, &SourceFile)> = sources
        .iter()
        .enumerate()
        .flat_map(|(i, scan)| scan.supported().map(move |f| (i, f)))
        .collect();
    let mut in_session: HashMap<(OsString, u64), usize> = HashMap::new();
    for (_, file) in &supported {
        *in_session.entry(key(file)).or_default() += 1;
    }
    let to_hash = |file: &SourceFile| {
        let key = key(file);
        !index.candidates(&key.0, key.1).is_empty() || in_session[&key] > 1
    };

    let roots: Vec<PathBuf> = sources.iter().map(|s| s.root.clone()).collect();
    let hashed = supported.iter().filter(|(_, f)| to_hash(f)).map(|(i, f)| (Some(*i), f.size));
    let mut tracker = Tracker::start(monitor, Phase::Planning, &roots, hashed);
    let cancel = tracker.cancel();

    let mut plan = SyncPlan {
        library: library.to_path_buf(),
        sources: roots.clone(),
        to_copy: Vec::new(),
        skipped: Vec::new(),
        duplicates: Vec::new(),
        failures: Vec::new(),
        library_failures,
    };
    // None: unreadable, already in library_failures. Cached because several
    // source files can share one candidate.
    let mut library_hashes: HashMap<PathBuf, Option<Hash>> = HashMap::new();
    // Per name and size, the hashes planned so far, with the planned path.
    let mut planned: HashMap<(OsString, u64), Vec<(Hash, PathBuf)>> = HashMap::new();

    for (source, file) in supported {
        if cancel.is_cancelled() {
            tracker.finish(true);
            return Err(PlanError::Cancelled);
        }
        if !to_hash(file) {
            plan.to_copy.push(file.clone());
            continue;
        }
        let key = key(file);
        let candidates = index.candidates(&key.0, key.1);
        tracker.file_started(&file.path, Some(source), file.size);
        let hash = match hash_file_with(&file.path, &cancel, |n| tracker.bytes(n)) {
            Ok(Some(hash)) => hash,
            Ok(None) => {
                tracker.finish(true);
                return Err(PlanError::Cancelled);
            }
            Err(source) => {
                plan.failures.push(ScanError { path: file.path.clone(), source });
                tracker.file_done(&file.path);
                continue;
            }
        };
        let seen = planned.entry(key).or_default();
        if let Some((_, of)) = seen.iter().find(|(h, _)| *h == hash) {
            plan.duplicates.push(Duplicate { file: file.clone(), of: of.clone() });
            tracker.file_done(&file.path);
            continue;
        }
        seen.push((hash, file.path.clone()));
        let mut in_library = None;
        for candidate in candidates {
            let cached = match library_hashes.get(candidate) {
                Some(cached) => *cached,
                None => {
                    let cached = match hash_file_with(candidate, &cancel, |_| {}) {
                        Ok(Some(hash)) => Some(hash),
                        Ok(None) => {
                            tracker.finish(true);
                            return Err(PlanError::Cancelled);
                        }
                        Err(source) => {
                            plan.library_failures
                                .push(ScanError { path: candidate.clone(), source });
                            None
                        }
                    };
                    library_hashes.insert(candidate.clone(), cached);
                    cached
                }
            };
            if cached == Some(hash) {
                in_library = Some(candidate);
                break;
            }
        }
        match in_library {
            Some(library_file) => plan
                .skipped
                .push(Skipped { file: file.clone(), library_file: library_file.clone() }),
            None => plan.to_copy.push(file.clone()),
        }
        tracker.file_done(&file.path);
    }
    tracker.finish(false);
    Ok(plan)
}

fn key(file: &SourceFile) -> (OsString, u64) {
    (file.path.file_name().unwrap_or_default().to_os_string(), file.size)
}

/// INV-10, on resolved paths, so a symlink or `..` cannot hide an overlap.
/// A source root is its volume's mount point, so "under the root" is "on the
/// source volume".
fn check_library(library: &Path, sources: &[SourceScan]) -> Result<(), PlanError> {
    let resolved = fs::canonicalize(library)
        .map_err(|source| PlanError::Library(ScanError { path: library.to_path_buf(), source }))?;
    // Cards are FAT or exFAT, which ignore case, so `dcim` is a DCIM too.
    let in_dcim = resolved
        .components()
        .any(|c| matches!(c, Component::Normal(n) if n.eq_ignore_ascii_case("DCIM")));
    if in_dcim {
        return Err(PlanError::LibraryInDcim { library: library.to_path_buf() });
    }
    for scan in sources {
        let root = fs::canonicalize(&scan.root)
            .map_err(|source| PlanError::Source(ScanError { path: scan.root.clone(), source }))?;
        if resolved.starts_with(&root) {
            return Err(PlanError::LibraryOnSource {
                library: library.to_path_buf(),
                source_root: scan.root.clone(),
            });
        }
        if root.starts_with(&resolved) {
            return Err(PlanError::LibraryContainsSource {
                library: library.to_path_buf(),
                source_root: scan.root.clone(),
            });
        }
    }
    Ok(())
}
