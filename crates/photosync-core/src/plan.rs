//! Planning a sync: which source files are already in the library, which are
//! extra copies within the session, and which must be copied. Planning only
//! reads; it writes nothing anywhere (INV-9).

use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::hash::{Hash, hash_file_with};
use crate::library::scan_library;
use crate::progress::{Monitor, Phase, Tracker};
use crate::source::{ScanError, SourceFile, SourceScan};
use crate::volume::{device_id, volume_root};

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
    pub to_copy: Vec<Planned>,
    /// Already synced: same name, size and hash as a library file.
    pub skipped: Vec<Skipped>,
    /// Same content as a file earlier in this plan, so it is planned once.
    pub duplicates: Vec<Duplicate>,
    /// Same content as a file an earlier sync of the session copied or
    /// found on the drive (`of`, on an earlier card): "duplicate in
    /// session", not "already on drive". Only a `Session` fills it.
    pub seen_earlier: Vec<Duplicate>,
    /// Source files that could not be read while hashing. Not planned.
    pub failures: Vec<ScanError>,
    /// Library entries that could not be read or hashed. A source file whose
    /// only candidates are here is planned for copy: a second copy in the
    /// library is safe, a photo left uncopied is not.
    pub library_failures: Vec<ScanError>,
    /// Where INV-10 found the library: its device and its volume (`None`:
    /// the boot disk). `copy_plan` checks the library is still there before
    /// it writes. Crate-only, as `source_devices`, so a plan can only come
    /// from `plan_sync` and its record from the checks (DEC-027).
    pub(crate) library_device: u64,
    pub(crate) library_volume: Option<PathBuf>,
    /// The device of each source, in the order of `sources`.
    pub(crate) source_devices: Vec<u64>,
}

/// A file to copy, with what planning learnt about it. The extras are
/// crate-only, so they come from planning and nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub file: SourceFile,
    /// Its scan's index in `SyncPlan::sources`. Progress counts the file
    /// there, whatever other root its path is under.
    pub(crate) source: usize,
    /// Its content, if planning read it (it had a name + size candidate).
    /// The copy must read the same, so a file changed since cannot vouch
    /// for its duplicates.
    pub(crate) hash: Option<Hash>,
    /// The plan's duplicates of it, in plan order: the same content on
    /// other sources, which the copy reads instead, with no new hashing,
    /// when this file cannot be read (F07).
    pub(crate) twins: Vec<SourceFile>,
}

impl Planned {
    pub fn source(&self) -> usize {
        self.source
    }

    /// Whether planning read it, so its copy is checked against that hash.
    pub fn is_hashed(&self) -> bool {
        self.hash.is_some()
    }

    pub fn twins(&self) -> &[SourceFile] {
        &self.twins
    }
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
    /// INV-10: the library is on the same device (`st_dev`) as a source,
    /// under another path: a second mount of the card, or a folder on the
    /// disk the source folder is on (DEC-025).
    LibraryOnSourceDevice { library: PathBuf, source_root: PathBuf },
    /// INV-10: the library's volume has a `DCIM` folder at its top: it is a
    /// card, a source or not (DEC-025).
    LibraryOnCard { library: PathBuf, volume: PathBuf },
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
            PlanError::LibraryOnSourceDevice { library, source_root } => write!(
                f,
                "library {} is on the same device as source {}",
                library.display(),
                source_root.display()
            ),
            PlanError::LibraryOnCard { library, volume } => write!(
                f,
                "library {} is on card {}, which has a DCIM folder",
                library.display(),
                volume.display()
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
    plan_in_session(library, sources, monitor, &HashMap::new())
}

/// `plan_sync` within a session: `earlier` maps each library file an
/// earlier sync of the session copied or found to the session file it
/// holds. A file matching one is a duplicate in session of that file.
pub(crate) fn plan_in_session(
    library: &Path,
    sources: &[SourceScan],
    monitor: &mut Monitor,
    earlier: &HashMap<PathBuf, PathBuf>,
) -> Result<SyncPlan, PlanError> {
    let (library_device, library_volume) = check_library(library, sources)?;
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
        seen_earlier: Vec::new(),
        failures: Vec::new(),
        library_failures,
        library_device,
        library_volume,
        source_devices: sources.iter().map(SourceScan::device).collect(),
    };
    // None: unreadable, already in library_failures. Cached because several
    // source files can share one candidate.
    let mut library_hashes: HashMap<PathBuf, Option<Hash>> = HashMap::new();
    // Per name and size, the hashes planned so far.
    let mut planned: HashMap<(OsString, u64), Vec<Seen>> = HashMap::new();

    for (source, file) in supported {
        if cancel.is_cancelled() {
            tracker.finish(true);
            return Err(PlanError::Cancelled);
        }
        if !to_hash(file) {
            plan.to_copy.push(Planned { file: file.clone(), source, hash: None, twins: Vec::new() });
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
        if let Some((_, of, copied)) = seen.iter().find(|(h, _, _)| *h == hash) {
            // A twin stands in for an unreadable card, so one on that same
            // card is none (INV-3): its read error would likely hit it too.
            // By device, not source index: two sources can be one card
            // (DEC-038).
            let other_card = |i: &usize| {
                sources[plan.to_copy[*i].source].device() != sources[source].device()
            };
            if let Some(i) = copied.filter(other_card) {
                plan.to_copy[i].twins.push(file.clone());
            }
            plan.duplicates.push(Duplicate { file: file.clone(), of: of.clone() });
            tracker.file_done(&file.path);
            continue;
        }
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
        let copied = match in_library {
            Some(library_file) => {
                // The same card synced again holds the very files that
                // put these photos there: already on drive, not twins.
                match earlier.get(library_file).filter(|of| **of != file.path) {
                    Some(of) => plan.seen_earlier.push(Duplicate { file: file.clone(), of: of.clone() }),
                    None => plan
                        .skipped
                        .push(Skipped { file: file.clone(), library_file: library_file.clone() }),
                }
                None
            }
            None => {
                plan.to_copy.push(Planned { file: file.clone(), source, hash: Some(hash), twins: Vec::new() });
                Some(plan.to_copy.len() - 1)
            }
        };
        seen.push((hash, file.path.clone(), copied));
        tracker.file_done(&file.path);
    }
    tracker.finish(false);
    Ok(plan)
}

/// A file planning has read: its hash, its path and, if it is to be
/// copied, its index in `to_copy`.
type Seen = (Hash, PathBuf, Option<usize>);

fn key(file: &SourceFile) -> (OsString, u64) {
    (file.path.file_name().unwrap_or_default().to_os_string(), file.size)
}

/// INV-10, on resolved paths, so a symlink or `..` cannot hide an overlap.
/// The path checks name the plain overlaps. The device check catches what
/// paths cannot: a second mount or a bind mount of the card, or another
/// folder on the disk a source folder is on, has an unrelated path but the
/// same device (DEC-025). Returns the library's device and volume, as
/// checked.
fn check_library(
    library: &Path,
    sources: &[SourceScan],
) -> Result<(u64, Option<PathBuf>), PlanError> {
    check_library_with(library, sources, volume_root)
}

/// `check_library` with the volume lookup injected, so unit tests can make
/// a temp folder a card's volume.
fn check_library_with(
    library: &Path,
    sources: &[SourceScan],
    volume_root: impl Fn(&Path) -> io::Result<Option<PathBuf>>,
) -> Result<(u64, Option<PathBuf>), PlanError> {
    let resolved = fs::canonicalize(library)
        .map_err(|source| PlanError::Library(ScanError { path: library.to_path_buf(), source }))?;
    // Cards are FAT or exFAT, which ignore case, so `dcim` is a DCIM too.
    let in_dcim = resolved
        .components()
        .any(|c| matches!(c, Component::Normal(n) if n.eq_ignore_ascii_case("DCIM")));
    if in_dcim {
        return Err(PlanError::LibraryInDcim { library: library.to_path_buf() });
    }
    let library_device = fs::metadata(&resolved)
        .and_then(|meta| device_id(&meta))
        .map_err(|source| PlanError::Library(ScanError { path: library.to_path_buf(), source }))?;
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
        if scan.device() == library_device {
            return Err(PlanError::LibraryOnSourceDevice {
                library: library.to_path_buf(),
                source_root: scan.root.clone(),
            });
        }
    }
    // A card that is not a source: nothing above knows it, only its `DCIM`.
    let volume = volume_root(&resolved)
        .map_err(|source| PlanError::Library(ScanError { path: library.to_path_buf(), source }))?;
    if let Some(volume) = &volume {
        // Followed, as discovery and `scan_source` follow it.
        let dcim = volume.join("DCIM");
        match fs::metadata(&dcim) {
            Ok(meta) if meta.is_dir() => {
                return Err(PlanError::LibraryOnCard {
                    library: library.to_path_buf(),
                    volume: volume.clone(),
                });
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            // It may be a card: fail closed.
            Err(source) => return Err(PlanError::Library(ScanError { path: dcim, source })),
        }
    }
    Ok((library_device, volume))
}

#[cfg(test)]
mod tests {
    use crate::common::{TempDir, at, scan_elsewhere, write_file};
    use super::*;

    fn card(root: &Path) -> PathBuf {
        write_file(root, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1_700_000_000));
        fs::canonicalize(root).unwrap()
    }

    /// The review's case: a card that is not a source, with the library on
    /// it or at its top. Only its `DCIM` tells it is a card (DEC-025).
    #[test]
    fn inv10_library_on_a_card_volume_is_refused_even_if_it_is_not_a_source() {
        let dir = TempDir::new();
        let other = card(&dir.0.join("OTHER"));
        let vol = card(&dir.0.join("CARD"));
        fs::create_dir(vol.join("Photos")).unwrap();
        let sources = [scan_elsewhere(&other)];

        for library in [vol.join("Photos"), vol.clone()] {
            match check_library_with(&library, &sources, |_| Ok(Some(vol.clone()))) {
                Err(PlanError::LibraryOnCard { library: l, volume }) => {
                    assert_eq!((l, volume), (library, vol.clone()));
                }
                other => panic!("expected LibraryOnCard, got {other:?}"),
            }
        }
    }

    /// A `DCIM` deeper in the volume (a source folder) or a file named
    /// `DCIM` at its top does not make a card; the boot disk (`None`) is
    /// never one.
    #[test]
    fn inv10_library_on_a_volume_without_a_dcim_folder_at_its_top_is_planned() {
        let dir = TempDir::new();
        let other = card(&dir.0.join("OTHER"));
        let drive = card(&dir.0.join("DRIVE/backup"));
        let drive = drive.parent().unwrap().to_path_buf();
        let odd = dir.0.join("ODD");
        write_file(&odd, "DCIM", b"x", at(1_700_000_000));
        let odd = fs::canonicalize(odd).unwrap();
        fs::create_dir(drive.join("Photos")).unwrap();
        fs::create_dir(odd.join("Photos")).unwrap();
        let sources = [scan_elsewhere(&other)];

        for vol in [drive, odd] {
            let library = vol.join("Photos");
            check_library_with(&library, &sources, |_| Ok(Some(vol.clone()))).unwrap();
        }
        check_library_with(&dir.0.join("DRIVE/Photos"), &sources, |_| Ok(None)).unwrap();
    }

    /// A volume whose top cannot be read may be a card: refused, as
    /// discovery never offers it (DEC-024). Needs Unix permission bits:
    /// skipped by `scripts/test_exfat.sh`.
    #[cfg(unix)]
    #[test]
    fn inv10_library_whose_volume_top_cannot_be_checked_is_refused() {
        use crate::common::Mode;
        let dir = TempDir::new();
        let other = card(&dir.0.join("OTHER"));
        let library = dir.0.join("lib");
        fs::create_dir(&library).unwrap();
        let shut = dir.0.join("SHUT");
        fs::create_dir(&shut).unwrap();
        let _mode = Mode::set(&shut, 0o000);

        match check_library_with(&library, &[scan_elsewhere(&other)], |_| Ok(Some(shut.clone()))) {
            Err(PlanError::Library(e)) => {
                assert_eq!(e.path, shut.join("DCIM"));
                assert_eq!(e.source.kind(), io::ErrorKind::PermissionDenied);
            }
            other => panic!("expected Library, got {other:?}"),
        }
    }

    #[test]
    fn inv10_library_whose_volume_cannot_be_found_is_refused() {
        let dir = TempDir::new();
        let other = card(&dir.0.join("OTHER"));
        let library = dir.0.join("lib");
        fs::create_dir(&library).unwrap();
        let gone = |_: &Path| Err(io::Error::other("unplugged"));

        match check_library_with(&library, &[scan_elsewhere(&other)], gone) {
            Err(PlanError::Library(e)) => assert_eq!(e.path, library),
            other => panic!("expected Library, got {other:?}"),
        }
    }
}
