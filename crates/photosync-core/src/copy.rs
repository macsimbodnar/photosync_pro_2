//! Executing a sync plan: the session folder, then for each new file a temp
//! copy, flush, verify and a rename that never replaces (INV-2, INV-3,
//! INV-7). The only files ever deleted are photosync's own temp files.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::hash::hash_file;
use crate::plan::SyncPlan;
use crate::source::{ScanError, SourceFile};

/// Every temp file photosync writes starts with this. The library scan skips
/// it, and the next sync deletes it.
const TEMP_PREFIX: &str = ".photosync-tmp-";

/// Large enough to keep a card reader streaming, small enough to hold.
const CHUNK: usize = 1 << 20;

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
}

/// Why the session folder cannot be fixed.
#[derive(Debug)]
pub enum FolderError {
    /// Empty, or holds `/` or NUL, so it is not one folder name.
    InvalidName(String),
    /// The library could not be listed, or a file's mtime has no local date.
    Io(ScanError),
}

impl fmt::Display for FolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FolderError::InvalidName(name) => write!(f, "invalid sync name {name:?}"),
            FolderError::Io(e) => e.fmt(f),
        }
    }
}

impl Error for FolderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FolderError::Io(e) => Some(e),
            FolderError::InvalidName(_) => None,
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
        if name.trim().is_empty() || name.contains(['/', '\0']) {
            return Err(FolderError::InvalidName(name.to_string()));
        }
        let existing = existing_named(library, name)
            .map_err(|source| FolderError::Io(ScanError { path: library.clone(), source }))?;
        if let Some(folder) = existing {
            return Ok(Some(SyncFolder { library: library.clone(), path: library.join(folder) }));
        }
    }
    let Some(earliest) = plan.to_copy.iter().min_by_key(|f| f.modified) else {
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

/// The earliest `YYYY-MM-DD - <name>` folder directly in `library`.
fn existing_named(library: &Path, name: &str) -> io::Result<Option<String>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(library)? {
        let entry = entry?;
        let Ok(folder) = entry.file_name().into_string() else {
            continue; // not UTF-8, so not a name the user typed
        };
        let dated = folder
            .strip_suffix(name)
            .and_then(|f| f.strip_suffix(" - "))
            .is_some_and(is_date);
        if dated && entry.file_type()?.is_dir() {
            found.push(folder);
        }
    }
    Ok(found.into_iter().min())
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
/// `failures`, every supported source file is in exactly one list (INV-6).
#[derive(Debug)]
pub struct CopyOutcome {
    /// Verified and in place under its final name, in plan order.
    pub copied: Vec<Copied>,
    /// Not copied, with the reason. Nothing is left under a final name.
    pub failed: Vec<Failed>,
    /// Stale temp files removed at the start, and temp files that could not
    /// be removed.
    pub temps: TempCleanup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub file: SourceFile,
    /// Where the verified copy is.
    pub to: PathBuf,
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
    /// The flushed copy's hash differs from what was read off the source.
    Mismatch,
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
            FailReason::Mismatch => write!(f, "copy does not match the source"),
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
            FailReason::Read(e) | FailReason::Write { error: e, .. } => Some(e),
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
        }
    }
}

impl Error for CopyError {}

#[derive(Debug, Default)]
pub struct TempCleanup {
    pub removed: Vec<PathBuf>,
    /// Temp files (or library folders) that could not be removed (or
    /// listed). The next sync tries again.
    pub failures: Vec<ScanError>,
}

/// Copies every file in `plan.to_copy` into `folder`, one at a time. Stale
/// temp files in the library are removed first. A failure fails that file
/// only; the outcome lists each file once.
pub fn copy_plan(plan: &SyncPlan, folder: &SyncFolder) -> Result<CopyOutcome, CopyError> {
    if plan.library != folder.library {
        return Err(CopyError::LibraryMismatch {
            plan: plan.library.clone(),
            folder: folder.library.clone(),
        });
    }
    let mut outcome =
        CopyOutcome { copied: Vec::new(), failed: Vec::new(), temps: remove_stale_temps(&folder.library) };
    let mut ready = Vec::new();
    // One file at a time: progress events and a cancel check (S005) go
    // between files and between chunks in `fill_and_verify`.
    for file in &plan.to_copy {
        match copy_file(file, folder, &mut ready, &mut outcome.temps) {
            Ok(to) => outcome.copied.push(Copied { file: file.clone(), to }),
            Err(reason) => outcome.failed.push(Failed { file: file.clone(), reason }),
        }
    }
    Ok(outcome)
}

/// Deletes every `.photosync-tmp-*` file under `library`: what an
/// interrupted sync left behind (the INV-2 exception). Walks what the
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
            match fs::remove_file(&path) {
                Ok(()) => cleanup.removed.push(path),
                Err(source) => cleanup.failures.push(ScanError { path, source }),
            }
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

fn write_error(path: &Path) -> impl FnOnce(io::Error) -> FailReason + '_ {
    move |error| FailReason::Write { path: path.to_path_buf(), error }
}

/// Copies one file to `<folder>/<TYPE>/<name>`. `ready` holds the type
/// folders already made and flushed in this run.
fn copy_file(
    file: &SourceFile,
    folder: &SyncFolder,
    ready: &mut Vec<PathBuf>,
    temps: &mut TempCleanup,
) -> Result<PathBuf, FailReason> {
    let (Some(media), Some(name)) = (file.media_type, file.path.file_name()) else {
        return Err(FailReason::Unsupported);
    };
    let dir = folder.path.join(media.dir_name());
    if !ready.contains(&dir) {
        prepare_dir(folder, &dir).map_err(write_error(&dir))?;
        ready.push(dir.clone());
    }
    let target = dir.join(name);
    // Saves reading the card; the rename below is what actually guards.
    match fs::symlink_metadata(&target) {
        Ok(_) => return Err(FailReason::NameCollision { existing: target }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(FailReason::Write { path: target, error: e }),
    }

    let temp = dir.join(temp_name(name));
    let out = File::options()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(write_error(&temp))?;
    // From here the temp file is ours: every failure removes it.
    let placed = fill_and_verify(file, out, &temp, &dir).and_then(|()| {
        rename_noreplace(&temp, &target, &folder.library).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                FailReason::NameCollision { existing: target.clone() }
            } else {
                FailReason::Write { path: target.clone(), error }
            }
        })
    });
    if let Err(reason) = placed {
        if let Err(source) = fs::remove_file(&temp) {
            temps.failures.push(ScanError { path: temp, source });
        }
        return Err(reason);
    }
    at_stage(Stage::Renamed, &target);
    // Until the folder is flushed the new name may not survive an unplug, so
    // the file does not count as copied yet (INV-3).
    sync_dir(&dir).map_err(write_error(&dir))?;
    at_stage(Stage::RenameSynced, &target);
    Ok(target)
}

/// Makes `dir` and flushes the folders above it, so the new folder entries
/// survive an unplug too.
fn prepare_dir(folder: &SyncFolder, dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    sync_dir(&folder.path)?;
    sync_dir(&folder.library)
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

/// Streams the source into `out` while hashing what was read, flushes file
/// and folder, then checks the copy's hash against it (INV-3). The card is
/// read once.
fn fill_and_verify(file: &SourceFile, mut out: File, temp: &Path, dir: &Path) -> Result<(), FailReason> {
    let mut src = File::open(&file.path).map_err(FailReason::Read)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0; CHUNK];
    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FailReason::Read(e)),
        };
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(write_error(temp))?;
        at_stage(Stage::Written, temp);
    }
    out.sync_all().map_err(write_error(temp))?;
    drop(out);
    at_stage(Stage::FileSynced, temp);
    sync_dir(dir).map_err(write_error(dir))?;
    at_stage(Stage::DirSynced, temp);
    // The read may come from the OS cache (DEC-012).
    if hash_file(temp).map_err(write_error(temp))? != hasher.finalize() {
        return Err(FailReason::Mismatch);
    }
    at_stage(Stage::Verified, temp);
    Ok(())
}

fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(unix)]
fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path holds a NUL byte"))
}

/// Renames `from` to `to`, failing with `AlreadyExists` if `to` exists. The
/// OS checks and renames in one step, so no other writer can slip in between
/// (DEC-008). `std::fs::rename` would replace `to`. Where the filesystem has
/// no such rename (exFAT), it checks and renames under `library`'s lock
/// instead (DEC-018).
fn rename_noreplace(from: &Path, to: &Path, library: &Path) -> io::Result<()> {
    rename_noreplace_with(rename_excl, from, to, library)
}

/// `rename_noreplace` over any OS rename, so tests can force "unsupported".
fn rename_noreplace_with(
    excl: fn(&Path, &Path) -> io::Result<()>,
    from: &Path,
    to: &Path,
    library: &Path,
) -> io::Result<()> {
    match excl(from, to) {
        // Only this errno: any other error may mean the rename half-ran or
        // the target is unusable, and falling back could hide it.
        Err(e) if EXCL_UNSUPPORTED.is_some_and(|code| e.raw_os_error() == Some(code)) => {
            rename_locked(from, to, library)
        }
        result => result,
    }
}

/// Checks that `to` is free, then renames, both under `library`'s lock, so
/// photosync's own threads never race on a name. Another program could still
/// create `to` in between; DEC-018 accepts that for filesystems without a
/// no-replace rename.
fn rename_locked(from: &Path, to: &Path, library: &Path) -> io::Result<()> {
    let lock = library_lock(library)?;
    // The lock guards no data, so one a panic poisoned is still sound.
    let _held = lock.lock().unwrap_or_else(PoisonError::into_inner);
    match fs::symlink_metadata(to) {
        Ok(_) => return Err(io::Error::new(io::ErrorKind::AlreadyExists, "target exists")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    fs::rename(from, to)
}

/// One lock per library for this process, keyed by its canonical path so two
/// spellings of one library share it (DEC-019). Kept for the process's life:
/// one small entry per library.
fn library_lock(library: &Path) -> io::Result<Arc<Mutex<()>>> {
    static LOCKS: Mutex<Vec<(PathBuf, Arc<Mutex<()>>)>> = Mutex::new(Vec::new());
    let key = fs::canonicalize(library)?;
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
}

// TEST ONLY: a per-thread hook the unit tests set to record stages, tamper
// with files, or panic to stand in for a kill (no cleanup code runs on the
// way out). A normal build has no hook and `at_stage` does nothing.
#[cfg(test)]
type Hook = Box<dyn FnMut(Stage, &Path)>;

#[cfg(test)]
thread_local! {
    static HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
}

fn at_stage(stage: Stage, path: &Path) {
    #[cfg(test)]
    HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(stage, path);
        }
    });
    #[cfg(not(test))]
    let _ = (stage, path);
}

#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::{TempDir, at, date_command, write_file};
    use super::*;
    use crate::plan::plan_sync;
    use crate::source::scan_source;
    use std::sync::Barrier;

    fn set_hook(hook: impl FnMut(Stage, &Path) + 'static) {
        HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
    }

    fn clear_hook() {
        HOOK.with(|h| *h.borrow_mut() = None);
    }

    /// Plans and copies `card` into `lib` as a sync named "Trip".
    fn sync(card: &Path, lib: &Path) -> (SyncPlan, CopyOutcome) {
        let plan = plan_sync(lib, &[scan_source(card).unwrap()]).unwrap();
        let folder = sync_folder(&plan, Some("Trip")).unwrap().unwrap();
        let outcome = copy_plan(&plan, &folder).unwrap();
        (plan, outcome)
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

        let err = rename_noreplace(&from, &to, &dir.0).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&to).unwrap(), b"old");
        assert_eq!(fs::read(&from).unwrap(), b"new");
        fs::remove_file(&to).unwrap();
        rename_noreplace(&from, &to, &dir.0).unwrap();
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
                    let library = dir.0.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        rename_noreplace(&temp, &target, &library)
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

    #[test]
    fn inv2_unsupported_rename_falls_back_and_never_replaces_an_existing_target() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();

        let err = rename_noreplace_with(excl_unsupported, &from, &to, &dir.0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&to).unwrap(), b"old");
        assert_eq!(fs::read(&from).unwrap(), b"new");

        // A dangling symlink is a name taken too.
        fs::remove_file(&to).unwrap();
        std::os::unix::fs::symlink(dir.0.join("nowhere"), &to).unwrap();
        let err = rename_noreplace_with(excl_unsupported, &from, &to, &dir.0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert!(fs::symlink_metadata(&to).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&from).unwrap(), b"new");

        fs::remove_file(&to).unwrap();
        rename_noreplace_with(excl_unsupported, &from, &to, &dir.0).unwrap();
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
                    let library = dir.0.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        rename_noreplace_with(excl_unsupported, &temp, &target, &library)
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
    fn unsupported_rename_checks_and_renames_under_the_library_lock() {
        let dir = TempDir::new();
        let (from, to) = (dir.0.join("from"), dir.0.join("to"));
        fs::write(&from, b"new").unwrap();
        // Another spelling of the same library must give the same lock.
        let lock = library_lock(&dir.0.join(".")).unwrap();
        let held = lock.lock().unwrap();

        let library = dir.0.clone();
        let (f, t) = (from.clone(), to.clone());
        let worker = std::thread::spawn(move || rename_noreplace_with(excl_unsupported, &f, &t, &library));
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!worker.is_finished(), "renamed without the lock");
        assert!(from.exists() && !to.exists());

        drop(held);
        worker.join().unwrap().unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
    }

    #[test]
    fn other_lock_for_another_library() {
        let (a, b) = (TempDir::new(), TempDir::new());
        let lock = library_lock(&a.0).unwrap();
        assert!(Arc::ptr_eq(&lock, &library_lock(&a.0).unwrap()));
        assert!(!Arc::ptr_eq(&lock, &library_lock(&b.0).unwrap()));
        let missing = library_lock(&a.0.join("missing")).unwrap_err();
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
            let err = rename_noreplace_with(fails, &from, &to, &dir.0).unwrap_err();
            assert_eq!(err.raw_os_error(), Some(code));
            assert!(from.exists() && !to.exists(), "errno {code} fell back");
        }
        // No errno at all, as on a platform with no no-replace rename.
        let err = rename_noreplace_with(
            |_, _| Err(io::Error::new(io::ErrorKind::Unsupported, "none")),
            &from,
            &to,
            &dir.0,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        assert!(from.exists() && !to.exists());
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
