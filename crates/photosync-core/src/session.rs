//! A sync session: one card after another, or several cards at once, into
//! one library and one folder (DEC-009). It keeps every sync's end, a
//! report or why there is none, and the worst of them, so no later sync
//! can hide an earlier one that was not complete. Front-ends only drive
//! it: scan lines and progress go to their `Watch`, every rule is here.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::copy::{
    CopyError, CopyOutcome, FolderError, SyncFolder, TempCleanup, check_name, copy_files,
    recheck_library, remove_stale_temps, sync_folder,
};
use crate::plan::{PlanError, plan_in_session};
use crate::progress::{Cancel, Event, Monitor};
use crate::report::{FileOutcome, Report, ReportError, add_by_type};
use crate::source::{MediaType, ScanError, SourceScan, scan_source};
use crate::Tally;

/// What a front-end is told while a sync runs, on the calling thread.
/// `Send`, so the sync can run on a worker thread.
pub trait Watch: Send {
    /// A source was scanned, before anything is planned.
    fn scanned(&mut self, scan: &SourceScan) {
        let _ = scan;
    }

    /// Planning or copying progress.
    fn event(&mut self, event: Event) {
        let _ = event;
    }
}

/// Why a sync gave no report. Every variant but `Report` stopped before
/// anything was written.
#[derive(Debug)]
pub enum SyncError {
    /// A source could not be scanned: it is gone, or has no readable `DCIM`.
    Scan(ScanError),
    /// `again` is `first` in another spelling (or the same).
    SourceTwice { first: PathBuf, again: PathBuf },
    Plan(PlanError),
    Folder(FolderError),
    Copy(CopyError),
    /// The copy ran but its report was refused, so nothing tells which
    /// files are safe (DEC-022 (5)).
    Report(ReportError),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Scan(e) => e.fmt(f),
            SyncError::SourceTwice { first, again } => {
                write!(f, "source {} is source {} again", again.display(), first.display())
            }
            SyncError::Plan(e) => e.fmt(f),
            SyncError::Folder(e) => e.fmt(f),
            SyncError::Copy(e) => e.fmt(f),
            SyncError::Report(e) => e.fmt(f),
        }
    }
}

impl Error for SyncError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SyncError::Scan(e) => Some(e),
            SyncError::SourceTwice { .. } => None,
            SyncError::Plan(e) => Some(e),
            SyncError::Folder(e) => Some(e),
            SyncError::Copy(e) => Some(e),
            SyncError::Report(e) => Some(e),
        }
    }
}

/// How one sync of the session ended.
#[derive(Debug)]
pub enum RunEnd {
    Report(Report),
    Refused(SyncError),
    /// It stopped with no result, e.g. its thread panicked: the front-end
    /// says so with `Session::record_crash`.
    Crashed(String),
}

/// One sync of the session: the sources it was given, and its end.
#[derive(Debug)]
pub struct Run {
    pub sources: Vec<PathBuf>,
    pub end: RunEnd,
}

/// What a sync, or the whole session, says about the cards, worst last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    /// Every supported file is on the drive and every entry was read.
    Complete,
    /// Refused before it wrote anything: its cards were not synced.
    Refused,
    /// It ran, but not every file is known to be on the drive.
    NotComplete,
}

impl Run {
    pub fn verdict(&self) -> Verdict {
        match &self.end {
            RunEnd::Report(report) if report.is_complete() => Verdict::Complete,
            RunEnd::Report(_) => Verdict::NotComplete,
            // Files may be on the drive, but nothing says which.
            RunEnd::Refused(SyncError::Report(_)) => Verdict::NotComplete,
            RunEnd::Refused(_) => Verdict::Refused,
            RunEnd::Crashed(_) => Verdict::NotComplete,
        }
    }
}

/// A sync session into one library. Its folder is fixed by its first plan
/// with something to copy, or a named folder already there (DEC-009).
/// Stale temp files are removed once, before its first copy, never again
/// while it runs. A file an earlier sync copied or found on the drive is a
/// "duplicate in session" when a later card holds it too.
#[derive(Debug)]
pub struct Session {
    library: PathBuf,
    name: Option<String>,
    folder: Option<SyncFolder>,
    cleaned: bool,
    /// Each library file an earlier sync copied or found on the drive, with
    /// the session's source file it holds.
    on_drive: HashMap<PathBuf, PathBuf>,
    runs: Vec<Run>,
}

impl Session {
    /// A session into `library`, an existing folder, with an optional sync
    /// name. Refuses a name that is not one folder name before anything
    /// runs; the library itself is checked by each sync (INV-10).
    pub fn new(library: &Path, name: Option<&str>) -> Result<Session, FolderError> {
        if let Some(name) = name {
            check_name(name)?;
        }
        Ok(Session {
            library: library.to_path_buf(),
            name: name.map(String::from),
            folder: None,
            cleaned: false,
            on_drive: HashMap::new(),
            runs: Vec::new(),
        })
    }

    pub fn library(&self) -> &Path {
        &self.library
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// `None` until a sync fixed it.
    pub fn folder(&self) -> Option<&SyncFolder> {
        self.folder.as_ref()
    }

    /// Every sync so far, in order.
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    pub fn into_runs(self) -> Vec<Run> {
        self.runs
    }

    /// One sync of the cards at `roots`, at once: scan, refuse a source
    /// given twice, plan, fix the folder if not yet fixed, check INV-10
    /// again, copy, report. A cancel during planning gives the cancelled
    /// plan's report, as no file was written. The sync is recorded,
    /// whatever its end.
    pub fn sync(&mut self, roots: &[PathBuf], cancel: &Cancel, watch: &mut dyn Watch) -> &Run {
        let mut scans = Vec::new();
        let mut scanned = Ok(());
        for root in roots {
            scanned = scan_source(root)
                .map_err(SyncError::Scan)
                .and_then(|scan| accept(&mut scans, scan, watch));
            if scanned.is_err() {
                break;
            }
        }
        let result = scanned.and_then(|()| self.run(scans, cancel, watch));
        self.end(roots.to_vec(), result)
    }

    /// `sync` of cards the caller has scanned already (`scan_source`), in
    /// this order. Each goes to `watch.scanned` here.
    pub fn sync_scans(
        &mut self,
        scans: Vec<SourceScan>,
        cancel: &Cancel,
        watch: &mut dyn Watch,
    ) -> &Run {
        let roots: Vec<PathBuf> = scans.iter().map(|s| s.root.clone()).collect();
        let mut accepted = Vec::new();
        let mut result = Ok(());
        for scan in scans {
            result = accept(&mut accepted, scan, watch);
            if result.is_err() {
                break;
            }
        }
        let result = result.and_then(|()| self.run(accepted, cancel, watch));
        self.end(roots, result)
    }

    fn end(&mut self, roots: Vec<PathBuf>, result: Result<Report, SyncError>) -> &Run {
        let end = match result {
            Ok(report) => {
                self.remember(&report);
                RunEnd::Report(report)
            }
            Err(e) => RunEnd::Refused(e),
        };
        self.push(&roots, end)
    }

    /// Records a sync of `roots` that ended with no result: the front-end
    /// lost it, e.g. its thread panicked with `what`. Not complete: files
    /// may be on the drive, but nothing says which.
    pub fn record_crash(&mut self, roots: &[PathBuf], what: String) -> &Run {
        self.push(roots, RunEnd::Crashed(what))
    }

    /// The worst verdict of every sync; `Complete` with none.
    pub fn verdict(&self) -> Verdict {
        self.runs.iter().map(Run::verdict).max().unwrap_or(Verdict::Complete)
    }

    /// The syncs that were not complete, with their index in `runs`, so a
    /// front-end can name each one.
    pub fn not_complete(&self) -> impl Iterator<Item = (usize, &Run)> {
        self.runs.iter().enumerate().filter(|(_, run)| run.verdict() != Verdict::Complete)
    }

    /// The reports of every sync, merged: totals over all of them.
    pub fn totals(&self) -> Tally {
        let mut tally = Tally::default();
        self.reports().for_each(|r| tally.merge(&r.totals()));
        tally
    }

    /// Totals per supported type over every sync's report.
    pub fn by_type(&self) -> BTreeMap<MediaType, Tally> {
        let mut types = BTreeMap::new();
        self.reports().flat_map(Report::sources).for_each(|s| add_by_type(&mut types, s));
        types
    }

    fn reports(&self) -> impl Iterator<Item = &Report> {
        self.runs.iter().filter_map(|run| match &run.end {
            RunEnd::Report(report) => Some(report),
            _ => None,
        })
    }

    fn push(&mut self, roots: &[PathBuf], end: RunEnd) -> &Run {
        self.runs.push(Run { sources: roots.to_vec(), end });
        &self.runs[self.runs.len() - 1]
    }

    fn run(
        &mut self,
        scans: Vec<SourceScan>,
        cancel: &Cancel,
        watch: &mut dyn Watch,
    ) -> Result<Report, SyncError> {
        let planned = {
            let mut monitor = Monitor::new(cancel.clone(), |e| watch.event(e));
            plan_in_session(&self.library, &scans, &mut monitor, &self.on_drive)
        };
        let plan = match planned {
            Ok(plan) => plan,
            Err(PlanError::Cancelled) => return Ok(Report::plan_cancelled(scans)),
            Err(e) => return Err(SyncError::Plan(e)),
        };
        if self.folder.is_none() {
            self.folder = sync_folder(&plan, self.name.as_deref()).map_err(SyncError::Folder)?;
        }
        // INV-10 again before this sync writes or deletes anything, the
        // cleanup included (DEC-027).
        recheck_library(&plan).map_err(SyncError::Copy)?;
        let temps = if self.cleaned {
            TempCleanup::default()
        } else {
            // Set first: a cleanup is never run twice, even after a panic.
            self.cleaned = true;
            remove_stale_temps(&self.library)
        };
        let outcome = match &self.folder {
            Some(folder) => {
                let mut monitor = Monitor::new(cancel.clone(), |e| watch.event(e));
                copy_files(&plan, folder, &mut monitor, temps)
            }
            None => CopyOutcome::nothing_to_copy(temps),
        };
        let earlier: HashSet<&PathBuf> = self.on_drive.values().collect();
        Report::build(scans, plan, outcome, &earlier).map_err(SyncError::Report)
    }

    /// Notes where each file of `report` that is on the drive is. A copy
    /// in place but not flushed is left out: a later card's twin of it is
    /// "already on drive", as the next sync would find it (DEC-017 (4)).
    /// The latest sync wins: a path it copied to or found holds its photo
    /// now, even if an earlier sync used that path before (S010 fast check).
    fn remember(&mut self, report: &Report) {
        for entry in report.sources().iter().flat_map(|s| &s.files) {
            let at = match &entry.outcome {
                FileOutcome::Copied { to } => to,
                FileOutcome::AlreadySynced { library_file } => library_file,
                _ => continue,
            };
            self.on_drive.insert(at.clone(), entry.file.path.clone());
        }
    }
}

/// Adds `scan` to the sync's `scans`, unless it is one of them in another
/// spelling (`/Volumes/A/` is `/Volumes/A`): refused before planning, as a
/// twin would be planned, and copied, twice.
fn accept(
    scans: &mut Vec<SourceScan>,
    scan: SourceScan,
    watch: &mut dyn Watch,
) -> Result<(), SyncError> {
    if let Some(first) = scans.iter().find(|s| s.resolved_root == scan.resolved_root) {
        return Err(SyncError::SourceTwice { first: first.root.clone(), again: scan.root });
    }
    watch.scanned(&scan);
    scans.push(scan);
    Ok(())
}
