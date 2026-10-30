//! A sync session: one card after another, or several cards at once, into
//! one library and one folder (DEC-009). It keeps every sync's end, a
//! report or why there is none, and the worst of them, so no later sync
//! can hide an earlier one that was not complete. The one thing that clears
//! a sync is the sync its refusal asked for: the same cards, complete
//! (`Session::completed_later`, DEC-050). Front-ends only drive it: scan
//! lines and progress go to their `Watch`, every rule is here.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::copy::{
    CopyError, CopyOutcome, FolderError, SyncFolder, TempCleanup, Threads, check_name,
    check_name_fits,
    copy_files, recheck_library, remove_stale_temps, sync_folder,
};
use crate::plan::{PlanError, plan_in_session};
use crate::progress::{Cancel, Event, Monitor};
use crate::report::{FileOutcome, Report, ReportError, add_by_type};
use crate::source::{MediaType, ScanError, SourceScan, scan_source};
use crate::Tally;

/// What a front-end is told while a sync runs, one call at a time. Scans
/// and planning come on the calling thread; copy events come from the
/// cards' copy threads too, under one lock (`Monitor::new`, S019). `Send`,
/// so the sync can run on a worker thread, and the copy threads can call
/// it.
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

/// What one card held when a sync read it: enough to tell later whether a
/// sync at that path had that card's photos, as a swapped card comes back at
/// the same path (DEC-034 (1), DEC-050).
#[derive(Debug)]
struct Listed {
    resolved_root: PathBuf,
    /// Under the root, so two spellings of one root compare equal.
    files: HashSet<(PathBuf, u64, SystemTime)>,
    /// False when part of the card could not be read: what it held is then
    /// not known, and no later sync can be shown to hold all of it.
    whole: bool,
}

impl Listed {
    fn of(scan: &SourceScan) -> Listed {
        let under_root = |path: &Path| path.strip_prefix(&scan.root).unwrap_or(path).to_path_buf();
        Listed {
            resolved_root: scan.resolved_root.clone(),
            files: scan.supported().map(|f| (under_root(&f.path), f.size, f.modified)).collect(),
            whole: scan.failures.is_empty(),
        }
    }

    /// Whether `later` is this card: same path, and all these photos on it.
    fn is_in(&self, later: &Listed) -> bool {
        self.whole
            && self.resolved_root == later.resolved_root
            && self.files.is_subset(&later.files)
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
    /// For each run, in the order of its sources, the cards it read. Stops
    /// at the first it could not; a recorded run read none.
    listed: Vec<Vec<Listed>>,
}

impl Session {
    /// A session into `library`, an existing folder, with an optional sync
    /// name. Refuses a name that is not one folder name before anything
    /// runs; the library itself is checked by each sync (INV-10).
    pub fn new(library: &Path, name: Option<&str>) -> Result<Session, FolderError> {
        if let Some(name) = name {
            check_name(name)?;
            check_name_fits(library, name)?;
        }
        Ok(Session {
            library: library.to_path_buf(),
            name: name.map(String::from),
            folder: None,
            cleaned: false,
            on_drive: HashMap::new(),
            runs: Vec::new(),
            listed: Vec::new(),
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
    /// again, remove stale temps if this is the session's first copy, then
    /// copy each card on its own thread (one per device, S019), report. A cancel during planning gives the cancelled
    /// plan's report, as no file was written. The sync is recorded,
    /// whatever its end.
    pub fn sync(&mut self, roots: &[PathBuf], cancel: &Cancel, watch: &mut dyn Watch) -> &Run {
        let mut scans = Vec::new();
        let mut listed = Vec::new();
        let mut scanned = Ok(());
        for root in roots {
            scanned = scan_source(root).map_err(SyncError::Scan).and_then(|scan| {
                listed.push(Listed::of(&scan));
                accept(&mut scans, scan, watch)
            });
            if scanned.is_err() {
                break;
            }
        }
        let result = scanned.and_then(|()| self.run(scans, cancel, watch));
        self.end(roots.to_vec(), listed, result)
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
        let listed = scans.iter().map(Listed::of).collect();
        let mut accepted = Vec::new();
        let mut result = Ok(());
        for scan in scans {
            result = accept(&mut accepted, scan, watch);
            if result.is_err() {
                break;
            }
        }
        let result = result.and_then(|()| self.run(accepted, cancel, watch));
        self.end(roots, listed, result)
    }

    fn end(
        &mut self,
        roots: Vec<PathBuf>,
        listed: Vec<Listed>,
        result: Result<Report, SyncError>,
    ) -> &Run {
        let end = match result {
            Ok(report) => {
                self.remember(&report);
                RunEnd::Report(report)
            }
            Err(e) => RunEnd::Refused(e),
        };
        self.listed.push(listed);
        self.push(&roots, end)
    }

    /// Records a sync of `roots` that ended with no result: the front-end
    /// lost it, e.g. its thread panicked with `what`. Not complete: files
    /// may be on the drive, but nothing says which.
    pub fn record_crash(&mut self, roots: &[PathBuf], what: String) -> &Run {
        self.listed.push(Vec::new());
        self.push(roots, RunEnd::Crashed(what))
    }

    /// Records a sync of `roots` that ended with `error`, not run here. For
    /// a front-end's tests: no real sync gives some ends, such as a report
    /// that does not add up, and a front-end may not use this crate's test
    /// seam (DEC-027, DEC-035). A refusal is never `Complete`, so recording
    /// one can only keep or worsen the session's verdict, never make the
    /// cards look safe.
    pub fn record_refusal(&mut self, roots: &[PathBuf], error: SyncError) -> &Run {
        self.listed.push(Vec::new());
        self.push(roots, RunEnd::Refused(error))
    }

    /// The worst verdict of every sync, but a refused one whose cards all
    /// completed later; `Complete` with none.
    pub fn verdict(&self) -> Verdict {
        self.not_complete().map(|(_, run)| run.verdict()).max().unwrap_or(Verdict::Complete)
    }

    /// The syncs that were not complete and still count, with their index in
    /// `runs`, so a front-end can name each one.
    pub fn not_complete(&self) -> impl Iterator<Item = (usize, &Run)> {
        self.runs.iter().enumerate().filter(|(i, run)| {
            run.verdict() != Verdict::Complete && !self.completed_later(*i)
        })
    }

    /// Whether sync `index` was refused, and every card it was given then
    /// completed in a later sync of this session (DEC-050). It wrote nothing
    /// (`Verdict::Refused`), so all that was left to do is what those syncs
    /// did. The sync itself stays in `runs`, refused.
    pub fn completed_later(&self, index: usize) -> bool {
        self.runs.get(index).is_some_and(|run| {
            !run.sources.is_empty() && self.cards_left(index).is_empty()
        })
    }

    /// The cards of refused sync `index` that no later sync completed: all
    /// of them for a sync of any other end.
    pub fn cards_left(&self, index: usize) -> Vec<&Path> {
        let Some(run) = self.runs.get(index) else {
            return Vec::new();
        };
        let sources = run.sources.iter().map(PathBuf::as_path);
        if run.verdict() != Verdict::Refused {
            return sources.collect();
        }
        // A whole report only: its other card's failure may be this card's
        // file too, as a twin takes its original's bucket (INV-6).
        let complete: Vec<&Listed> = (index + 1..self.runs.len())
            .filter(|&later| matches!(&self.runs[later].end, RunEnd::Report(r) if r.is_complete()))
            .flat_map(|later| &self.listed[later])
            .collect();
        let read = &self.listed[index];
        sources
            .enumerate()
            .filter(|(i, _)| {
                // Not read: nothing says which card was there.
                !read.get(*i).is_some_and(|card| complete.iter().any(|later| card.is_in(later)))
            })
            .map(|(_, source)| source)
            .collect()
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
                copy_files(&plan, folder, &mut monitor, temps, Threads::PerCard)
            }
            None => CopyOutcome::nothing_to_copy(temps),
        };
        let earlier: HashSet<&PathBuf> = self.on_drive.values().collect();
        refused_for_test(Report::build(scans, plan, outcome, &earlier)).map_err(SyncError::Report)
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

// TEST ONLY: no real sync gives a report that does not add up (DEC-034
// (6)), so a unit test arms this to refuse the next one, after its copy.
// A normal build has no switch and `refused_for_test` passes the report on.
#[cfg(test)]
thread_local! {
    static REFUSE_REPORT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn refused_for_test(report: Result<Report, ReportError>) -> Result<Report, ReportError> {
    #[cfg(test)]
    if REFUSE_REPORT.with(|armed| armed.replace(false)) {
        return Err(ReportError::Unaccounted(PathBuf::from("armed by a test")));
    }
    report
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{TempDir, at, scan_elsewhere, write_file};
    use crate::plan::{Duplicate, plan_sync};

    struct Quiet;
    impl Watch for Quiet {}

    /// S022, the case S020 dropped: a sync whose report is refused after
    /// its copy is not complete. Files are on the drive, but nothing says
    /// which, and a later complete sync of the same card does not hide it.
    #[test]
    fn a_report_refused_after_copying_is_not_complete() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(1_700_000_000));
        std::fs::create_dir_all(&lib).unwrap();
        let mut session = Session::new(&lib, None).unwrap();

        REFUSE_REPORT.with(|armed| armed.set(true));
        let run = session.sync_scans(vec![scan_elsewhere(&card)], &Cancel::new(), &mut Quiet);
        assert!(matches!(run.end, RunEnd::Refused(SyncError::Report(_))), "{run:?}");
        assert_eq!(run.verdict(), Verdict::NotComplete);
        assert!(session.folder().unwrap().path().join("RAW/DSCF0001.RAF").is_file(), "not copied");
        assert_eq!(session.verdict(), Verdict::NotComplete);

        let again = session.sync_scans(vec![scan_elsewhere(&card)], &Cancel::new(), &mut Quiet);
        assert!(matches!(&again.end, RunEnd::Report(r) if r.is_complete()), "{again:?}");
        assert_eq!(session.verdict(), Verdict::NotComplete, "a later sync hid it");
        let named: Vec<usize> = session.not_complete().map(|(i, _)| i).collect();
        assert_eq!(named, [0]);
    }

    /// S038: a file in `seen_earlier` was never copied, so it vouches for
    /// nothing. Planning never lists a duplicate of one, and a plan that did
    /// would name the wrong card: refused.
    #[test]
    fn a_duplicate_of_a_file_found_in_an_earlier_sync_is_refused() {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        for folder in ["100_FUJI", "101_FUJI"] {
            write_file(&card, &format!("DCIM/{folder}/DSCF0001.RAF"), b"raw", at(1_700_000_000));
        }
        write_file(&lib, "2026-01-01/RAW/DSCF0001.RAF", b"raw", at(1_700_000_000));
        let scans = vec![scan_elsewhere(&card)];
        let mut plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
        let skipped = plan.skipped.pop().unwrap();
        let first = skipped.file.path.clone();
        let original = dir.0.join("old/DCIM/100_FUJI/DSCF0001.RAF");
        plan.seen_earlier.push(Duplicate { file: skipped.file, of: original.clone() });
        assert_eq!(plan.duplicates[0].of, first);

        let outcome = CopyOutcome::nothing_to_copy(TempCleanup::default());
        match Report::build(scans, plan, outcome, &HashSet::from([&original])) {
            Err(ReportError::NoOriginal { of, .. }) => assert_eq!(of, first),
            other => panic!("expected NoOriginal, got {other:?}"),
        }
    }
}
