//! Planning a sync: which source files are already in the library, which are
//! extra copies within the session, and which must be copied. Planning only
//! reads; it writes nothing anywhere (INV-9).

use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::hash::{Hash, hash_file};
use crate::library::scan_library;
use crate::source::{ScanError, SourceFile, SourceScan};

/// What a sync of `sources` into a library would do. Every supported source
/// file is in exactly one list (INV-6). Ignored files and scan failures stay
/// in the `SourceScan`s.
#[derive(Debug)]
pub struct SyncPlan {
    /// The library the plan is for, as given to `plan_sync`, which checked
    /// it against the sources (INV-10).
    pub library: PathBuf,
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
pub fn plan_sync(library: &Path, sources: &[SourceScan]) -> Result<SyncPlan, PlanError> {
    check_library(library, sources)?;
    let mut index = scan_library(library).map_err(PlanError::Library)?;

    let supported: Vec<&SourceFile> = sources.iter().flat_map(SourceScan::supported).collect();
    let mut in_session: HashMap<(OsString, u64), usize> = HashMap::new();
    for file in &supported {
        *in_session.entry(key(file)).or_default() += 1;
    }

    let mut plan = SyncPlan {
        library: library.to_path_buf(),
        to_copy: Vec::new(),
        skipped: Vec::new(),
        duplicates: Vec::new(),
        failures: Vec::new(),
        library_failures: std::mem::take(&mut index.failures),
    };
    // None: unreadable, already in library_failures. Cached because several
    // source files can share one candidate.
    let mut library_hashes: HashMap<PathBuf, Option<Hash>> = HashMap::new();
    // Per name and size, the hashes planned so far, with the planned path.
    let mut planned: HashMap<(OsString, u64), Vec<(Hash, PathBuf)>> = HashMap::new();

    for file in supported {
        let key = key(file);
        let candidates = index.candidates(&key.0, key.1);
        if candidates.is_empty() && in_session[&key] == 1 {
            plan.to_copy.push(file.clone());
            continue;
        }
        let hash = match hash_file(&file.path) {
            Ok(hash) => hash,
            Err(source) => {
                plan.failures.push(ScanError { path: file.path.clone(), source });
                continue;
            }
        };
        let seen = planned.entry(key).or_default();
        if let Some((_, of)) = seen.iter().find(|(h, _)| *h == hash) {
            plan.duplicates.push(Duplicate { file: file.clone(), of: of.clone() });
            continue;
        }
        seen.push((hash, file.path.clone()));
        let in_library = candidates.iter().find(|c| {
            let cached = library_hashes.entry((*c).clone()).or_insert_with(|| {
                hash_file(c)
                    .map_err(|source| {
                        plan.library_failures.push(ScanError { path: (*c).clone(), source })
                    })
                    .ok()
            });
            *cached == Some(hash)
        });
        match in_library {
            Some(library_file) => plan
                .skipped
                .push(Skipped { file: file.clone(), library_file: library_file.clone() }),
            None => plan.to_copy.push(file.clone()),
        }
    }
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
