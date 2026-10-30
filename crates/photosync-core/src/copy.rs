//! Executing a sync plan: the session folder, then for each new file a temp
//! copy, flush, verify and a rename that never replaces (INV-2, INV-3,
//! INV-7). The only files ever deleted are photosync's own temp files.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::hash::{CHUNK, Hash, hash_file};
use crate::plan::{Planned, SyncPlan};
use crate::progress::{Cancel, Monitor, Phase, Tracker};
use crate::source::{ScanError, SourceFile};
use crate::volume::{device_id, same_inode, volume_root};

/// Every temp file photosync writes starts with this. The library scan skips
/// it, and the next sync deletes it.
const TEMP_PREFIX: &str = ".photosync-tmp-";

/// The session's `<date>[ - <name>]` folder in the library, fixed once from
/// the session's first plan (DEC-009). Nothing is created until a file is
/// copied into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFolder {
    library: PathBuf,
    path: PathBuf,
}

impl SyncFolder {
    pub fn library(&self) -> &Path {
        &self.library
    }

    /// `<library>/<date>[ - <name>]`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the folder is there now, as a real folder. It is fixed before
    /// it is made, and may never be made: a report must not name it then.
    pub fn there(&self) -> io::Result<bool> {
        Ok(lookup(&self.path)?.is_some_and(|meta| meta.is_dir()))
    }
}

/// Why the session folder cannot be fixed.
#[derive(Debug)]
pub enum FolderError {
    /// Not one folder name on every drive, or too long for this one
    /// (`check_name`, `check_name_fits`).
    InvalidName(String),
    /// The name fits, but the library's path with that folder in it is over
    /// the system's limit: another name of this length would be refused too.
    PathTooLong { library: PathBuf, name: String },
    /// The library could not be listed, or a file's mtime has no local date.
    Io(ScanError),
}

impl fmt::Display for FolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FolderError::InvalidName(name) => write!(f, "invalid sync name {name:?}"),
            FolderError::PathTooLong { library, name } => {
                write!(f, "path too long for sync name {name:?} in {}", library.display())
            }
            FolderError::Io(e) => e.fmt(f),
        }
    }
}

impl Error for FolderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FolderError::Io(e) => Some(e),
            FolderError::InvalidName(_) | FolderError::PathTooLong { .. } => None,
        }
    }
}

/// Fixes the session folder from the session's first plan. With a `name`, an
/// existing `<any date> - <name>` folder is reused (the earliest, if several),
/// so a re-run after an interruption lands where the first run did.
/// Otherwise `<date>` is the local date of the earliest new file.
///
/// `None`: the plan copies nothing and there is no named folder to reuse.
/// Reads only; creates nothing.
pub fn sync_folder(plan: &SyncPlan, name: Option<&str>) -> Result<Option<SyncFolder>, FolderError> {
    let library = &plan.library;
    if let Some(name) = name {
        check_name(name)?;
        let existing = existing_named(library, name)
            .map_err(|source| FolderError::Io(ScanError { path: library.clone(), source }))?;
        if let Some(folder) = existing {
            return Ok(Some(SyncFolder { library: library.clone(), path: library.join(folder) }));
        }
    }
    let Some(earliest) = plan.to_copy.iter().map(|p| &p.file).min_by_key(|f| f.modified) else {
        return Ok(None);
    };
    let date = local_date(earliest.modified)
        .map_err(|source| FolderError::Io(ScanError { path: earliest.path.clone(), source }))?;
    let folder = match name {
        Some(name) => format!("{date} - {name}"),
        None => date,
    };
    Ok(Some(SyncFolder { library: library.clone(), path: library.join(folder) }))
}

/// The longest sync name, in bytes: `YYYY-MM-DD - <name>` must fit the 255
/// bytes of a folder name, measured on APFS, exFAT, HFS+ and FAT32 (DEC-053).
pub const MAX_NAME_BYTES: usize = 255 - "YYYY-MM-DD - ".len();

/// A sync name must be one folder name on every drive, as a drive moves
/// between computers: not blank, none of the FAT family's characters, no
/// control character or Unicode noncharacter, no ending a drive would drop or
/// change (DEC-053). `/` and NUL are of those too.
pub(crate) fn check_name(name: &str) -> Result<(), FolderError> {
    if name.trim().is_empty()
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || name.chars().any(|c| c.is_control() || is_noncharacter(c))
        || name.ends_with([' ', '.'])
    {
        return Err(FolderError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// APFS refuses these; exFAT and HFS+ on macOS store some under another name.
fn is_noncharacter(c: char) -> bool {
    let c = u32::from(c);
    (0xfdd0..=0xfdef).contains(&c) || c & 0xfffe == 0xfffe
}

/// Whether the name fits a folder name in `library`, for a session to refuse
/// it before any card is read. Over the limit on any drive, or by the
/// filesystem's own count: HFS+ counts a name decomposed, so one under the
/// limit may not fit, and a lookup says so without writing anything. Any other
/// answer says nothing about the name, and each sync checks the library.
pub(crate) fn check_name_fits(library: &Path, name: &str) -> Result<(), FolderError> {
    if name.len() > MAX_NAME_BYTES {
        return Err(FolderError::InvalidName(name.to_string()));
    }
    let too_long = |folder: &str| {
        matches!(
            fs::symlink_metadata(library.join(folder)),
            Err(e) if e.kind() == io::ErrorKind::InvalidFilename
        )
    };
    let folder = format!("0000-00-00 - {name}");
    if !too_long(&folder) {
        return Ok(());
    }
    // The same error comes from a whole path over the system's limit. Plain
    // ASCII of the same length fits every drive, so if that is too long as
    // well, the path is at fault and not this name (DEC-056).
    if too_long(&"0".repeat(folder.len())) {
        return Err(FolderError::PathTooLong {
            library: library.to_path_buf(),
            name: name.to_string(),
        });
    }
    Err(FolderError::InvalidName(name.to_string()))
}

/// The earliest `YYYY-MM-DD - <name>` folder directly in `library`, as the
/// filesystem sees the name: HFS+ and exFAT on macOS list `Tirane\u{308}` for
/// a folder made as `Tiran\u{eb}`, so comparing bytes would miss it. No
/// normalisation table of ours can know each filesystem's rule, so a dated
/// folder is looked up as the user spelt the name (DEC-044).
fn existing_named(library: &Path, name: &str) -> io::Result<Option<String>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(library)? {
        let entry = entry?;
        let Ok(folder) = entry.file_name().into_string() else {
            continue; // not UTF-8, so not a name the user typed
        };
        let Some(date) = folder.get(..10).filter(|d| is_date(d)) else {
            continue;
        };
        let Some(listed_name) = folder.strip_prefix(date).and_then(|f| f.strip_prefix(" - ")) else {
            continue;
        };
        // Same bytes need no lookup: inode numbers on some filesystems are
        // not stable, and they must not be able to split a plain re-run.
        let same = listed_name == name
            || same_entry(&library.join(&folder), &library.join(format!("{date} - {name}")))?;
        if same && entry.file_type()?.is_dir() {
            found.push(folder);
        }
    }
    Ok(found.into_iter().min())
}

/// Whether both paths are one entry. Symlinks are not followed: a symlink is
/// never a session folder (DEC-020).
fn same_entry(a: &Path, b: &Path) -> io::Result<bool> {
    let (Some(a), Some(b)) = (lookup(a)?, lookup(b)?) else {
        return Ok(false);
    };
    Ok(same_inode(&a, &b) && device_id(&a)? == device_id(&b)?)
}

/// `None` when the filesystem says no such entry can be there. Any other
/// error says nothing about the entry, and is not taken for a no.
fn lookup(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(e) if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::InvalidFilename) => Ok(None),
        Err(e) => Err(e),
    }
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
}

/// `YYYY-MM-DD` of `t` in the computer's time zone. std has no time zones
/// and no date crate is approved, so this asks the C library: `localtime_r`
/// reads `TZ` and the system zone the way `date` does.
#[cfg(unix)]
fn local_date(t: SystemTime) -> io::Result<String> {
    let out_of_range = || io::Error::new(io::ErrorKind::InvalidData, "time out of range");
    // Whole seconds, rounded down, so a pre-1970 instant keeps its day.
    let secs = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).map_err(|_| out_of_range())?,
        Err(e) => {
            let d = e.duration();
            let s = i64::try_from(d.as_secs()).map_err(|_| out_of_range())?;
            -s - i64::from(d.subsec_nanos() > 0)
        }
    };
    let secs = secs as libc::time_t;
    // SAFETY: `tm` is plain data, so all zeroes is a valid value.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are valid for the duration of the call.
    if unsafe { libc::localtime_r(&secs, &mut tm) }.is_null() {
        return Err(out_of_range());
    }
    Ok(format!("{:04}-{:02}-{:02}", i64::from(tm.tm_year) + 1900, tm.tm_mon + 1, tm.tm_mday))
}

#[cfg(not(unix))]
fn local_date(_: SystemTime) -> io::Result<String> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "local dates are not supported on this platform"))
}

/// What `copy_plan` did. With the plan's `skipped`, `duplicates` and
/// `failures`, every supported source file is in exactly one list (INV-6);
/// `Report::new` joins them.
#[derive(Debug)]
pub struct CopyOutcome {
    /// Verified and in place under its final name, in plan order.
    pub copied: Vec<Copied>,
    /// Not copied, with the reason. Nothing is left under a final name.
    pub failed: Vec<Failed>,
    /// Not copied because of a cancel, in plan order: the file cut short
    /// (its temp file removed) and every file after it. Nothing is left
    /// under a final name (INV-7).
    pub cancelled: Vec<SourceFile>,
    /// Planned files whose read failed, each copied from a twin on another
    /// card instead (F07), in plan order.
    pub fallbacks: Vec<Fallback>,
    /// Planned files whose read failed, whose twin's copy is in place but
    /// not flushed (S023), in plan order. Not in `failed`.
    pub unflushed_fallbacks: Vec<UnflushedFallback>,
    /// Stale temp files removed at the start, and temp files that could not
    /// be removed.
    pub temps: TempCleanup,
}

impl CopyOutcome {
    /// The outcome of a plan with nothing to copy, where `copy_plan` cannot
    /// run (no folder): only the stale temp cleanup, from
    /// `remove_stale_temps`, which the caller runs instead.
    pub fn nothing_to_copy(temps: TempCleanup) -> CopyOutcome {
        CopyOutcome {
            copied: Vec::new(),
            failed: Vec::new(),
            cancelled: Vec::new(),
            fallbacks: Vec::new(),
            unflushed_fallbacks: Vec::new(),
            temps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub file: SourceFile,
    /// Where the verified copy is.
    pub to: PathBuf,
}

/// A planned file whose read failed, copied from one of its twins instead:
/// the same name, size and planning hash on another card (F07). The copy
/// read the twin's bytes as that hash, so the drive holds exactly what
/// planning read from `original`.
#[derive(Debug)]
pub struct Fallback {
    /// The planned file, which could not be read.
    pub original: SourceFile,
    pub error: io::Error,
    /// The twins tried before `copied`, in plan order, and why each failed:
    /// a read error, or content not as planned (`SourceChanged`,
    /// `SizeChanged`).
    pub tried: Vec<Failed>,
    /// The twin whose bytes are on the drive now.
    pub copied: Copied,
}

/// A planned file whose read failed, and whose twin's copy is in place
/// but whose folder flush failed (S023): nothing confirms the photo is on
/// the drive, so the file fails with its own read error and the twin as
/// "not flushed", never hidden. The tries end there, as after any failure
/// on the drive's side.
#[derive(Debug)]
pub struct UnflushedFallback {
    /// The planned file, which could not be read.
    pub original: SourceFile,
    pub error: io::Error,
    /// The twins tried before `twin`, in plan order, as in `Fallback`.
    pub tried: Vec<Failed>,
    /// The twin whose copy is in place, `FailReason::NotFlushed`.
    pub twin: Failed,
}

#[derive(Debug)]
pub struct Failed {
    pub file: SourceFile,
    pub reason: FailReason,
}

/// Why one file was not copied. The sync goes on with the next file.
#[derive(Debug)]
pub enum FailReason {
    /// The target folder already holds a file with this name (DEC-013).
    /// It is left untouched.
    NameCollision { existing: PathBuf },
    /// Reading the source file failed.
    Read(io::Error),
    /// Writing, flushing or renaming on the destination failed at `path`.
    Write { path: PathBuf, error: io::Error },
    /// The verified copy is in place at `to`, but flushing its folder failed,
    /// so its name may not survive an unplug (INV-3). The next sync skips it
    /// as already synced if it survived, and copies it again if not.
    NotFlushed { to: PathBuf, error: io::Error },
    /// The flushed copy's hash differs from what was read off the source.
    Mismatch,
    /// The source file's content is not what planning read: it changed
    /// since, so what planning compared it with says nothing about it.
    SourceChanged,
    /// The source read as more or fewer bytes than the scan listed. The
    /// read-back cannot see this: it compares the copy with what was read.
    SizeChanged { scanned: u64, read: u64 },
    /// INV-10 checked again before making a folder, and the library is no
    /// longer where it was planned (DEC-027).
    Library(CopyError),
    /// Not a supported file; only a hand-built plan can hold one.
    Unsupported,
}

impl fmt::Display for FailReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FailReason::NameCollision { existing } => {
                write!(f, "name collision with {}", existing.display())
            }
            FailReason::Read(e) => write!(f, "read error: {e}"),
            FailReason::Write { path, error } => {
                write!(f, "cannot write {}: {error}", path.display())
            }
            FailReason::NotFlushed { to, error } => write!(
                f,
                "copy is in place at {}, but flushing its folder failed: {error}",
                to.display()
            ),
            FailReason::Mismatch => write!(f, "copy does not match the source"),
            FailReason::SourceChanged => write!(f, "the file changed on the source since it was planned"),
            FailReason::SizeChanged { scanned, read } => write!(
                f,
                "read {read} bytes, the scan listed {scanned}: the file changed on the source, or the source is failing"
            ),
            FailReason::Library(e) => write!(f, "library check failed: {e}"),
            FailReason::Unsupported => write!(f, "not a supported file"),
        }
    }
}

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.path.display(), self.reason)
    }
}

impl Error for Failed {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.reason {
            FailReason::Read(e)
            | FailReason::Write { error: e, .. }
            | FailReason::NotFlushed { error: e, .. } => Some(e),
            FailReason::Library(e) => Some(e),
            _ => None,
        }
    }
}

/// Why `copy_plan` did not start.
#[derive(Debug)]
pub enum CopyError {
    /// The folder belongs to another library than the plan, which was
    /// checked against its sources for that library only (INV-10).
    LibraryMismatch { plan: PathBuf, folder: PathBuf },
    /// INV-10: the library is on another device than when it was planned,
    /// e.g. a card mounted where the drive was. The plan's checks no longer
    /// hold.
    LibraryDeviceChanged { library: PathBuf, planned: u64, now: u64 },
    /// INV-10: the library is on another volume than when it was planned
    /// (`None`: the boot disk).
    LibraryVolumeChanged { library: PathBuf, planned: Option<PathBuf>, now: Option<PathBuf> },
    /// INV-10: the library is on the device of a source of the plan.
    LibraryOnSourceDevice { library: PathBuf, source_root: PathBuf },
    /// The library cannot be found or its device read, e.g. it was
    /// unplugged, so INV-10 cannot be checked again.
    Library(ScanError),
}

impl fmt::Display for CopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CopyError::LibraryMismatch { plan, folder } => write!(
                f,
                "plan is for library {} but the folder is in {}",
                plan.display(),
                folder.display()
            ),
            CopyError::LibraryDeviceChanged { library, planned, now } => write!(
                f,
                "library {} is on device {now:#x}, not on device {planned:#x} as when planned",
                library.display()
            ),
            CopyError::LibraryVolumeChanged { library, planned, now } => {
                let name = |v: &Option<PathBuf>| {
                    v.as_ref().map_or("the boot disk".to_string(), |v| v.display().to_string())
                };
                write!(
                    f,
                    "library {} is on volume {}, not on {} as when planned",
                    library.display(),
                    name(now),
                    name(planned)
                )
            }
            CopyError::LibraryOnSourceDevice { library, source_root } => write!(
                f,
                "library {} is on the same device as source {}",
                library.display(),
                source_root.display()
            ),
            CopyError::Library(e) => e.fmt(f),
        }
    }
}

impl Error for CopyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CopyError::Library(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct TempCleanup {
    pub removed: Vec<PathBuf>,
    /// Temp files (or library folders) that could not be removed (or
    /// listed). The next sync tries again.
    pub failures: Vec<ScanError>,
}

/// Copies every file in `plan.to_copy` into `folder`, one at a time, in
/// plan order, on the calling thread: a session of one sync, whose events
/// keep `EventKind`'s order. (A `Session` copies each card on its own
/// thread instead, S019.) First checks the library is still on the device
/// and volume it was planned on (INV-10), and refuses before writing or
/// deleting anything if not. Stale temp files in the library are removed
/// next; so this must not run beside another copy into the same library in
/// this process (another process's temps are safe, DEC-054).
/// A `Session` cleans once and copies through `copy_files`. A failure
/// fails that file only; the outcome lists each file once. A file whose
/// read fails is copied from its first twin that reads as the planning
/// hash, if it has one (`Planned::twins`, F07); a failure on the drive's
/// side is never retried on a twin. A twin's copy left in place but not
/// flushed ends the tries and is reported (`unflushed_fallbacks`).
///
/// Progress counts the files in `plan.to_copy` (`Phase::Copying`). A file
/// read from a twin is still that one file of its own source: no event
/// names the twin, and its bytes count only past what the failed read
/// counted (DEC-036). A cancel is checked between files, between twins and
/// before each chunk read: a file cut short loses its temp file, and it
/// and the rest go to `cancelled`. A file past its last read finishes.
pub fn copy_plan(
    plan: &SyncPlan,
    folder: &SyncFolder,
    monitor: &mut Monitor,
) -> Result<CopyOutcome, CopyError> {
    if plan.library != folder.library {
        return Err(CopyError::LibraryMismatch {
            plan: plan.library.clone(),
            folder: folder.library.clone(),
        });
    }
    recheck_library(plan)?;
    let temps = remove_stale_temps(&folder.library);
    Ok(copy_files(plan, folder, monitor, temps, Threads::One))
}

/// How `copy_files` spreads the plan's files over threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Threads {
    /// Every file on the calling thread, in plan order.
    One,
    /// One thread per card: the files grouped by their source's device, so
    /// two sources on one card (DEC-038) share a thread and the card is
    /// read one file at a time. A single card with files copies on the
    /// calling thread.
    PerCard,
}

/// The copy loop of `copy_plan`, with no check or cleanup first: the
/// caller has done both, so no copy thread starts before the cleanup has
/// ended. `folder` is in `plan.library`.
///
/// With `Threads::PerCard` each card's files go to one thread, in plan
/// order (S019). What the threads share: the progress tracker behind one
/// lock (so the observer is called by one thread at a time, each event a
/// whole snapshot), the cancel, a flag that a thread panicked, and a flag
/// that INV-10's re-check failed. Each thread keeps its own folders made
/// and temp failures. Renames into one folder wait for that folder's lock,
/// from any thread (DEC-008). A file is copied by the thread of its own
/// card only; its twins (F07) are read by that same thread, as they are
/// the plan's duplicates of it, which no thread copies as its own: two
/// threads never copy one photo. The outcome is merged in plan order,
/// whatever the timing. A panic in one thread stops the others at their
/// next check, as a cancel does; once every thread has ended, the panic
/// goes on to the caller, as it would with one thread.
pub(crate) fn copy_files(
    plan: &SyncPlan,
    folder: &SyncFolder,
    monitor: &mut Monitor,
    temps: TempCleanup,
    threads: Threads,
) -> CopyOutcome {
    let files = plan.to_copy.iter().map(|p| (Some(p.source), p.file.size));
    let tracker = Tracker::start(monitor, Phase::Copying, &plan.sources, files);
    let shared = Shared {
        cancel: tracker.cancel(),
        tracker: Mutex::new(tracker),
        panicked: AtomicBool::new(false),
        moved: AtomicBool::new(false),
        rename_way: rename_way(),
        gave_up: Mutex::new(Vec::new()),
    };
    let groups = groups(plan, threads);
    let lots: Vec<Lot> = if groups.len() < 2 {
        groups.iter().map(|g| Copier::new(plan, folder, &shared).copy_all(g)).collect()
    } else {
        let hooks = inherited_hooks();
        let joined: Vec<thread::Result<Lot>> = thread::scope(|scope| {
            let workers: Vec<_> = groups
                .iter()
                .map(|group| {
                    let (hooks, shared) = (hooks.clone(), &shared);
                    scope.spawn(move || {
                        adopt_hooks(hooks);
                        let _flag = RaiseOnPanic(&shared.panicked);
                        Copier::new(plan, folder, shared).copy_all(group)
                    })
                })
                .collect();
            // Every thread is joined before a panic goes on: none is left
            // running unnoticed.
            workers.into_iter().map(|w| w.join()).collect()
        });
        joined.into_iter().collect::<thread::Result<_>>().unwrap_or_else(|p| panic::resume_unwind(p))
    };

    let mut outcome = CopyOutcome::nothing_to_copy(temps);
    let mut done: Vec<(usize, Done)> = Vec::new();
    for lot in lots {
        done.extend(lot.files);
        outcome.temps.failures.extend(lot.temps_failed);
    }
    done.sort_by_key(|(i, _)| *i);
    for (i, d) in done {
        let file = plan.to_copy[i].file.clone();
        match d {
            Done::Copied(to) => outcome.copied.push(Copied { file, to }),
            Done::Failed(reason) => outcome.failed.push(Failed { file, reason }),
            Done::Fallback(f) => outcome.fallbacks.push(f),
            Done::Unflushed(f) => outcome.unflushed_fallbacks.push(f),
            Done::Cancelled => outcome.cancelled.push(file),
        }
    }
    let tracker = shared.tracker.into_inner().unwrap_or_else(PoisonError::into_inner);
    tracker.finish(!outcome.cancelled.is_empty());
    outcome
}

/// The plan's files as indices into `to_copy`, one group per thread, each
/// in plan order; no group is empty.
fn groups(plan: &SyncPlan, threads: Threads) -> Vec<Vec<usize>> {
    let mut groups: Vec<(u64, Vec<usize>)> = Vec::new();
    for (i, planned) in plan.to_copy.iter().enumerate() {
        let card = match threads {
            Threads::One => 0,
            Threads::PerCard => plan.source_devices[planned.source],
        };
        match groups.iter_mut().find(|(c, _)| *c == card) {
            Some((_, group)) => group.push(i),
            None => groups.push((card, vec![i])),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

/// What the copy threads share.
struct Shared<'m, 'a> {
    tracker: Mutex<Tracker<'m, 'a>>,
    cancel: Cancel,
    /// A copy thread panicked: the others stop at their next check.
    panicked: AtomicBool,
    /// The INV-10 re-check has failed, in any thread: from then on every
    /// thread runs it before every file, not only before a new folder.
    moved: AtomicBool,
    rename_way: RenameWay,
    /// The folders whose lock another process held past the limit, in this
    /// run (DEC-057). A later sync tries them again.
    gave_up: Mutex<Vec<PathBuf>>,
}

impl<'m, 'a> Shared<'m, 'a> {
    /// A cancel, or a panic in another thread.
    fn halted(&self) -> bool {
        self.cancel.is_cancelled() || self.panicked.load(Ordering::SeqCst)
    }

    fn progress(&self, update: impl FnOnce(&mut Tracker<'m, 'a>)) {
        // A thread that panicked in the observer leaves the counts whole:
        // each update is one call, and the panic ends the copy anyway.
        update(&mut self.tracker.lock().unwrap_or_else(PoisonError::into_inner));
    }

    /// Moves a verified temp to its final name. Nothing is renamed on a
    /// failure or a cancel: the caller removes the temp.
    fn rename_into_place(&self, temp: &Path, target: &Path) -> Result<(), Stop> {
        let halted = || self.halted();
        let wait = LockWait { limit: self.rename_way.limit, halted: &halted, gave_up: &self.gave_up };
        rename_noreplace(self.rename_way.excl, temp, target, &wait).map_err(|stop| match stop {
            RenameStop::Halted => Stop::Cancelled,
            RenameStop::Io(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                FailReason::NameCollision { existing: target.to_path_buf() }.into()
            }
            RenameStop::Io(error) => FailReason::Write { path: target.to_path_buf(), error }.into(),
        })
    }
}

/// Raises the flag if its thread unwinds, so the other threads stop.
struct RaiseOnPanic<'f>(&'f AtomicBool);

impl Drop for RaiseOnPanic<'_> {
    fn drop(&mut self) {
        if thread::panicking() {
            self.0.store(true, Ordering::SeqCst);
            at_stage(Stage::Halted, Path::new(""));
        }
    }
}

/// How one planned file ended.
enum Done {
    Copied(PathBuf),
    Failed(FailReason),
    Fallback(Fallback),
    Unflushed(UnflushedFallback),
    Cancelled,
}

/// What one thread did: each of its files by index in `to_copy`, and the
/// temp files it could not remove.
struct Lot {
    files: Vec<(usize, Done)>,
    temps_failed: Vec<ScanError>,
}

/// INV-10 again, before the first write and before each folder is made:
/// `plan_sync` checked the library on the device and volume it was on
/// then. A card swapped in under the same path, or a drive remounted
/// elsewhere, changes them.
pub(crate) fn recheck_library(plan: &SyncPlan) -> Result<(), CopyError> {
    let library = &plan.library;
    let lost = |source| CopyError::Library(ScanError { path: library.clone(), source });
    let resolved = fs::canonicalize(library).map_err(lost)?;
    let now = injected(Fault::Device)
        .and_then(|()| fs::metadata(&resolved))
        .and_then(|meta| device_id(&meta))
        .map_err(lost)?;
    if now != plan.library_device {
        return Err(CopyError::LibraryDeviceChanged {
            library: library.clone(),
            planned: plan.library_device,
            now,
        });
    }
    let volume = volume_root(&resolved).map_err(lost)?;
    if volume != plan.library_volume {
        return Err(CopyError::LibraryVolumeChanged {
            library: library.clone(),
            planned: plan.library_volume.clone(),
            now: volume,
        });
    }
    if let Some(i) = plan.source_devices.iter().position(|d| *d == now) {
        return Err(CopyError::LibraryOnSourceDevice {
            library: library.clone(),
            source_root: plan.sources[i].clone(),
        });
    }
    Ok(())
}

/// How a fallback ended with a twin's copy in place: verified and
/// flushed, or not flushed. Each with the twins tried before it.
enum TwinCopy {
    Copied(Vec<Failed>, Copied),
    NotFlushed(Vec<Failed>, Failed),
}

/// Why one file's copy stopped short.
enum Stop {
    Failed(FailReason),
    Cancelled,
}

impl From<FailReason> for Stop {
    fn from(reason: FailReason) -> Stop {
        Stop::Failed(reason)
    }
}

/// Deletes every stale `.photosync-tmp-*` file under `library`: what an
/// interrupted sync left behind (the INV-2 exception). `stale_temp` says
/// which: a temp another live process may be writing stays. Walks what the
/// library scan walks, so a temp inside trash or a dot folder stays. Call it
/// directly when there is nothing to copy; `copy_plan` calls it itself.
pub fn remove_stale_temps(library: &Path) -> TempCleanup {
    let mut cleanup = TempCleanup::default();
    if let Err(source) = remove_temps_in(library, &mut cleanup) {
        cleanup.failures.push(ScanError { path: library.to_path_buf(), source });
    }
    cleanup.removed.sort();
    cleanup.failures.sort_by(|a, b| a.path.cmp(&b.path));
    cleanup
}

fn remove_temps_in(dir: &Path, cleanup: &mut TempCleanup) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(source) => {
                cleanup.failures.push(ScanError { path, source });
                continue;
            }
        };
        let temp = name.as_encoded_bytes().starts_with(TEMP_PREFIX.as_bytes());
        // A folder or symlink named like a temp file is not ours: never touched.
        if temp && file_type.is_file() {
            if !stale_temp(&name) {
                continue;
            }
            let removal = fs::remove_file(&path);
            note_temp_removal(path, removal, cleanup);
        } else if !temp
            && file_type.is_dir()
            && !crate::library::skipped_entry(&name)
            && let Err(source) = remove_temps_in(&path, cleanup)
        {
            cleanup.failures.push(ScanError { path, source });
        }
    }
    Ok(())
}

fn note_temp_removal(path: PathBuf, removal: io::Result<()>, cleanup: &mut TempCleanup) {
    match removal {
        Ok(()) => cleanup.removed.push(path),
        // Another process's cleanup got there first: this one removed nothing,
        // and nothing is left to report.
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => cleanup.failures.push(ScanError { path, source }),
    }
}

/// Whether a temp named `name` was left by a run that is over (DEC-054). Its
/// writer is the pid in its name. This process's own pid counts as over: a
/// cleanup runs before the copies of its session. A name that holds no pid is
/// not one photosync wrote, so nothing shows it is stale.
fn stale_temp(name: &OsStr) -> bool {
    temp_pid(name).is_some_and(|pid| pid.cast_unsigned() == std::process::id() || !process_alive(pid))
}

/// The pid in `.photosync-tmp-<pid>-<n>-<name>`. Above zero only: `kill` takes
/// zero and below for whole process groups.
fn temp_pid(name: &OsStr) -> Option<i32> {
    let rest = name.as_encoded_bytes().strip_prefix(TEMP_PREFIX.as_bytes())?;
    let mut parts = rest.splitn(3, |&b| b == b'-');
    let number = |part: &[u8]| !part.is_empty() && part.iter().all(u8::is_ascii_digit);
    let (pid, n, _) = (parts.next()?, parts.next()?, parts.next()?);
    if !number(pid) || !number(n) {
        return None;
    }
    std::str::from_utf8(pid).ok()?.parse().ok().filter(|&pid| pid > 0)
}

/// Whether a process has this pid. Only "no such process" says no: a process
/// of another user answers EPERM, and any other answer proves nothing, so the
/// temp stays. A pid reused by another program also says yes, which only
/// leaves a temp for a later sync.
#[cfg(unix)]
fn process_alive(pid: i32) -> bool {
    // SAFETY: signal 0 sends nothing; it only asks if the process is there.
    let answered = unsafe { libc::kill(pid, 0) } == 0;
    answered || io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

#[cfg(not(unix))]
fn process_alive(_: i32) -> bool {
    // No way to ask here, and a temp left is safe.
    true
}

fn write_error(path: &Path) -> impl FnOnce(io::Error) -> FailReason + '_ {
    move |error| FailReason::Write { path: path.to_path_buf(), error }
}

/// One thread's run of the copy loop.
struct Copier<'p, 's, 'm, 'a> {
    plan: &'p SyncPlan,
    folder: &'p SyncFolder,
    /// The type folders this thread made, or found, and flushed. Each
    /// thread checks INV-10 again before its first file into each one.
    ready: Vec<PathBuf>,
    temps_failed: Vec<ScanError>,
    shared: &'s Shared<'m, 'a>,
}

impl<'p, 's, 'm, 'a> Copier<'p, 's, 'm, 'a> {
    fn new(plan: &'p SyncPlan, folder: &'p SyncFolder, shared: &'s Shared<'m, 'a>) -> Self {
        Copier { plan, folder, ready: Vec::new(), temps_failed: Vec::new(), shared }
    }

    /// Copies the files of `group` (indices into `to_copy`), in order. On a
    /// cancel, the file cut short and the rest of the group are cancelled.
    fn copy_all(mut self, group: &[usize]) -> Lot {
        let mut files = Vec::new();
        for (k, &i) in group.iter().enumerate() {
            let planned = &self.plan.to_copy[i];
            let (file, source) = (&planned.file, Some(planned.source));
            if self.shared.halted() {
                files.extend(group[k..].iter().map(|&i| (i, Done::Cancelled)));
                break;
            }
            self.shared.progress(|t| t.file_started(&file.path, source, file.size));
            let done = match self.copy_file(file, planned.hash, planned.source) {
                // Only a read of the card: the twin is on another card, but a
                // failure on the drive would fail its copy the same way.
                Err(Stop::Failed(FailReason::Read(error))) if planned.hash.is_some() => {
                    let original = file.clone();
                    match self.copy_twin(planned) {
                        Ok(Some(TwinCopy::Copied(tried, copied))) => {
                            Ok(Done::Fallback(Fallback { original, error, tried, copied }))
                        }
                        Ok(Some(TwinCopy::NotFlushed(tried, twin))) => {
                            Ok(Done::Unflushed(UnflushedFallback { original, error, tried, twin }))
                        }
                        Ok(None) => Ok(Done::Failed(FailReason::Read(error))),
                        Err(stop) => Err(stop),
                    }
                }
                Ok(to) => Ok(Done::Copied(to)),
                Err(Stop::Failed(reason)) => Ok(Done::Failed(reason)),
                Err(Stop::Cancelled) => Err(Stop::Cancelled),
            };
            match done {
                Ok(done) => files.push((i, done)),
                Err(_) => {
                    files.extend(group[k..].iter().map(|&i| (i, Done::Cancelled)));
                    break;
                }
            }
            self.shared.progress(|t| t.file_done_of(source, &file.path));
        }
        Lot { files, temps_failed: self.temps_failed }
    }

    /// F07: `planned` could not be read, so its twins are copied instead,
    /// in plan order, until one is read as the hash planning took. A twin
    /// that cannot be read, or reads otherwise, is passed for the next.
    /// `None`: no twin could be copied, or a failure on the drive's side
    /// ended the tries; the file fails with its own read error then. A
    /// twin's copy left in place unflushed ends them too, but is reported.
    /// The twins are the plan's duplicates of `planned`, read here only:
    /// no thread copies one as its own file (S019).
    fn copy_twin(&mut self, planned: &Planned) -> Result<Option<TwinCopy>, Stop> {
        let mut tried = Vec::new();
        let source = Some(planned.source);
        for twin in &planned.twins {
            if self.shared.halted() {
                return Err(Stop::Cancelled);
            }
            self.shared.progress(|t| t.reread_of(source));
            match self.copy_file(twin, planned.hash, planned.source) {
                Ok(to) => return Ok(Some(TwinCopy::Copied(tried, Copied { file: twin.clone(), to }))),
                Err(Stop::Failed(
                    reason @ (FailReason::Read(_) | FailReason::SourceChanged | FailReason::SizeChanged { .. }),
                )) => {
                    tried.push(Failed { file: twin.clone(), reason });
                }
                // Its bytes are on the drive under the final name: the
                // report must say so (INV-6), not leave it unnamed.
                Err(Stop::Failed(reason @ FailReason::NotFlushed { .. })) => {
                    return Ok(Some(TwinCopy::NotFlushed(tried, Failed { file: twin.clone(), reason })));
                }
                // The drive, not the card: the next twin would fail alike.
                Err(Stop::Failed(_)) => return Ok(None),
                Err(Stop::Cancelled) => return Err(Stop::Cancelled),
            }
        }
        Ok(None)
    }

    /// Copies `file` to `<folder>/<TYPE>/<name>`. `expected`: the hash
    /// planning took, which the bytes read must match. Its progress counts
    /// under `source`, the planned file's.
    fn copy_file(&mut self, file: &SourceFile, expected: Option<Hash>, source: usize) -> Result<PathBuf, Stop> {
        let (Some(media), Some(name)) = (file.media_type, file.path.file_name()) else {
            return Err(FailReason::Unsupported.into());
        };
        let dir = self.folder.path.join(media.dir_name());
        let new_dir = !self.ready.contains(&dir);
        if new_dir || self.shared.moved.load(Ordering::SeqCst) {
            // The library may have moved since this run checked it: a
            // folder made now would land wherever its path leads (DEC-027).
            at_stage(Stage::Checking, &file.path);
            if let Err(e) = recheck_library(self.plan) {
                self.shared.moved.store(true, Ordering::SeqCst);
                return Err(FailReason::Library(e).into());
            }
        }
        if new_dir {
            prepare_dir(self.folder, &dir)?;
            at_stage(Stage::FolderReady, &dir);
            self.ready.push(dir.clone());
        }
        let target = dir.join(name);
        // Saves reading the card; the rename below is what actually guards.
        match fs::symlink_metadata(&target) {
            Ok(_) => return Err(FailReason::NameCollision { existing: target }.into()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(FailReason::Write { path: target, error: e }.into()),
        }
        // Reading, verifying and waiting a minute would end the same way, for
        // every photo left. After the name's check: a taken name stays one.
        if listed(&self.shared.gave_up, &dir) {
            return Err(FailReason::Write { path: target, error: lock_held_too_long() }.into());
        }

        let temp = dir.join(temp_name(name));
        let out = File::options()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(write_error(&temp))?;
        // From here the temp file is ours: every failure and a cancel remove it.
        let placed = fill_and_verify(file, expected, out, &temp, &dir, self.shared, source)
            .and_then(|()| self.shared.rename_into_place(&temp, &target));
        if let Err(reason) = placed {
            if let Err(source) = fs::remove_file(&temp) {
                self.temps_failed.push(ScanError { path: temp, source });
            }
            return Err(reason);
        }
        at_stage(Stage::Renamed, &target);
        // Until the folder is flushed the new name may not survive an unplug, so
        // the file does not count as copied yet (INV-3).
        match synced(Stage::RenameSynced, &target, sync_dir(&dir)) {
            Ok(()) => Ok(target),
            Err(error) => Err(FailReason::NotFlushed { to: target, error }.into()),
        }
    }
}

/// Makes the session folder and `dir` in it, if absent, and flushes the
/// folders above `dir`, so the new folder entries survive an unplug too.
fn prepare_dir(folder: &SyncFolder, dir: &Path) -> Result<(), FailReason> {
    real_dir(&folder.path).map_err(write_error(&folder.path))?;
    real_dir(dir).map_err(write_error(dir))?;
    sync_dir(&folder.path).map_err(write_error(&folder.path))?;
    sync_dir(&folder.library).map_err(write_error(&folder.library))
}

/// Makes `dir` unless it exists, then checks it is a real folder. A symlink
/// there is never followed: it could lead out of the library (INV-9), even
/// onto the card (INV-10). `create_dir_all` would follow it.
fn real_dir(dir: &Path) -> io::Result<()> {
    match fs::create_dir(dir) {
        Err(e) if e.kind() != io::ErrorKind::AlreadyExists => return Err(e),
        _ => {}
    }
    if fs::symlink_metadata(dir)?.is_dir() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::NotADirectory, "not a folder (a symlink is never followed)"))
    }
}

/// `.photosync-tmp-<pid>-<n>-<name>`: unique per process and file, so a temp
/// file is never shared, and the camera name stays readable.
fn temp_name(name: &OsStr) -> OsString {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut temp = OsString::from(format!("{TEMP_PREFIX}{}-{n}-", std::process::id()));
    temp.push(name);
    temp
}

/// Streams the source into `out` while hashing what was read, checks its
/// length against the scan and its hash against `expected` if planning
/// hashed the file, flushes file and folder,
/// then checks the copy's hash against what was read (INV-3). The card is
/// read once. A cancel (or a panic in another copy thread) is checked
/// before each read; each chunk counts under `source`.
fn fill_and_verify(
    file: &SourceFile,
    expected: Option<Hash>,
    mut out: File,
    temp: &Path,
    dir: &Path,
    shared: &Shared,
    source: usize,
) -> Result<(), Stop> {
    let mut src = File::open(&file.path).map_err(FailReason::Read)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0; CHUNK];
    let mut total = 0;
    loop {
        if shared.halted() {
            return Err(Stop::Cancelled);
        }
        let n = match injected(Fault::Read).and_then(|()| src.read(&mut buf)) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FailReason::Read(e).into()),
        };
        total += n as u64;
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(write_error(temp))?;
        at_stage(Stage::Written, temp);
        shared.progress(|t| t.bytes_of(Some(source), n as u64));
    }
    // Before the hash check: planning may have hashed the short file too, so
    // the hash can agree. A short read gives no error (DEC-042).
    if total != file.size {
        return Err(FailReason::SizeChanged { scanned: file.size, read: total }.into());
    }
    let read = hasher.finalize();
    if expected.is_some_and(|planned| planned != read) {
        return Err(FailReason::SourceChanged.into());
    }
    synced(Stage::FileSynced, temp, flush(&out)).map_err(write_error(temp))?;
    drop(out);
    synced(Stage::DirSynced, temp, sync_dir(dir)).map_err(write_error(dir))?;
    // The read may come from the OS cache (DEC-012).
    if hash_file(temp).map_err(write_error(temp))? != read {
        return Err(FailReason::Mismatch.into());
    }
    at_stage(Stage::Verified, temp);
    Ok(())
}

fn sync_dir(dir: &Path) -> io::Result<()> {
    injected(Fault::SyncDir)?;
    flush(&File::open(dir)?)
}

/// The one way a copy reaches the disk (INV-3), for a file and for a folder.
fn flush(handle: &File) -> io::Result<()> {
    handle.sync_all()?;
    note_flush(handle);
    Ok(())
}

/// Passes on a flush's result, reporting `stage` only once it succeeded. The
/// stage does not show a flush ran: `Ok(())` in the flush's place keeps it.
/// `note_flush` shows `flush` was called on the right handle, and
/// `inv3_a_flush_reaches_the_os` that `flush` reaches the OS.
fn synced(stage: Stage, path: &Path, flushed: io::Result<()>) -> io::Result<()> {
    flushed?;
    at_stage(stage, path);
    Ok(())
}

#[cfg(unix)]
pub(crate) fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path holds a NUL byte"))
}

/// Renames `from` to `to`, failing with `AlreadyExists` if `to` exists. The
/// OS checks and renames in one step, so no other writer can slip in between
/// (DEC-008). `std::fs::rename` would replace `to`. Where the filesystem has
/// no such rename (exFAT), it checks and renames under a lock other photosync
/// processes honour too (DEC-018, DEC-055). Either way it holds the lock of
/// `to`'s folder, so this process's renames into one folder never overlap
/// (DEC-008). `excl` is the OS's rename: tests can force "unsupported".
fn rename_noreplace(
    excl: ExclRename,
    from: &Path,
    to: &Path,
    wait: &LockWait,
) -> Result<(), RenameStop> {
    let folder = match to.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let lock = folder_lock(folder)?;
    // The lock guards no data, so one a panic poisoned is still sound.
    let _held = lock.lock().unwrap_or_else(PoisonError::into_inner);
    at_stage(Stage::RenameLocked, to);
    let renamed = match excl(from, to) {
        // Only this errno: any other error may mean the rename half-ran or
        // the target is unusable, and falling back could hide it.
        Err(e) if EXCL_UNSUPPORTED.is_some_and(|code| e.raw_os_error() == Some(code)) => {
            rename_checked(folder, from, to, wait)
        }
        result => result.map_err(RenameStop::Io),
    };
    at_stage(Stage::RenameUnlocking, to);
    renamed
}

/// Checks that `to` is free, then renames, for filesystems without a no-replace
/// rename. The caller holds the lock of `to`'s folder, for this process's
/// threads; `hold_folder` keeps other photosync processes out. If it cannot,
/// nothing is renamed: the copy fails and is reported. Another program could
/// still create `to` in between; DEC-018 accepts that.
fn rename_checked(folder: &Path, from: &Path, to: &Path, wait: &LockWait) -> Result<(), RenameStop> {
    let _held = hold_folder(folder, wait)?;
    match fs::symlink_metadata(to) {
        Ok(_) => return Err(io::Error::new(io::ErrorKind::AlreadyExists, "target exists").into()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    at_fallback_checked();
    Ok(fs::rename(from, to)?)
}

/// Far above one check and rename, even on a slow drive. A holder stopped by
/// hand must fail this copy, which a later sync redoes, not hang this sync.
const FOLDER_HOLD_WAIT: Duration = Duration::from_secs(60);

/// The OS's no-replace rename.
type ExclRename = fn(&Path, &Path) -> io::Result<()>;

/// How one copy run renames. A value, so a test can force the fallback and a
/// short wait on any filesystem, for every copy thread of the run.
#[derive(Clone, Copy)]
struct RenameWay {
    excl: ExclRename,
    limit: Duration,
}

fn rename_way() -> RenameWay {
    #[cfg(test)]
    if let Some(way) = RENAME_WAY.with(std::cell::Cell::get) {
        return way;
    }
    RenameWay { excl: rename_excl, limit: FOLDER_HOLD_WAIT }
}

/// What a wait for a folder's lock answers to.
struct LockWait<'w> {
    limit: Duration,
    /// Asked on each poll: the mutex of the folder is held all through the
    /// wait, so other threads wait behind this one (DEC-057).
    halted: &'w dyn Fn() -> bool,
    gave_up: &'w Mutex<Vec<PathBuf>>,
}

fn listed(gave_up: &Mutex<Vec<PathBuf>>, folder: &Path) -> bool {
    gave_up.lock().unwrap_or_else(PoisonError::into_inner).iter().any(|f| f == folder)
}

/// Why a rename did not happen.
#[derive(Debug)]
enum RenameStop {
    /// A cancel, or a panic in another copy thread, ended the lock's wait.
    Halted,
    Io(io::Error),
}

impl From<io::Error> for RenameStop {
    fn from(error: io::Error) -> RenameStop {
        RenameStop::Io(error)
    }
}

fn lock_held_too_long() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "another photosync process holds the lock of this folder")
}

/// Takes the kernel's lock on `folder` itself, until the file returned is
/// dropped. On the folder, so no lock file is ever left on the drive (INV-9),
/// and the kernel drops it when a process dies, so a crash leaves nothing stuck
/// (DEC-055). Fails if another process holds it for `wait.limit`, as polling,
/// unlike a blocking `flock`, cannot hang.
fn hold_folder(folder: &Path, wait: &LockWait) -> Result<File, RenameStop> {
    // A thread behind the folder's mutex comes here when the wait ahead of it
    // ends: it must not start a limit of its own.
    if listed(wait.gave_up, folder) {
        return Err(lock_held_too_long().into());
    }
    let held = File::open(folder)?;
    let start = Instant::now();
    loop {
        match try_flock(&held) {
            Ok(()) => return Ok(held),
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {}
            Err(e) => return Err(e.into()),
        }
        // After the try: with the lock free a verified photo is renamed, as on
        // a filesystem with a no-replace rename.
        if (wait.halted)() {
            return Err(RenameStop::Halted);
        }
        if start.elapsed() >= wait.limit {
            wait.gave_up.lock().unwrap_or_else(PoisonError::into_inner).push(folder.to_path_buf());
            return Err(lock_held_too_long().into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(unix)]
fn try_flock(file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: plain call on a descriptor `file` keeps open.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Never reached here: `EXCL_UNSUPPORTED` is `None`, so no fallback runs.
#[cfg(not(unix))]
fn try_flock(_: &File) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "no lock across processes on this platform"))
}

/// One lock per target folder for this process, keyed by its canonical path,
/// so every path into one folder shares it, even from two libraries nested in
/// each other (DEC-019). Kept for the process's life: one small entry per
/// folder.
fn folder_lock(folder: &Path) -> io::Result<Arc<Mutex<()>>> {
    static LOCKS: Mutex<Vec<(PathBuf, Arc<Mutex<()>>)>> = Mutex::new(Vec::new());
    let key = fs::canonicalize(folder)?;
    let mut locks = LOCKS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((_, lock)) = locks.iter().find(|(path, _)| *path == key) {
        return Ok(lock.clone());
    }
    let lock = Arc::new(Mutex::new(()));
    locks.push((key, lock.clone()));
    Ok(lock)
}

/// The errno `rename_excl` gives when the filesystem cannot rename without
/// replacing: macOS says ENOTSUP (exFAT, checked on a real image).
#[cfg(target_vendor = "apple")]
const EXCL_UNSUPPORTED: Option<i32> = Some(libc::ENOTSUP);

/// `renameat2` says EINVAL for a flag the filesystem does not support. With
/// a temp file as `from` and one valid flag, that is its only cause.
#[cfg(target_os = "linux")]
const EXCL_UNSUPPORTED: Option<i32> = Some(libc::EINVAL);

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
const EXCL_UNSUPPORTED: Option<i32> = None;

#[cfg(target_vendor = "apple")]
fn rename_excl(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (c_path(from)?, c_path(to)?);
    // SAFETY: both are NUL-terminated strings that outlive the call.
    if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn rename_excl(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (c_path(from)?, c_path(to)?);
    // SAFETY: both are NUL-terminated strings that outlive the call.
    let rc = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if rc == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// No no-replace rename is wired up here, and a replacing one could
/// overwrite a photo (INV-2), so every copy fails instead. This error has no
/// errno, so it never takes the locked fallback either.
#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
fn rename_excl(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no rename that refuses to replace a file on this platform",
    ))
}

/// Points in one file's copy, for test fault injection only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// After each chunk is written to the temp file.
    Written,
    FileSynced,
    DirSynced,
    /// Hash checked, not yet renamed.
    Verified,
    Renamed,
    RenameSynced,
    // The stages below reach the shared hook only: the per-thread hook's
    // tests list the stages of one copy.
    /// The INV-10 re-check is due before copying this source file.
    Checking,
    /// This type folder is made (or was there) and flushed.
    FolderReady,
    /// The folder's rename lock is held, before the rename.
    RenameLocked,
    /// The rename is done, and the lock not yet released.
    RenameUnlocking,
    /// A copy thread is unwinding from a panic, and has told the others
    /// to stop.
    Halted,
}

#[cfg(test)]
impl Stage {
    fn of_one_copy(self) -> bool {
        matches!(
            self,
            Stage::Written
                | Stage::FileSynced
                | Stage::DirSynced
                | Stage::Verified
                | Stage::Renamed
                | Stage::RenameSynced
        )
    }
}

// TEST ONLY: a per-thread hook the unit tests set to record stages, tamper
// with files, or panic to stand in for a kill (no cleanup code runs on the
// way out). A normal build has no hook and `at_stage` does nothing. The
// shared hook is for the tests of several copy threads (S019): each copy
// thread runs with the hook of the thread that started it. The per-thread
// one stays on its own thread, so it sees one card's copy only when that
// card copies on the calling thread.
#[cfg(test)]
type Hook = Box<dyn FnMut(Stage, &Path)>;

#[cfg(test)]
type SharedHook = Arc<dyn Fn(Stage, &Path) + Send + Sync>;

#[cfg(test)]
thread_local! {
    static HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
    static SHARED_HOOK: std::cell::RefCell<Option<SharedHook>> = const { std::cell::RefCell::new(None) };
}

// TEST ONLY: runs when the fallback found the final name free and has not
// renamed yet. Apart from `Stage`: tests list a copy's stages, the same on
// every filesystem, and only exFAT passes here.
#[cfg(test)]
thread_local! {
    static FALLBACK_CHECKED: std::cell::RefCell<Option<Box<dyn Fn()>>> = const { std::cell::RefCell::new(None) };
}

// TEST ONLY: replaces how the copy runs started on this thread rename.
#[cfg(test)]
thread_local! {
    static RENAME_WAY: std::cell::Cell<Option<RenameWay>> = const { std::cell::Cell::new(None) };
}

/// Until dropped, copy runs started on this thread take the fallback, whatever
/// the filesystem, and wait `limit` for its lock.
#[cfg(test)]
struct ForcedFallback;

#[cfg(test)]
fn forced_fallback(limit: Duration) -> ForcedFallback {
    let excl: ExclRename = |_, _| Err(io::Error::from_raw_os_error(EXCL_UNSUPPORTED.unwrap()));
    RENAME_WAY.with(|way| way.set(Some(RenameWay { excl, limit })));
    ForcedFallback
}

#[cfg(test)]
impl Drop for ForcedFallback {
    fn drop(&mut self) {
        RENAME_WAY.with(|way| way.set(None));
    }
}

fn at_fallback_checked() {
    #[cfg(test)]
    FALLBACK_CHECKED.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook();
        }
    });
}

fn at_stage(stage: Stage, path: &Path) {
    #[cfg(test)]
    {
        if stage.of_one_copy() {
            HOOK.with(|hook| {
                if let Some(hook) = hook.borrow_mut().as_mut() {
                    hook(stage, path);
                }
            });
        }
        // Cloned out first: the hook may panic, or set it again.
        if let Some(hook) = SHARED_HOOK.with(|h| h.borrow().clone()) {
            hook(stage, path);
        }
    }
    #[cfg(not(test))]
    let _ = (stage, path);
}

/// What a copy thread takes from the thread that starts it: the shared
/// test hook, or nothing in a normal build.
#[cfg(test)]
type Hooks = Option<SharedHook>;
#[cfg(not(test))]
#[derive(Clone)]
struct Hooks;

#[cfg(test)]
fn inherited_hooks() -> Hooks {
    SHARED_HOOK.with(|h| h.borrow().clone())
}

#[cfg(not(test))]
fn inherited_hooks() -> Hooks {
    Hooks
}

#[cfg(test)]
fn adopt_hooks(hooks: Hooks) {
    SHARED_HOOK.with(|h| *h.borrow_mut() = hooks);
}

#[cfg(not(test))]
fn adopt_hooks(_: Hooks) {}

// TEST ONLY: what each flush that returned Ok held, read from the handle, not
// from what the caller says it flushed. Per thread, so a test sees its own
// copy when that copy runs on the calling thread. A normal build keeps nothing.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flushed {
    inode: u64,
    /// The file's size when flushed; `None` for a folder.
    file_size: Option<u64>,
}

#[cfg(test)]
thread_local! {
    static FLUSHED: std::cell::RefCell<Vec<Flushed>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn note_flush(handle: &File) {
    #[cfg(test)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = handle.metadata().unwrap();
        let file_size = (!meta.is_dir()).then_some(meta.len());
        FLUSHED.with(|f| f.borrow_mut().push(Flushed { inode: meta.ino(), file_size }));
    }
    #[cfg(not(test))]
    let _ = handle;
}

/// An I/O error a test hook can make the next read or folder flush return,
/// or every device read of the INV-10 re-check from then on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    Read,
    SyncDir,
    /// Reading the library's device in the INV-10 re-check. Stays armed,
    /// as a drive that is gone stays gone.
    Device,
}

#[cfg(test)]
thread_local! {
    static FAULT: std::cell::Cell<Option<Fault>> = const { std::cell::Cell::new(None) };
}

/// EIO if a test hook armed `fault`: once, or for `Device` until cleared. A
/// normal build never fails here.
fn injected(fault: Fault) -> io::Result<()> {
    #[cfg(test)]
    if FAULT.with(|f| f.get()) == Some(fault) {
        if fault != Fault::Device {
            FAULT.with(|f| f.set(None));
        }
        return Err(io::Error::from_raw_os_error(libc::EIO));
    }
    #[cfg(not(test))]
    let _ = fault;
    Ok(())
}

#[cfg(test)]
mod threads_tests;

#[cfg(test)]
mod tests {
    use crate::common::{Mode, TempDir, at, date_command, quiet_child, scan_card, scan_elsewhere, write_file};
    use super::*;
    use crate::plan::plan_sync;
    use std::sync::Barrier;

    fn set_hook(hook: impl FnMut(Stage, &Path) + 'static) {
        HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
    }

    fn clear_hook() {
        HOOK.with(|h| *h.borrow_mut() = None);
    }

    /// Plans and copies `card` into `lib` as a sync named "Trip".
    fn sync(card: &Path, lib: &Path) -> (SyncPlan, CopyOutcome) {
        let plan = plan_sync(lib, &[scan_elsewhere(card)], &mut Monitor::default()).unwrap();
        let folder = sync_folder(&plan, Some("Trip")).unwrap().unwrap();
        let outcome = copy_plan(&plan, &folder, &mut Monitor::default()).unwrap();
        (plan, outcome)
    }

    /// Another process's cleanup got there first: nothing is left to remove,
    /// so the report must not say a temp could not be removed.
    #[test]
    fn a_temp_already_gone_is_neither_removed_nor_failed() {
        let dir = TempDir::new();
        let gone = dir.0.join(".photosync-tmp-1-1-DSCF0001.RAF");
        let mut cleanup = TempCleanup::default();
        note_temp_removal(gone.clone(), fs::remove_file(&gone), &mut cleanup);
        assert_eq!(cleanup.removed, Vec::<PathBuf>::new());
        assert!(cleanup.failures.is_empty(), "{:?}", cleanup.failures);

        // Any other error is still a failure.
        let denied = Err(io::Error::from_raw_os_error(libc::EACCES));
        note_temp_removal(gone.clone(), denied, &mut cleanup);
        assert_eq!(cleanup.failures.len(), 1);
        assert_eq!(cleanup.failures[0].path, gone);
    }

    fn temps_under(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.extend(temps_under(&path));
            } else if path.file_name().unwrap().to_string_lossy().starts_with(TEMP_PREFIX) {
                out.push(path);
            }
        }
        out
    }

    /// Two files, the second larger than one chunk so a kill mid-copy
    /// leaves a partial temp file.
    fn card_with_two_files(root: &Path) -> (Vec<u8>, Vec<u8>) {
        let one = b"first raw".to_vec();
        let two: Vec<u8> = (0..CHUNK + CHUNK / 2).map(|i| (i % 251) as u8).collect();
        write_file(root, "DCIM/100_FUJI/DSCF0001.RAF", &one, at(1_700_000_000));
        write_file(root, "DCIM/100_FUJI/DSCF0002.RAF", &two, at(1_700_000_100));
        (one, two)
    }

    #[test]
    fn inv7_interrupted_sync_leaves_no_partial_final_file_and_rerun_completes() {
        let stages = [
            Stage::Written,
            Stage::FileSynced,
            Stage::DirSynced,
            Stage::Verified,
            Stage::Renamed,
            Stage::RenameSynced,
        ];
        for kill_at in stages {
            let dir = TempDir::new();
            let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
            fs::create_dir_all(&lib).unwrap();
            let (one, two) = card_with_two_files(&card);

            // Kill at the first time the second file reaches `kill_at`.
            set_hook(move |stage, path| {
                if stage == kill_at && path.to_string_lossy().contains("DSCF0002") {
                    panic!("simulated kill at {stage:?}");
                }
            });
            let killed = std::panic::catch_unwind(|| sync(&card, &lib));
            clear_hook();
            assert!(killed.is_err(), "{kill_at:?}: hook never fired");

            let raw = fs::read_dir(&lib).unwrap().next().unwrap().unwrap().path().join("RAW");
            assert_eq!(fs::read(raw.join("DSCF0001.RAF")).unwrap(), one, "{kill_at:?}");
            // No partial file under a final name: absent or complete.
            match fs::read(raw.join("DSCF0002.RAF")) {
                Ok(bytes) => assert_eq!(bytes, two, "{kill_at:?}"),
                Err(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound, "{kill_at:?}"),
            }
            let renamed = matches!(kill_at, Stage::Renamed | Stage::RenameSynced);
            assert_eq!(temps_under(&lib).len(), usize::from(!renamed), "{kill_at:?}");
            if kill_at == Stage::Written {
                let temp = &temps_under(&lib)[0];
                assert_eq!(fs::metadata(temp).unwrap().len(), CHUNK as u64, "partial temp");
            }

            let (plan, rerun) = sync(&card, &lib);
            assert_eq!(plan.skipped.len(), if renamed { 2 } else { 1 }, "{kill_at:?}");
            assert_eq!(rerun.copied.len(), usize::from(!renamed), "{kill_at:?}");
            assert!(rerun.failed.is_empty(), "{kill_at:?}: {:?}", rerun.failed);
            assert_eq!(rerun.temps.removed.len(), usize::from(!renamed), "{kill_at:?}");
            assert_eq!(fs::read(raw.join("DSCF0002.RAF")).unwrap(), two, "{kill_at:?}");
            assert!(temps_under(&lib).is_empty(), "{kill_at:?}");
        }
    }

    /// A cancel set at any point of the second file's copy: that file is
    /// either complete under its final name (past its last read, so it
    /// finishes) or absent with no temp left, and nothing after it starts.
    #[test]
    fn inv7_cancel_at_any_stage_leaves_no_partial_final_file_and_no_temp() {
        let stages = [
            Stage::Written,
            Stage::FileSynced,
            Stage::DirSynced,
            Stage::Verified,
            Stage::Renamed,
            Stage::RenameSynced,
        ];
        for cancel_at in stages {
            let dir = TempDir::new();
            let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
            fs::create_dir_all(&lib).unwrap();
            let (one, two) = card_with_two_files(&card);
            write_file(&card, "DCIM/100_FUJI/DSCF0003.RAF", b"third raw", at(1_700_000_200));
            let plan = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
            let folder = sync_folder(&plan, Some("Trip")).unwrap().unwrap();

            let cancel = Cancel::new();
            let flag = cancel.clone();
            set_hook(move |stage, path| {
                if stage == cancel_at && path.to_string_lossy().contains("DSCF0002") {
                    flag.cancel();
                }
            });
            let outcome = copy_plan(&plan, &folder, &mut Monitor::new(cancel, |_| {}));
            clear_hook();
            let outcome = outcome.unwrap();

            let raw = folder.path().join("RAW");
            assert_eq!(fs::read(raw.join("DSCF0001.RAF")).unwrap(), one, "{cancel_at:?}");
            assert!(temps_under(&lib).is_empty(), "{cancel_at:?}: temp left");
            assert!(outcome.failed.is_empty(), "{cancel_at:?}: {:?}", outcome.failed);
            assert!(!raw.join("DSCF0003.RAF").exists(), "{cancel_at:?}: went past the cancel");
            let cancelled: Vec<&Path> = outcome.cancelled.iter().map(|f| f.path.as_path()).collect();
            let three = card.join("DCIM/100_FUJI/DSCF0003.RAF");
            if cancel_at == Stage::Written {
                // Cut short after its first chunk: the partial temp is gone.
                assert!(!raw.join("DSCF0002.RAF").exists(), "{cancel_at:?}");
                assert_eq!(outcome.copied.len(), 1, "{cancel_at:?}");
                assert_eq!(cancelled, [card.join("DCIM/100_FUJI/DSCF0002.RAF"), three]);
            } else {
                assert_eq!(fs::read(raw.join("DSCF0002.RAF")).unwrap(), two, "{cancel_at:?}");
                assert_eq!(outcome.copied.len(), 2, "{cancel_at:?}");
                assert_eq!(cancelled, [three.as_path()], "{cancel_at:?}");
            }
        }
    }

    #[test]
    fn inv3_file_and_folder_are_flushed_before_verify_and_rename() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1_700_000_000));
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = seen.clone();
        set_hook(move |stage, _| log.borrow_mut().push(stage));
        let (_, outcome) = sync(&card, &lib);
        clear_hook();

        assert_eq!(outcome.copied.len(), 1);
        assert_eq!(
            *seen.borrow(),
            [
                Stage::Written,
                Stage::FileSynced,
                Stage::DirSynced,
                Stage::Verified,
                Stage::Renamed,
                Stage::RenameSynced,
            ]
        );
    }

    /// S030 (M02, M03): the stages above stay when a flush is replaced by
    /// `Ok(())`, so this reads what each real flush held. The folders above
    /// come first, for a new folder's entry; then the whole temp file, its
    /// folder, and after the rename the folder again. Folders are flushed
    /// through the handle `sync_dir` opens, so the inodes say which.
    #[test]
    fn inv3_each_real_flush_holds_the_whole_copy_or_its_folder() {
        use std::os::unix::fs::MetadataExt;
        let inode = |path: &Path| fs::metadata(path).unwrap().ino();
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        // Two chunks: a flush after the first would hold part of the file.
        let raw: Vec<u8> = (0..CHUNK + CHUNK / 2).map(|i| (i % 251) as u8).collect();
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", &raw, at(1_700_000_000));
        let flushes = || FLUSHED.with(|f| f.borrow().len());
        FLUSHED.with(|f| f.borrow_mut().clear());
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let temp = std::rc::Rc::new(std::cell::Cell::new(0));
        let (log, temp_seen) = (seen.clone(), temp.clone());
        set_hook(move |stage, path| {
            // Now, not after the rename: exFAT may number a renamed file anew.
            if stage == Stage::FileSynced {
                temp_seen.set(fs::metadata(path).unwrap().ino());
            }
            log.borrow_mut().push((stage, flushes()));
        });
        let (_, outcome) = sync(&card, &lib);
        clear_hook();

        let target = &outcome.copied[0].to;
        let raw_dir = target.parent().unwrap();
        let folder = |path: &Path| Flushed { inode: inode(path), file_size: None };
        let whole = Flushed { inode: temp.get(), file_size: Some(raw.len() as u64) };
        assert_eq!(
            FLUSHED.with(|f| f.borrow().clone()),
            [folder(raw_dir.parent().unwrap()), folder(&lib), whole, folder(raw_dir), folder(raw_dir)]
        );
        // Each stage comes right after its flush, and verifying and renaming
        // come after the file's and its folder's.
        let mut seen = seen.borrow().clone();
        seen.dedup();
        assert_eq!(
            seen,
            [
                (Stage::Written, 2),
                (Stage::FileSynced, 3),
                (Stage::DirSynced, 4),
                (Stage::Verified, 4),
                (Stage::Renamed, 4),
                (Stage::RenameSynced, 5),
            ]
        );
    }

    /// S030: `note_flush` runs after the flush, so the test above would not
    /// see a flush taken out of `flush` itself. The OS refuses to flush a
    /// pipe: only a real flush returns that error.
    #[test]
    fn inv3_a_flush_reaches_the_os() {
        let (reader, _writer) = io::pipe().unwrap();
        let pipe = File::from(std::os::fd::OwnedFd::from(reader));
        FLUSHED.with(|f| f.borrow_mut().clear());
        assert!(flush(&pipe).is_err(), "a pipe was flushed: no flush reached the OS");
        assert_eq!(FLUSHED.with(|f| f.borrow().len()), 0, "noted a flush that failed");
    }

    #[test]
    fn inv3_copy_that_fails_verify_is_not_placed_and_its_temp_is_removed() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"good raw", at(1_700_000_000));
        // Stands in for bytes that went bad on the way to the disk.
        set_hook(|stage, path| {
            if stage == Stage::DirSynced {
                fs::write(path, b"bad! raw").unwrap();
            }
        });
        let (_, outcome) = sync(&card, &lib);
        clear_hook();

        assert!(outcome.copied.is_empty());
        assert!(matches!(outcome.failed[..], [Failed { reason: FailReason::Mismatch, .. }]));
        let folder = fs::read_dir(&lib).unwrap().next().unwrap().unwrap().path();
        assert!(!folder.join("RAW/DSCF0001.RAF").exists());
        assert!(temps_under(&lib).is_empty());
    }

    #[test]
    fn read_error_partway_through_a_copy_removes_the_partial_temp_and_fails_that_file_only() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        let (one, _) = card_with_two_files(&card);
        write_file(&card, "DCIM/100_FUJI/DSCF0003.RAF", b"third raw", at(1_700_000_200));
        // After the first chunk of the second file is written, its next read
        // fails, as when a card goes bad mid-file.
        let partial = std::rc::Rc::new(std::cell::RefCell::new(None));
        let seen = partial.clone();
        set_hook(move |stage, temp| {
            if stage == Stage::Written && temp.to_string_lossy().contains("DSCF0002") {
                *seen.borrow_mut() = Some((temp.to_path_buf(), fs::metadata(temp).unwrap().len()));
                FAULT.with(|f| f.set(Some(Fault::Read)));
            }
        });
        let (_, outcome) = sync(&card, &lib);
        clear_hook();

        let (temp, len) = partial.borrow_mut().take().expect("hook never fired");
        assert_eq!(len, CHUNK as u64, "a partial temp existed");
        assert!(!temp.exists(), "partial temp left behind");
        assert!(temps_under(&lib).is_empty());
        assert!(outcome.temps.failures.is_empty());
        let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
        assert!(failed.file.path.ends_with("DSCF0002.RAF"));
        assert!(matches!(&failed.reason, FailReason::Read(e) if e.raw_os_error() == Some(libc::EIO)));
        let raw = temp.parent().unwrap();
        assert!(!raw.join("DSCF0002.RAF").exists());
        let copied: Vec<&Path> = outcome.copied.iter().map(|c| c.to.as_path()).collect();
        assert_eq!(copied, [raw.join("DSCF0001.RAF"), raw.join("DSCF0003.RAF")]);
        assert_eq!(fs::read(raw.join("DSCF0001.RAF")).unwrap(), one);
        assert_eq!(fs::read(raw.join("DSCF0003.RAF")).unwrap(), b"third raw");
    }

    #[test]
    fn folder_flush_failing_after_the_rename_says_the_file_is_in_place() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1_700_000_000));
        set_hook(|stage, _| {
            if stage == Stage::Renamed {
                FAULT.with(|f| f.set(Some(Fault::SyncDir)));
            }
        });
        let (_, outcome) = sync(&card, &lib);
        clear_hook();

        let target = fs::read_dir(&lib).unwrap().next().unwrap().unwrap().path().join("RAW/DSCF0001.RAF");
        assert!(outcome.copied.is_empty(), "not flushed, so not copied (INV-3)");
        let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
        assert!(matches!(&failed.reason, FailReason::NotFlushed { to, error }
            if *to == target && error.raw_os_error() == Some(libc::EIO)));
        let reason = failed.reason.to_string();
        assert!(reason.contains("in place") && reason.contains(&*target.to_string_lossy()), "{reason}");
        assert_eq!(fs::read(&target).unwrap(), b"raw");
        assert!(temps_under(&lib).is_empty());
        // DEC-017 (4): the next sync skips it as already synced.
        let (plan, rerun) = sync(&card, &lib);
        assert_eq!(plan.skipped.len(), 1);
        assert!(rerun.copied.is_empty() && rerun.failed.is_empty());
    }

    #[test]
    fn inv2_file_appearing_at_the_final_name_before_rename_survives() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"from card", at(1_700_000_000));
        // Another program writes the name after the collision check.
        set_hook(|stage, temp| {
            if stage == Stage::Verified {
                fs::write(temp.with_file_name("DSCF0001.RAF"), b"other program").unwrap();
            }
        });
        let (_, outcome) = sync(&card, &lib);
        clear_hook();

        let folder = fs::read_dir(&lib).unwrap().next().unwrap().unwrap().path();
        let target = folder.join("RAW/DSCF0001.RAF");
        assert!(outcome.copied.is_empty());
        assert!(matches!(
            &outcome.failed[..],
            [Failed { reason: FailReason::NameCollision { existing }, .. }] if *existing == target
        ));
        assert_eq!(fs::read(&target).unwrap(), b"other program");
        assert!(temps_under(&lib).is_empty());
    }

    #[test]
    fn rename_noreplace_fails_if_the_target_exists() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();

        let err = rename_noreplace(&from, &to).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&to).unwrap(), b"old");
        assert_eq!(fs::read(&from).unwrap(), b"new");
        fs::remove_file(&to).unwrap();
        rename_noreplace(&from, &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
        assert!(!from.exists());
    }

    #[test]
    fn two_racing_renames_onto_one_name_lose_no_file() {
        let dir = TempDir::new();
        for round in 0..200 {
            let target = dir.0.join(format!("DSCF{round:04}.RAF"));
            let temps: Vec<PathBuf> =
                (0..2).map(|w| dir.0.join(format!("{TEMP_PREFIX}{round}-{w}"))).collect();
            for (w, temp) in temps.iter().enumerate() {
                fs::write(temp, format!("writer {w}")).unwrap();
            }
            let barrier = Arc::new(Barrier::new(2));
            let workers: Vec<_> = temps
                .iter()
                .map(|temp| {
                    let (temp, target, barrier) = (temp.clone(), target.clone(), barrier.clone());
                    std::thread::spawn(move || {
                        barrier.wait();
                        rename_noreplace(&temp, &target)
                    })
                })
                .collect();
            let results: Vec<io::Result<()>> = workers.into_iter().map(|w| w.join().unwrap()).collect();

            let winners: Vec<usize> = (0..2).filter(|w| results[*w].is_ok()).collect();
            assert_eq!(winners.len(), 1, "round {round}: {results:?}");
            let (winner, loser) = (winners[0], 1 - winners[0]);
            assert_eq!(results[loser].as_ref().unwrap_err().kind(), io::ErrorKind::AlreadyExists);
            assert_eq!(fs::read(&target).unwrap(), format!("writer {winner}").as_bytes());
            assert_eq!(fs::read(&temps[loser]).unwrap(), format!("writer {loser}").as_bytes());
        }
    }

    /// Stands in for a filesystem without a no-replace rename, like exFAT.
    fn excl_unsupported(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(EXCL_UNSUPPORTED.unwrap()))
    }

    /// A wait no cancel reaches and that nothing gave up on before.
    fn plain_wait<R>(limit: Duration, run: impl FnOnce(&LockWait) -> Result<R, RenameStop>) -> io::Result<R> {
        let gave_up = Mutex::new(Vec::new());
        run(&LockWait { limit, halted: &|| false, gave_up: &gave_up }).map_err(|stop| match stop {
            RenameStop::Io(error) => error,
            RenameStop::Halted => panic!("nothing halts a plain wait"),
        })
    }

    fn rename_noreplace_with(excl: ExclRename, from: &Path, to: &Path) -> io::Result<()> {
        plain_wait(FOLDER_HOLD_WAIT, |wait| super::rename_noreplace(excl, from, to, wait))
    }

    fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        rename_noreplace_with(rename_excl, from, to)
    }

    fn hold_folder(folder: &Path, limit: Duration) -> io::Result<File> {
        plain_wait(limit, |wait| super::hold_folder(folder, wait))
    }

    #[test]
    fn inv2_unsupported_rename_falls_back_and_never_replaces_an_existing_target() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();

        let err = rename_noreplace_with(excl_unsupported, &from, &to).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&to).unwrap(), b"old");
        assert_eq!(fs::read(&from).unwrap(), b"new");

        // A dangling symlink is a name taken too.
        fs::remove_file(&to).unwrap();
        std::os::unix::fs::symlink(dir.0.join("nowhere"), &to).unwrap();
        let err = rename_noreplace_with(excl_unsupported, &from, &to).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert!(fs::symlink_metadata(&to).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&from).unwrap(), b"new");

        fs::remove_file(&to).unwrap();
        rename_noreplace_with(excl_unsupported, &from, &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
        assert!(!from.exists());
    }

    #[test]
    fn inv2_unsupported_rename_racing_threads_lose_no_file() {
        let dir = TempDir::new();
        for round in 0..200 {
            let target = dir.0.join(format!("DSCF{round:04}.RAF"));
            let temps: Vec<PathBuf> =
                (0..2).map(|w| dir.0.join(format!("{TEMP_PREFIX}{round}-{w}"))).collect();
            for (w, temp) in temps.iter().enumerate() {
                fs::write(temp, format!("writer {w}")).unwrap();
            }
            let barrier = Arc::new(Barrier::new(2));
            let workers: Vec<_> = temps
                .iter()
                .map(|temp| {
                    let (temp, target, barrier) = (temp.clone(), target.clone(), barrier.clone());
                    std::thread::spawn(move || {
                        barrier.wait();
                        rename_noreplace_with(excl_unsupported, &temp, &target)
                    })
                })
                .collect();
            let results: Vec<io::Result<()>> = workers.into_iter().map(|w| w.join().unwrap()).collect();

            let winners: Vec<usize> = (0..2).filter(|w| results[*w].is_ok()).collect();
            assert_eq!(winners.len(), 1, "round {round}: {results:?}");
            let (winner, loser) = (winners[0], 1 - winners[0]);
            assert_eq!(results[loser].as_ref().unwrap_err().kind(), io::ErrorKind::AlreadyExists);
            assert_eq!(fs::read(&target).unwrap(), format!("writer {winner}").as_bytes());
            assert_eq!(fs::read(&temps[loser]).unwrap(), format!("writer {loser}").as_bytes());
        }
    }

    #[test]
    fn unsupported_rename_checks_and_renames_under_the_folder_lock() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        // Another spelling of the same folder must give the same lock.
        let lock = folder_lock(&dir.0.join(".")).unwrap();
        let held = lock.lock().unwrap();

        let (f, t) = (from.clone(), to.clone());
        let worker = std::thread::spawn(move || rename_noreplace_with(excl_unsupported, &f, &t));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!worker.is_finished(), "renamed without the lock");
        assert!(from.exists() && !to.exists());

        drop(held);
        worker.join().unwrap().unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
    }

    /// DEC-008: every rename into a folder, not only the exFAT fallback,
    /// waits for that folder's lock, so two copies in one process never
    /// rename into one folder at once.
    #[test]
    fn supported_rename_also_waits_for_the_folder_lock() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        let lock = folder_lock(&dir.0).unwrap();
        let held = lock.lock().unwrap();

        let (f, t) = (from.clone(), to.clone());
        let worker = std::thread::spawn(move || rename_noreplace(&f, &t));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!worker.is_finished(), "renamed without the folder's lock");
        assert!(from.exists() && !to.exists());

        drop(held);
        worker.join().unwrap().unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
    }

    #[test]
    fn unsupported_rename_into_one_folder_by_two_paths_shares_a_lock() {
        let dir = TempDir::new();
        let raw = dir.0.join("lib/2026-10-28/RAW");
        fs::create_dir_all(&raw).unwrap();
        let (from, to) = (raw.join("from"), raw.join("to"));
        fs::write(&from, b"new").unwrap();
        // Another path into the same folder, as from a nested library.
        let lock = folder_lock(&dir.0.join("lib/2026-10-28/../2026-10-28/RAW")).unwrap();
        let held = lock.lock().unwrap();

        let (f, t) = (from.clone(), to.clone());
        let worker = std::thread::spawn(move || rename_noreplace_with(excl_unsupported, &f, &t));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!worker.is_finished(), "renamed without the folder's lock");
        assert!(from.exists() && !to.exists());

        drop(held);
        worker.join().unwrap().unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
    }

    /// A fallback writer in a process of its own, killed if the test ends first.
    struct Writer(std::process::Child);

    impl Writer {
        fn start(race: &Path, writer: &str) -> Writer {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args(["--ignored", "--exact", "copy::tests::fallback_writer"])
                .env("PHOTOSYNC_TEST_RACE", race)
                .env("PHOTOSYNC_TEST_WRITER", writer);
            Writer(quiet_child(command))
        }
    }

    impl Drop for Writer {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Waits for `mark` to exist, at most `limit`.
    fn wait_for_mark(mark: &Path, limit: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while !mark.exists() {
            if start.elapsed() > limit {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        true
    }

    /// Run by the tests below, as a process: threads share a lock that two
    /// processes do not. Renames `<race>/<writer>.temp` onto
    /// `<race>/folder/DSCF0001.RAF` by the fallback and stops after the check:
    /// writers `0` and `1` until the other has checked too, or a while, which
    /// holds the window open if nothing keeps them apart; `doomed` until killed.
    /// Marks and the result go in `<race>`, outside the folder renamed into.
    #[test]
    #[ignore = "a helper of the tests of two processes below"]
    fn fallback_writer() {
        let race = PathBuf::from(std::env::var_os("PHOTOSYNC_TEST_RACE").expect("only the tests below run this"));
        let me = std::env::var("PHOTOSYNC_TEST_WRITER").unwrap();
        let other = if me == "0" { "1" } else { "0" };
        let forever = std::time::Duration::from_secs(120);
        let (hook_race, hook_me, hook_other) = (race.clone(), me.clone(), other);
        FALLBACK_CHECKED.with(|h| {
            *h.borrow_mut() = Some(Box::new(move || {
                fs::write(hook_race.join(format!("checked-{hook_me}")), b"").unwrap();
                if hook_me == "doomed" {
                    std::thread::sleep(forever);
                    panic!("nobody killed this writer");
                }
                wait_for_mark(&hook_race.join(format!("checked-{hook_other}")), std::time::Duration::from_secs(2));
            }));
        });
        if me != "doomed" {
            // Both past their start before either checks.
            fs::write(race.join(format!("ready-{me}")), b"").unwrap();
            assert!(wait_for_mark(&race.join(format!("ready-{other}")), forever));
        }
        let result = rename_noreplace_with(
            excl_unsupported,
            &race.join(format!("{me}.temp")),
            &race.join("folder/DSCF0001.RAF"),
        );
        let written = format!("{:?}", result.map_err(|e| e.kind()));
        fs::write(race.join(format!("result-{me}")), written).unwrap();
    }

    /// S043: the race itself, in two processes, with the window between check
    /// and rename held open. One file, one "already exists", and the name is
    /// the winner's. The folder holds nothing else: the lock leaves no file.
    #[test]
    fn inv2_two_processes_on_the_fallback_give_one_file_and_one_collision() {
        let dir = TempDir::new();
        fs::create_dir(dir.0.join("folder")).unwrap();
        for w in ["0", "1"] {
            fs::write(dir.0.join(format!("{w}.temp")), format!("writer {w}")).unwrap();
        }
        let mut writers = [Writer::start(&dir.0, "0"), Writer::start(&dir.0, "1")];
        for writer in &mut writers {
            assert!(writer.0.wait().unwrap().success());
        }

        let result = |w: &str| fs::read_to_string(dir.0.join(format!("result-{w}"))).unwrap();
        let results = [result("0"), result("1")];
        let Some(winner) = results.iter().position(|r| r == "Ok(())") else {
            panic!("no writer renamed: {results:?}");
        };
        let loser = 1 - winner;
        assert_eq!(results[loser], "Err(AlreadyExists)", "both report a copy: {results:?}");
        let target = dir.0.join("folder/DSCF0001.RAF");
        assert_eq!(fs::read(&target).unwrap(), format!("writer {winner}").as_bytes());
        assert!(!dir.0.join(format!("{winner}.temp")).exists());
        assert_eq!(
            fs::read(dir.0.join(format!("{loser}.temp"))).unwrap(),
            format!("writer {loser}").as_bytes()
        );
        assert!(!dir.0.join(format!("checked-{loser}")).exists(), "the loser checked inside the winner's lock");
        let names: Vec<_> = fs::read_dir(dir.0.join("folder")).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, [OsString::from("DSCF0001.RAF")]);
    }

    /// S043: a crash leaves nothing stuck. A process killed inside the window,
    /// lock held, frees the name's folder at once. Its temp stays for a cleanup.
    #[test]
    fn a_process_killed_holding_the_fallback_lock_leaves_it_free() {
        let dir = TempDir::new();
        let folder = dir.0.join("folder");
        fs::create_dir(&folder).unwrap();
        fs::write(dir.0.join("doomed.temp"), b"doomed").unwrap();
        fs::write(dir.0.join("mine.temp"), b"mine").unwrap();

        let doomed = Writer::start(&dir.0, "doomed");
        assert!(wait_for_mark(&dir.0.join("checked-doomed"), std::time::Duration::from_secs(60)));
        let err = hold_folder(&folder, Duration::from_millis(50)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "the writer holds no lock in its window");
        drop(doomed);

        let start = Instant::now();
        let target = folder.join("DSCF0001.RAF");
        rename_noreplace_with(excl_unsupported, &dir.0.join("mine.temp"), &target).unwrap();
        assert!(start.elapsed() < FOLDER_HOLD_WAIT / 2);
        assert_eq!(fs::read(&target).unwrap(), b"mine");
        assert_eq!(fs::read(dir.0.join("doomed.temp")).unwrap(), b"doomed");
    }

    /// A lock held by another description (another process, in real use) stops
    /// the wait at its limit, with an error that is no name collision, and the
    /// lock is free again once dropped.
    #[test]
    fn a_folder_lock_held_past_the_limit_is_an_error_not_a_hang() {
        let dir = TempDir::new();
        let held = hold_folder(&dir.0, Duration::ZERO).unwrap();

        let err = hold_folder(&dir.0, Duration::from_millis(50)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert_ne!(err.kind(), io::ErrorKind::AlreadyExists);

        drop(held);
        hold_folder(&dir.0, Duration::ZERO).unwrap();
    }

    /// A library planned from `card`, with a process of its own holding the lock
    /// of the session's RAW folder until the `Writer` drops. The helper locks
    /// `<dir>/folder`, so that is made a way to the RAW folder.
    fn plan_with_raw_held(dir: &TempDir, card: &Path, lib: &Path) -> (SyncPlan, SyncFolder, PathBuf, Writer) {
        let (plan, folder) = plan_for(card, lib);
        let raw = folder.path().join("RAW");
        fs::create_dir_all(&raw).unwrap();
        std::os::unix::fs::symlink(&raw, dir.0.join("folder")).unwrap();
        fs::write(dir.0.join("doomed.temp"), b"doomed").unwrap();
        let holder = Writer::start(&dir.0, "doomed");
        assert!(wait_for_mark(&dir.0.join("checked-doomed"), Duration::from_secs(60)));
        (plan, folder, raw, holder)
    }

    fn plan_for(card: &Path, lib: &Path) -> (SyncPlan, SyncFolder) {
        let plan = plan_sync(lib, &[scan_elsewhere(card)], &mut Monitor::default()).unwrap();
        let folder = sync_folder(&plan, Some("Trip")).unwrap().unwrap();
        (plan, folder)
    }

    /// S044: another process holds the folder's lock, and the verified copy
    /// waits for it. A cancel ends the wait at once: nothing is renamed, the
    /// temp goes, and every photo is cancelled, none failed and none copied.
    #[test]
    fn s044_a_cancel_is_heard_while_another_process_holds_the_folder_lock() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        card_with_two_files(&card);
        let (plan, folder, raw, _holder) = plan_with_raw_held(&dir, &card, &lib);
        let _way = forced_fallback(Duration::from_secs(10));

        let cancel = Cancel::new();
        let cancelled_at = Arc::new(Mutex::new(None));
        let (flag, when) = (cancel.clone(), cancelled_at.clone());
        set_hook(move |stage, _| {
            if stage == Stage::Verified {
                let (flag, when) = (flag.clone(), when.clone());
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(200));
                    *when.lock().unwrap() = Some(Instant::now());
                    flag.cancel();
                });
            }
        });
        let outcome = copy_plan(&plan, &folder, &mut Monitor::new(cancel, |_| {}));
        clear_hook();
        let outcome = outcome.unwrap();

        assert!(outcome.copied.is_empty(), "{:?}", outcome.copied);
        assert!(outcome.failed.is_empty(), "{:?}", outcome.failed);
        assert_eq!(outcome.cancelled.len(), 2);
        let cancelled_at = cancelled_at.lock().unwrap().expect("the copy reached its wait");
        assert!(cancelled_at.elapsed() < Duration::from_secs(2), "{:?}", cancelled_at.elapsed());
        assert!(temps_under(&lib).is_empty());
        assert_eq!(fs::read_dir(&raw).unwrap().count(), 0, "a file was renamed");
    }

    /// S044: one wait to the limit, then the folder's other photos fail with
    /// the same reason, with the card not read. A taken name is still called
    /// that, another folder still copies, and a later sync redoes the failed.
    #[test]
    fn s044_one_lock_timeout_fails_the_folders_other_photos_at_once() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        let photos = ["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0003.RAF", "DSCF0004.RAF", "DSCF0005.JPG"];
        for (n, name) in (0..).zip(photos) {
            let at_time = at(1_700_000_000 + n);
            write_file(&card, &format!("DCIM/100_FUJI/{name}"), name.as_bytes(), at_time);
        }
        let (plan, folder, raw, holder) = plan_with_raw_held(&dir, &card, &lib);
        fs::write(raw.join("DSCF0003.RAF"), b"another photo").unwrap();
        let _way = forced_fallback(Duration::from_millis(200));

        let read = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = read.clone();
        set_hook(move |stage, path| {
            if stage == Stage::Written {
                log.borrow_mut().push(path.to_string_lossy().into_owned());
            }
        });
        let outcome = copy_plan(&plan, &folder, &mut Monitor::default());
        clear_hook();
        let outcome = outcome.unwrap();

        let read = read.borrow();
        assert_eq!(read.len(), 2, "{read:?}");
        assert!(read[0].ends_with("DSCF0001.RAF") && read[1].ends_with("DSCF0005.JPG"), "{read:?}");
        let timed_out: Vec<_> = outcome
            .failed
            .iter()
            .filter_map(|failed| match &failed.reason {
                FailReason::Write { path, error } if error.kind() == io::ErrorKind::TimedOut => {
                    Some((path.clone(), error.to_string()))
                }
                _ => None,
            })
            .collect();
        let reason = lock_held_too_long().to_string();
        let expected: Vec<_> =
            ["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0004.RAF"].map(|n| (raw.join(n), reason.clone())).to_vec();
        assert_eq!(timed_out, expected);
        assert_eq!(outcome.failed.len(), 4, "{:?}", outcome.failed);
        assert!(matches!(outcome.failed[2].reason, FailReason::NameCollision { .. }), "{:?}", outcome.failed);
        let copied: Vec<_> = outcome.copied.iter().map(|c| c.to.clone()).collect();
        assert_eq!(copied, [folder.path().join("JPEG/DSCF0005.JPG")]);
        assert!(outcome.cancelled.is_empty());
        assert!(temps_under(&lib).is_empty());
        assert_eq!(fs::read(raw.join("DSCF0003.RAF")).unwrap(), b"another photo");

        // The holder is gone: nothing of the first sync sticks.
        drop(holder);
        let (plan, folder) = plan_for(&card, &lib);
        let outcome = copy_plan(&plan, &folder, &mut Monitor::default()).unwrap();
        assert_eq!(outcome.copied.len(), 3, "{:?}", outcome.failed);
        assert_eq!(fs::read(raw.join("DSCF0001.RAF")).unwrap(), b"DSCF0001.RAF");
        assert_eq!(outcome.failed.len(), 1, "{:?}", outcome.failed);
    }

    /// S044: a folder given up on is not waited for again, even by a copy that
    /// had reached its rename already. It stays failed with the lock free now:
    /// one rule for the run, and a later sync redoes the file.
    #[test]
    fn s044_a_folder_given_up_on_fails_at_once() {
        let dir = TempDir::new();
        let gave_up = Mutex::new(vec![dir.0.clone()]);
        let wait = LockWait { limit: Duration::from_secs(60), halted: &|| false, gave_up: &gave_up };

        let start = Instant::now();
        let stop = super::hold_folder(&dir.0, &wait).unwrap_err();

        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(matches!(&stop, RenameStop::Io(e) if e.kind() == io::ErrorKind::TimedOut), "{stop:?}");
    }

    /// S044: a limit reached is recorded for the folder, so the next copy knows.
    #[test]
    fn s044_a_wait_that_reaches_its_limit_records_the_folder() {
        let dir = TempDir::new();
        let _held = hold_folder(&dir.0, Duration::ZERO).unwrap();
        let gave_up = Mutex::new(Vec::new());
        let wait = LockWait { limit: Duration::from_millis(20), halted: &|| false, gave_up: &gave_up };

        assert!(matches!(super::hold_folder(&dir.0, &wait), Err(RenameStop::Io(_))));
        assert_eq!(*gave_up.lock().unwrap(), [dir.0.clone()]);
    }

    /// S044: a cancel ends only a wait. With the lock free, a verified photo is
    /// renamed, as on a filesystem with a no-replace rename: the same stage of a
    /// cancel gives the same outcome on either.
    #[test]
    fn s044_a_cancel_with_the_lock_free_still_renames() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        let gave_up = Mutex::new(Vec::new());
        let wait = LockWait { limit: Duration::from_secs(60), halted: &|| true, gave_up: &gave_up };

        super::rename_noreplace(excl_unsupported, &from, &to, &wait).unwrap();

        assert_eq!(fs::read(&to).unwrap(), b"new");
    }

    /// Where the lock cannot be taken, nothing is renamed: going on unguarded
    /// is the race itself. A folder not readable is one way to fail it.
    #[test]
    fn a_fallback_that_cannot_lock_the_folder_renames_nothing() {
        let dir = TempDir::new();
        let folder = dir.0.join("folder");
        fs::create_dir(&folder).unwrap();
        let (from, to) = (dir.0.join("from"), folder.join("to"));
        fs::write(&from, b"new").unwrap();
        let mode = Mode::set(&folder, 0o300);

        let err = rename_noreplace_with(excl_unsupported, &from, &to).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::symlink_metadata(&to).unwrap_err().kind(), io::ErrorKind::NotFound);
        drop(mode);
        assert_eq!(fs::read(&from).unwrap(), b"new");
    }

    /// The lock across processes is the fallback's alone: a real no-replace
    /// rename needs none, so one held elsewhere does not stop it.
    #[test]
    fn a_supported_rename_takes_no_lock_across_processes() {
        fn supported(from: &Path, to: &Path) -> io::Result<()> {
            fs::rename(from, to)
        }
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        let _held = hold_folder(&dir.0, Duration::ZERO).unwrap();

        let start = Instant::now();
        rename_noreplace_with(supported, &from, &to).unwrap();

        assert!(start.elapsed() < FOLDER_HOLD_WAIT / 2);
        assert_eq!(fs::read(&to).unwrap(), b"new");
    }

    #[test]
    fn other_lock_for_another_folder() {
        let (a, b) = (TempDir::new(), TempDir::new());
        let lock = folder_lock(&a.0).unwrap();
        assert!(Arc::ptr_eq(&lock, &folder_lock(&a.0).unwrap()));
        assert!(!Arc::ptr_eq(&lock, &folder_lock(&b.0).unwrap()));
        let missing = folder_lock(&a.0.join("missing")).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn inv2_other_rename_errors_never_fall_back() {
        // A plain `fn` cannot capture, so the fake reads its errno from here.
        static CODE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
        fn fails(_: &Path, _: &Path) -> io::Result<()> {
            Err(io::Error::from_raw_os_error(CODE.load(Ordering::Relaxed)))
        }
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        // Includes the other OS's "unsupported" code: only this OS's counts.
        let codes = [
            libc::EIO,
            libc::EXDEV,
            libc::EACCES,
            libc::EPERM,
            libc::ENOSPC,
            libc::ENOSYS,
            libc::ENOTSUP,
            libc::EOPNOTSUPP,
            libc::EINVAL,
        ];
        for code in codes.into_iter().filter(|c| Some(*c) != EXCL_UNSUPPORTED) {
            CODE.store(code, Ordering::Relaxed);
            let err = rename_noreplace_with(fails, &from, &to).unwrap_err();
            assert_eq!(err.raw_os_error(), Some(code));
            assert!(from.exists() && !to.exists(), "errno {code} fell back");
        }
        // No errno at all, as on a platform with no no-replace rename.
        let err = rename_noreplace_with(
            |_, _| Err(io::Error::new(io::ErrorKind::Unsupported, "none")),
            &from,
            &to,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        assert!(from.exists() && !to.exists());
    }

    /// The plan checked INV-10 for the library's device and volume at the
    /// time. If either has changed by copy time (a card swapped in under
    /// the library's path), or the library is now on a source's device,
    /// nothing is written: no folder, no file, and stale temps stay too.
    /// The change is simulated on the plan's record.
    #[test]
    fn inv10_copy_refuses_a_library_whose_place_changed_since_planning() {
        use crate::common::{OTHER_DEVICE, manifest};
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        card_with_two_files(&card);
        write_file(&lib, &format!("2026-01-01/RAW/.photosync-tmp-{}-0-DSCF0009.RAF", std::process::id()), b"x", at(1_700_000_000));
        let good = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
        let folder = sync_folder(&good, None).unwrap().unwrap();
        let before = manifest(&lib);
        let moved_to = Some(dir.0.join("CARD"));

        let mut plan = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
        plan.library_device = OTHER_DEVICE;
        match copy_plan(&plan, &folder, &mut Monitor::default()) {
            Err(CopyError::LibraryDeviceChanged { library, planned, now }) => {
                assert_eq!((library, planned, now), (lib.clone(), OTHER_DEVICE, good.library_device));
            }
            other => panic!("expected LibraryDeviceChanged, got {other:?}"),
        }
        assert_eq!(manifest(&lib), before);

        let mut plan = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
        let was = plan.library_volume.clone();
        plan.library_volume = moved_to.clone();
        match copy_plan(&plan, &folder, &mut Monitor::default()) {
            Err(CopyError::LibraryVolumeChanged { library, planned, now }) => {
                assert_eq!((library, planned, now), (lib.clone(), moved_to.clone(), was));
            }
            other => panic!("expected LibraryVolumeChanged, got {other:?}"),
        }
        assert_eq!(manifest(&lib), before);

        let mut plan = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
        plan.source_devices = vec![plan.library_device];
        match copy_plan(&plan, &folder, &mut Monitor::default()) {
            Err(CopyError::LibraryOnSourceDevice { library, source_root }) => {
                assert_eq!((library, source_root), (lib.clone(), card.clone()));
            }
            other => panic!("expected LibraryOnSourceDevice, got {other:?}"),
        }
        assert_eq!(manifest(&lib), before);

        // Unchanged, the same plan copies.
        let outcome = copy_plan(&good, &folder, &mut Monitor::default()).unwrap();
        assert_eq!(outcome.copied.len(), 2);
    }

    /// S016 fast check: INV-10 is checked again before each folder is made,
    /// not only when the copy starts. The library's device cannot be read
    /// when the RAW folder is due: it is not made, its file fails with the
    /// reason, and from then on every file is checked, folder made or not.
    #[test]
    fn inv10_copy_checks_the_library_again_before_each_new_folder() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        // Path order: JPEG first, then RAW, then a JPEG into a folder made.
        write_file(&card, "DCIM/100_FUJI/DSCF0001.JPG", b"jpeg", at(1_700_000_000));
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1_700_000_000));
        write_file(&card, "DCIM/100_FUJI/DSCF0002.JPG", b"jpeg two", at(1_700_000_000));
        let plan = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
        let folder = sync_folder(&plan, None).unwrap().unwrap();
        set_hook(|stage, path| {
            if stage == Stage::RenameSynced && path.ends_with("JPEG/DSCF0001.JPG") {
                FAULT.with(|f| f.set(Some(Fault::Device)));
            }
        });
        let outcome = copy_plan(&plan, &folder, &mut Monitor::default());
        clear_hook();
        FAULT.with(|f| f.set(None));
        let outcome = outcome.unwrap();

        let copied: Vec<&Path> = outcome.copied.iter().map(|c| c.to.as_path()).collect();
        assert_eq!(copied, [folder.path().join("JPEG/DSCF0001.JPG")]);
        let failed: Vec<&Path> = outcome.failed.iter().map(|f| f.file.path.as_path()).collect();
        assert_eq!(failed, [card.join("DCIM/100_FUJI/DSCF0001.RAF"), card.join("DCIM/100_FUJI/DSCF0002.JPG")]);
        for f in &outcome.failed {
            assert!(
                matches!(&f.reason, FailReason::Library(CopyError::Library(e))
                    if e.path == lib && e.source.raw_os_error() == Some(libc::EIO)),
                "{:?}",
                f.reason
            );
        }
        assert!(!folder.path().join("RAW").exists(), "made a folder after the check failed");
        assert!(temps_under(&lib).is_empty());
    }

    /// A library that cannot be found at copy time (unplugged) is refused
    /// before anything is written, not copied into file by file.
    #[test]
    fn inv10_copy_refuses_a_library_it_cannot_find() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        fs::create_dir_all(&lib).unwrap();
        card_with_two_files(&card);
        let plan = plan_sync(&lib, &[scan_elsewhere(&card)], &mut Monitor::default()).unwrap();
        let folder = sync_folder(&plan, None).unwrap().unwrap();
        fs::remove_dir(&lib).unwrap();

        match copy_plan(&plan, &folder, &mut Monitor::default()) {
            Err(CopyError::Library(e)) => {
                assert_eq!(e.path, lib);
                assert_eq!(e.source.kind(), io::ErrorKind::NotFound);
            }
            other => panic!("expected Library, got {other:?}"),
        }
        assert!(!lib.exists());
    }

    /// F07: `n` cards in backup slots, each with the same RAF of two
    /// chunks, so the first card's file is planned and the others are its
    /// twins, in card order.
    fn backup_cards(dir: &Path, n: usize) -> (Vec<PathBuf>, PathBuf, Vec<u8>) {
        let lib = dir.join("lib");
        fs::create_dir_all(&lib).unwrap();
        let bytes: Vec<u8> = (0..CHUNK + CHUNK / 2).map(|i| (i % 251) as u8).collect();
        let cards: Vec<PathBuf> = (0..n).map(|i| dir.join(format!("card{i}"))).collect();
        for card in &cards {
            write_file(card, "DCIM/100_FUJI/DSCF0001.RAF", &bytes, at(1_700_000_000));
        }
        (cards, lib, bytes)
    }

    fn raf(card: &Path) -> PathBuf {
        card.join("DCIM/100_FUJI/DSCF0001.RAF")
    }

    /// One planned run of `cards` into `lib`.
    struct Run {
        scans: Vec<crate::SourceScan>,
        plan: SyncPlan,
        outcome: CopyOutcome,
        folder: SyncFolder,
        /// Chunks written to temp files, over every file of the run.
        written: usize,
    }

    /// Plans and copies `cards`. Each `(n, fault)` arms `fault` once the
    /// n-th chunk of the run (from 1) is written, so the file being copied
    /// fails at its next read (`Read`) or folder flush (`SyncDir`). The
    /// run is cancelled once chunk `cancel_at` is written, if given.
    fn run_failing(
        cards: &[PathBuf],
        lib: &Path,
        faults: &[(usize, Fault)],
        cancel_at: Option<usize>,
        monitor: impl FnOnce(Cancel) -> Monitor<'static>,
    ) -> Run {
        let scans = on_own_cards(cards);
        let plan = plan_sync(lib, &scans, &mut Monitor::default()).unwrap();
        let folder = sync_folder(&plan, None).unwrap().unwrap();
        let written = std::rc::Rc::new(std::cell::Cell::new(0));
        let (count, faults) = (written.clone(), faults.to_vec());
        let cancel = Cancel::new();
        let flag = cancel.clone();
        set_hook(move |stage, _| {
            if stage == Stage::Written {
                count.set(count.get() + 1);
                if let Some((_, fault)) = faults.iter().find(|(n, _)| *n == count.get()) {
                    FAULT.with(|f| f.set(Some(*fault)));
                }
                if cancel_at == Some(count.get()) {
                    flag.cancel();
                }
            }
        });
        let outcome = copy_plan(&plan, &folder, &mut monitor(cancel));
        clear_hook();
        FAULT.with(|f| f.set(None));
        Run { scans, plan, outcome: outcome.unwrap(), folder, written: written.get() }
    }

    /// Each card on a fake device of its own: a twin must be on another
    /// device than the file it stands in for (DEC-038).
    fn on_own_cards(cards: &[PathBuf]) -> Vec<crate::SourceScan> {
        cards.iter().zip(1..).map(|(c, n)| scan_card(c, n)).collect()
    }

    fn quiet(cancel: Cancel) -> Monitor<'static> {
        Monitor::new(cancel, |_| {})
    }

    fn eio(e: &io::Error) -> bool {
        e.raw_os_error() == Some(libc::EIO)
    }

    /// The report entry of `path`.
    fn entry<'r>(report: &'r crate::Report, path: &Path) -> &'r crate::FileOutcome {
        let mut entries = report.sources().iter().flat_map(|s| &s.files);
        &entries.find(|e| e.file.path == path).expect("listed").outcome
    }

    /// F07: the first card's file fails partway through its read, so the
    /// copy reads its twin on the second card instead, checked against
    /// the hash planning took. The third card is never read. The twin is
    /// copied; the unread file is a duplicate in session with its error.
    #[test]
    fn f07_a_file_whose_read_fails_is_copied_from_its_first_readable_twin() {
        use crate::{Bucket, FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, bytes) = backup_cards(&dir.0, 3);

        let run = run_failing(&cards, &lib, &[(1, Fault::Read)], None, quiet);

        let target = run.folder.path().join("RAW/DSCF0001.RAF");
        let o = &run.outcome;
        assert!(o.copied.is_empty() && o.failed.is_empty() && o.cancelled.is_empty(), "{o:?}");
        let [fallback] = &o.fallbacks[..] else { panic!("{:?}", o.fallbacks) };
        assert_eq!(fallback.original.path, raf(&cards[0]));
        assert!(eio(&fallback.error), "{:?}", fallback.error);
        assert!(fallback.tried.is_empty(), "{:?}", fallback.tried);
        assert_eq!((&fallback.copied.file.path, &fallback.copied.to), (&raf(&cards[1]), &target));
        assert_eq!(fs::read(&target).unwrap(), bytes);
        assert_eq!(run.written, 1 + 2, "one chunk of the first card, then the twin, read once");
        assert!(temps_under(&lib).is_empty());

        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        assert!(matches!(entry(&report, &raf(&cards[0])),
            FileOutcome::DuplicateUnread { of, error } if *of == raf(&cards[1]) && eio(error)));
        assert!(matches!(entry(&report, &raf(&cards[1])), FileOutcome::Copied { to } if *to == target));
        assert!(matches!(entry(&report, &raf(&cards[2])),
            FileOutcome::DuplicateInSession { of } if *of == raf(&cards[1])));
        let size = bytes.len() as u64;
        let buckets: Vec<Bucket> = report.sources().iter().map(|s| s.files[0].outcome.bucket()).collect();
        assert_eq!(buckets, [Bucket::Skipped, Bucket::Copied, Bucket::Skipped]);
        for source in report.sources() {
            assert_eq!(source.totals().supported().files, 1, "INV-6 per card");
        }
        let t = report.totals();
        assert_eq!((t.copied.bytes, t.skipped.bytes, t.unread.bytes), (size, 2 * size, size));
        assert_eq!(report.sources()[0].totals().unread.files, 1, "counted under the unread card");
        assert!(report.is_complete(), "the photo is on the drive");
    }

    /// F07: a twin that cannot be read, or whose content is not what
    /// planning read, is skipped for the next one. A twin that read fine
    /// before is a duplicate with its error; one that changed fails, as
    /// its bytes now are not the ones on the drive (INV-3).
    #[test]
    fn f07_a_twin_whose_read_fails_or_whose_content_changed_is_skipped_for_the_next() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, bytes) = backup_cards(&dir.0, 4);
        let mut changed = bytes.clone();
        changed[CHUNK + 1] ^= 1;
        // `run_failing` plans and copies at once; the third card changes in
        // between, so this run is spelled out.
        let run = {
            let scans = on_own_cards(&cards);
            let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
            write_file(&cards[2], "DCIM/100_FUJI/DSCF0001.RAF", &changed, at(1_700_000_000));
            let folder = sync_folder(&plan, None).unwrap().unwrap();
            let written = std::rc::Rc::new(std::cell::Cell::new(0));
            let count = written.clone();
            set_hook(move |stage, _| {
                if stage == Stage::Written {
                    count.set(count.get() + 1);
                    if count.get() <= 2 {
                        FAULT.with(|f| f.set(Some(Fault::Read)));
                    }
                }
            });
            let outcome = copy_plan(&plan, &folder, &mut Monitor::default());
            clear_hook();
            FAULT.with(|f| f.set(None));
            Run { scans, plan, outcome: outcome.unwrap(), folder, written: written.get() }
        };

        let target = run.folder.path().join("RAW/DSCF0001.RAF");
        let [fallback] = &run.outcome.fallbacks[..] else { panic!("{:?}", run.outcome) };
        assert_eq!(fallback.original.path, raf(&cards[0]));
        let tried: Vec<&Path> = fallback.tried.iter().map(|f| f.file.path.as_path()).collect();
        assert_eq!(tried, [raf(&cards[1]), raf(&cards[2])]);
        assert!(matches!(&fallback.tried[0].reason, FailReason::Read(e) if eio(e)));
        assert!(matches!(fallback.tried[1].reason, FailReason::SourceChanged));
        assert_eq!(fallback.copied.file.path, raf(&cards[3]));
        assert_eq!(fs::read(&target).unwrap(), bytes, "the planned bytes, not the changed ones");
        assert_eq!(run.written, 1 + 1 + 2 + 2);
        assert!(temps_under(&lib).is_empty());

        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        for unread in [&cards[0], &cards[1]] {
            assert!(matches!(entry(&report, &raf(unread)),
                FileOutcome::DuplicateUnread { of, error } if *of == raf(&cards[3]) && eio(error)));
        }
        assert!(matches!(entry(&report, &raf(&cards[2])), FileOutcome::Failed(FailReason::SourceChanged)));
        assert!(matches!(entry(&report, &raf(&cards[3])), FileOutcome::Copied { .. }));
        assert!(!report.is_complete(), "the changed card's bytes are not on the drive");
    }

    /// F07, audit 2026-10-30 F01: a twin that reads short is passed for the
    /// next, and fails: its bytes are not the ones on the drive.
    #[test]
    fn f07_a_twin_that_reads_short_is_passed_for_the_next() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, bytes) = backup_cards(&dir.0, 3);
        let short = CHUNK as u64 + 1;
        let scans = on_own_cards(&cards);
        let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
        let shrunk = raf(&cards[1]);
        File::options().write(true).open(&shrunk).unwrap().set_len(short).unwrap();
        let folder = sync_folder(&plan, None).unwrap().unwrap();
        let written = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = written.clone();
        set_hook(move |stage, _| {
            if stage == Stage::Written {
                count.set(count.get() + 1);
                if count.get() == 1 {
                    FAULT.with(|f| f.set(Some(Fault::Read)));
                }
            }
        });
        let outcome = copy_plan(&plan, &folder, &mut Monitor::default());
        clear_hook();
        FAULT.with(|f| f.set(None));
        let outcome = outcome.unwrap();

        let target = folder.path().join("RAW/DSCF0001.RAF");
        let [fallback] = &outcome.fallbacks[..] else { panic!("{outcome:?}") };
        let [tried] = &fallback.tried[..] else { panic!("{:?}", fallback.tried) };
        assert_eq!(tried.file.path, shrunk);
        let size = bytes.len() as u64;
        assert!(matches!(tried.reason, FailReason::SizeChanged { scanned, read } if scanned == size && read == short));
        assert_eq!(fallback.copied.file.path, raf(&cards[2]));
        assert_eq!(fs::read(&target).unwrap(), bytes);
        assert!(temps_under(&lib).is_empty());

        let report = Report::new(scans, plan, outcome).unwrap();
        assert!(matches!(entry(&report, &shrunk), FileOutcome::Failed(FailReason::SizeChanged { .. })));
        assert!(!report.is_complete(), "the short card's bytes are not on the drive");
    }

    /// F07: only a read of the card is retried on a twin. A failure on the
    /// drive's side (here a folder flush) would fail the same way for the
    /// twin: the original's is not retried, and a twin's ends the
    /// fallback, so the file fails with its own read error.
    #[test]
    fn f07_a_drive_side_failure_is_not_retried_on_a_twin() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, _) = backup_cards(&dir.0, 3);

        let run = run_failing(&cards, &lib, &[(2, Fault::SyncDir)], None, quiet);
        let [failed] = &run.outcome.failed[..] else { panic!("{:?}", run.outcome) };
        assert_eq!(failed.file.path, raf(&cards[0]));
        assert!(matches!(&failed.reason, FailReason::Write { error, .. } if eio(error)), "{failed}");
        assert!(run.outcome.fallbacks.is_empty() && run.outcome.copied.is_empty());
        assert_eq!(run.written, 2, "no twin was read");

        let run = run_failing(&cards, &lib, &[(1, Fault::Read), (3, Fault::SyncDir)], None, quiet);
        let [failed] = &run.outcome.failed[..] else { panic!("{:?}", run.outcome) };
        assert_eq!(failed.file.path, raf(&cards[0]));
        assert!(matches!(&failed.reason, FailReason::Read(e) if eio(e)), "{failed}");
        assert!(run.outcome.fallbacks.is_empty() && run.outcome.copied.is_empty());
        assert_eq!(run.written, 1 + 2, "the third card was not read");
        assert!(!run.folder.path().join("RAW/DSCF0001.RAF").exists());
        assert!(temps_under(&lib).is_empty());
        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        for twin in &cards[1..] {
            assert!(matches!(entry(&report, &raf(twin)),
                FileOutcome::DuplicateOfFailed { of } if *of == raf(&cards[0])));
        }
    }

    /// F07: with every twin unreadable too, the file fails with its own
    /// read error and its twins are failed duplicates of it.
    #[test]
    fn f07_with_every_twin_unreadable_the_file_fails_with_its_own_read_error() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, _) = backup_cards(&dir.0, 3);

        let faults = [(1, Fault::Read), (2, Fault::Read), (3, Fault::Read)];
        let run = run_failing(&cards, &lib, &faults, None, quiet);

        let [failed] = &run.outcome.failed[..] else { panic!("{:?}", run.outcome) };
        assert_eq!(failed.file.path, raf(&cards[0]));
        assert!(matches!(&failed.reason, FailReason::Read(e) if eio(e)), "{failed}");
        assert!(run.outcome.fallbacks.is_empty() && run.outcome.copied.is_empty());
        assert_eq!(run.written, 3, "each card was tried once");
        assert!(!run.folder.path().join("RAW/DSCF0001.RAF").exists());
        assert!(temps_under(&lib).is_empty());
        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        assert!(matches!(entry(&report, &raf(&cards[0])), FileOutcome::Failed(FailReason::Read(_))));
        for twin in &cards[1..] {
            assert!(matches!(entry(&report, &raf(twin)),
                FileOutcome::DuplicateOfFailed { of } if *of == raf(&cards[0])));
        }
        assert!(!report.is_complete());
    }

    /// F07: a cancel while a twin is read cancels the file, as any cut
    /// short copy: no temp, nothing under the final name.
    #[test]
    fn f07_cancel_while_reading_a_twin_cancels_the_file_and_leaves_no_temp() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, _) = backup_cards(&dir.0, 2);

        let run = run_failing(&cards, &lib, &[(1, Fault::Read)], Some(2), quiet);

        let cancelled: Vec<&Path> = run.outcome.cancelled.iter().map(|f| f.path.as_path()).collect();
        assert_eq!(cancelled, [raf(&cards[0])]);
        assert!(run.outcome.fallbacks.is_empty() && run.outcome.failed.is_empty());
        assert!(!run.folder.path().join("RAW/DSCF0001.RAF").exists());
        assert!(temps_under(&lib).is_empty());
        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        assert!(matches!(entry(&report, &raf(&cards[1])),
            FileOutcome::DuplicateOfCancelled { of } if *of == raf(&cards[0])));
    }

    /// S023: twins are only on another card (INV-3). The same photo in two
    /// folders of one card is a duplicate of the first, never its twin:
    /// when the first cannot be read, the second is not read in its place,
    /// as the same card's read error would likely hit it too. With a twin
    /// on another card, that one is read, and the second folder's copy is
    /// a duplicate of it.
    #[test]
    fn s023_the_same_photo_in_two_folders_of_one_card_is_no_twin() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, bytes) = backup_cards(&dir.0, 2);
        let (card, other) = (&cards[0], &cards[1]);
        write_file(card, "DCIM/101_FUJI/DSCF0001.RAF", &bytes, at(1_700_000_000));
        let (first, second) = (raf(card), card.join("DCIM/101_FUJI/DSCF0001.RAF"));

        let run = run_failing(std::slice::from_ref(card), &lib, &[(1, Fault::Read)], None, quiet);

        let [planned] = &run.plan.to_copy[..] else { panic!("{:?}", run.plan.to_copy) };
        assert_eq!(planned.file.path, first);
        assert!(planned.twins().is_empty(), "{:?}", planned.twins());
        let [failed] = &run.outcome.failed[..] else { panic!("{:?}", run.outcome) };
        assert!(matches!(&failed.reason, FailReason::Read(e) if eio(e)), "{failed}");
        assert!(run.outcome.fallbacks.is_empty() && run.outcome.copied.is_empty());
        assert_eq!(run.written, 1, "the second folder's copy was read in place of the first");
        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        assert!(matches!(entry(&report, &second), FileOutcome::DuplicateOfFailed { of } if *of == first));
        assert_eq!(report.totals().unread.files, 0);
        assert!(!report.is_complete());

        let run = run_failing(&cards, &lib, &[(1, Fault::Read)], None, quiet);
        let twins: Vec<&Path> = run.plan.to_copy[0].twins().iter().map(|t| t.path.as_path()).collect();
        assert_eq!(twins, [raf(other)]);
        let report = Report::new(run.scans, run.plan, run.outcome).unwrap();
        assert!(matches!(entry(&report, &first),
            FileOutcome::DuplicateUnread { of, .. } if *of == raf(other)));
        assert!(matches!(entry(&report, &second),
            FileOutcome::DuplicateInSession { of } if *of == raf(other)));
        assert!(matches!(entry(&report, &raf(other)), FileOutcome::Copied { .. }));
        assert!(report.is_complete());
    }

    /// S023: a twin's copy in place whose folder flush failed is not
    /// hidden. The twin fails as "not flushed", counted in `not_flushed`,
    /// as any copy in place unflushed; the planned file fails with its own
    /// read error, since nothing confirms the photo is on the drive; the
    /// untried twin is a failed duplicate of the copy in place.
    #[test]
    fn s023_a_twin_copy_in_place_but_not_flushed_is_reported_as_not_flushed() {
        use crate::{FileOutcome, Report};
        let dir = TempDir::new();
        let (cards, lib, bytes) = backup_cards(&dir.0, 3);
        let scans = on_own_cards(&cards);
        let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
        let folder = sync_folder(&plan, None).unwrap().unwrap();
        let written = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = written.clone();
        set_hook(move |stage, _| {
            let fault = match stage {
                Stage::Written => {
                    count.set(count.get() + 1);
                    (count.get() == 1).then_some(Fault::Read)
                }
                // Only the twin gets this far: the flush after its rename.
                Stage::Renamed => Some(Fault::SyncDir),
                _ => None,
            };
            if fault.is_some() {
                FAULT.with(|f| f.set(fault));
            }
        });
        let outcome = copy_plan(&plan, &folder, &mut Monitor::default()).unwrap();
        clear_hook();
        FAULT.with(|f| f.set(None));

        let target = folder.path().join("RAW/DSCF0001.RAF");
        assert_eq!(fs::read(&target).unwrap(), bytes, "the twin's copy is in place");
        assert_eq!(written.get(), 1 + 2, "the third card was not read");
        assert!(outcome.copied.is_empty() && outcome.fallbacks.is_empty(), "{outcome:?}");
        assert!(temps_under(&lib).is_empty());
        let report = Report::new(scans, plan, outcome).unwrap();
        assert!(matches!(entry(&report, &raf(&cards[0])), FileOutcome::Failed(FailReason::Read(e)) if eio(e)));
        assert!(matches!(entry(&report, &raf(&cards[1])),
            FileOutcome::Failed(FailReason::NotFlushed { to, error }) if *to == target && eio(error)));
        assert!(matches!(entry(&report, &raf(&cards[2])),
            FileOutcome::DuplicateOfFailed { of } if *of == raf(&cards[1])));
        let t = report.totals();
        assert_eq!((t.failed.files, t.not_flushed.files, t.unread.files), (3, 1, 0));
        assert_eq!(report.sources()[1].totals().not_flushed.files, 1, "counted under the twin's card");
        for source in report.sources() {
            assert_eq!(source.totals().supported().files, 1, "INV-6 per card");
        }
        assert!(!report.is_complete());
    }

    /// F07: a fallback is still one planned file to progress. Its bytes
    /// count under the planned file's card (DEC-036), and the twin's first
    /// bytes, as many as the failed read had counted, count nothing
    /// again: done never goes back nor past the total, and meets it.
    #[test]
    fn f07_fallback_progress_counts_the_file_once_under_its_planned_card() {
        use crate::progress::Event;
        let dir = TempDir::new();
        let (cards, lib, bytes) = backup_cards(&dir.0, 2);
        let events = std::sync::Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();

        let run = run_failing(&cards, &lib, &[(1, Fault::Read)], None, move |cancel| {
            Monitor::new(cancel, move |e: Event| log.lock().unwrap().push(e))
        });

        assert_eq!(run.outcome.fallbacks.len(), 1);
        let events = events.lock().unwrap();
        let size = bytes.len() as u64;
        let whole = crate::Counts { files: 1, bytes: size };
        for pair in events.windows(2) {
            let (p, q) = (&pair[0].progress, &pair[1].progress);
            assert_eq!(q.total, whole, "total never changes");
            assert!(q.done.files >= p.done.files && q.done.bytes >= p.done.bytes, "done went back");
            assert!(q.done.bytes <= size);
        }
        let started: Vec<&crate::EventKind> =
            events.iter().map(|e| &e.kind).filter(|k| !matches!(k, crate::EventKind::Bytes)).collect();
        use crate::EventKind::*;
        assert_eq!(
            started,
            [&Started, &FileStarted(raf(&cards[0])), &FileDone(raf(&cards[0])), &Finished]
        );
        let last = &events.last().unwrap().progress;
        assert_eq!(last.done, whole);
        assert_eq!((last.sources[0].done, last.sources[0].total), (whole, whole));
        assert_eq!(last.sources[1].total, crate::Counts::default());
    }

    #[test]
    fn local_date_matches_the_date_command() {
        // Around midnight UTC and a DST change, where a UTC date would differ.
        for secs in [0, 1_700_000_000, 1_761_609_599, 1_761_609_600, 1_761_696_000, 1_774_746_000] {
            let t = UNIX_EPOCH + std::time::Duration::from_secs(secs);
            assert_eq!(local_date(t).unwrap(), date_command(secs), "secs {secs}");
        }
    }
}
