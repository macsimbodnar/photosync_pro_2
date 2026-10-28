//! The sync report: every file each source scan listed, with what happened
//! to it and why, and totals per source and per type (INV-6). Data only:
//! front-ends render it (INV-8).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::copy::{CopyOutcome, FailReason, TempCleanup};
use crate::plan::SyncPlan;
use crate::progress::{Counts, add};
use crate::source::{MediaType, ScanError, SourceFile, SourceScan};

/// What a sync did, per source, in the order the sources were given.
///
/// The fields are private, so a report comes only from `Report::new` or
/// `Report::plan_cancelled`, and their checks cannot be skipped:
///
/// ```compile_fail,E0451
/// let forged = photosync_core::Report {
///     sources: Vec::new(),
///     library_failures: Vec::new(),
///     temps: Default::default(),
/// };
/// ```
///
/// ```compile_fail,E0616
/// fn edit(report: &mut photosync_core::Report) {
///     report.sources.clear();
/// }
/// ```
#[derive(Debug)]
pub struct Report {
    sources: Vec<SourceReport>,
    library_failures: Vec<ScanError>,
    temps: TempCleanup,
}

#[derive(Debug)]
pub struct SourceReport {
    pub root: PathBuf,
    /// Every file the scan listed, supported and ignored, once each, in path
    /// order.
    pub files: Vec<FileReport>,
    /// Entries the scan could not read, in path order. Outside the INV-6
    /// sum: the scan never listed them as files.
    pub unreadable: Vec<Unreadable>,
}

#[derive(Debug)]
pub struct FileReport {
    pub file: SourceFile,
    pub outcome: FileOutcome,
}

/// What happened to one file, with its reason.
#[derive(Debug)]
pub enum FileOutcome {
    /// Verified and in place at `to`.
    Copied { to: PathBuf },
    /// Skipped: the library already holds it at `library_file`.
    AlreadySynced { library_file: PathBuf },
    /// Skipped: same content as `of`, a file earlier in the session, which
    /// was planned instead and is on the drive ("duplicate in session").
    DuplicateInSession { of: PathBuf },
    /// Skipped: reading this file failed with `error`, so its twin `of`, on
    /// another card, was copied instead (F07). `of` has the same name, size
    /// and content as planning read here, so the photo is on the drive; the
    /// error is kept, as this card could not be read.
    DuplicateUnread { of: PathBuf, error: std::io::Error },
    /// Failed: same content as `of`, which was planned instead and failed
    /// (its entry has the reason). This card may hold the only copy.
    DuplicateOfFailed { of: PathBuf },
    /// Cancelled: same content as `of`, which was planned instead and was
    /// cut off by the cancel.
    DuplicateOfCancelled { of: PathBuf },
    /// Not copied. A read error while planning is `FailReason::Read` too.
    Failed(FailReason),
    /// Not copied because of a cancel. Nothing is under a final name (INV-7).
    Cancelled,
    /// Not a supported type, a dot entry, or not a regular file.
    Ignored,
}

/// The report's buckets. The INV-6 sum is over all but `Ignored`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bucket {
    Copied,
    Skipped,
    Failed,
    Cancelled,
    Ignored,
}

impl FileOutcome {
    pub fn bucket(&self) -> Bucket {
        match self {
            FileOutcome::Copied { .. } => Bucket::Copied,
            FileOutcome::AlreadySynced { .. }
            | FileOutcome::DuplicateInSession { .. }
            | FileOutcome::DuplicateUnread { .. } => Bucket::Skipped,
            FileOutcome::Failed(_) | FileOutcome::DuplicateOfFailed { .. } => Bucket::Failed,
            FileOutcome::Cancelled | FileOutcome::DuplicateOfCancelled { .. } => Bucket::Cancelled,
            FileOutcome::Ignored => Bucket::Ignored,
        }
    }
}

/// An entry under `DCIM` the scan could not read.
#[derive(Debug)]
pub struct Unreadable {
    pub error: ScanError,
    /// A folder (or an entry that may be one): what it holds is unknown and
    /// not counted anywhere.
    pub contents_unknown: bool,
}

/// Files and bytes per bucket.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub copied: Counts,
    pub skipped: Counts,
    pub failed: Counts,
    pub cancelled: Counts,
    pub ignored: Counts,
    /// Of `failed`, the copies in place whose folder flush failed
    /// (`FailReason::NotFlushed`): a front-end can show them as "in place,
    /// flush not confirmed". Already in `failed`, so not in the sum again.
    pub not_flushed: Counts,
    /// Of `skipped`, the files that could not be read and were copied from
    /// a twin on another card (`FileOutcome::DuplicateUnread`): the photo
    /// is on the drive, but a front-end shows that the card had a read
    /// error. Already in `skipped`, so not in the sum again.
    pub unread: Counts,
}

impl Tally {
    /// Adds `other`'s counts to these.
    pub(crate) fn merge(&mut self, other: &Tally) {
        for (to, from) in [
            (&mut self.copied, other.copied),
            (&mut self.skipped, other.skipped),
            (&mut self.failed, other.failed),
            (&mut self.cancelled, other.cancelled),
            (&mut self.ignored, other.ignored),
            (&mut self.not_flushed, other.not_flushed),
            (&mut self.unread, other.unread),
        ] {
            add(to, from.files, from.bytes);
        }
    }

    /// copied + skipped + failed + cancelled: the supported files listed
    /// (INV-6).
    pub fn supported(&self) -> Counts {
        let mut sum = Counts::default();
        for c in [self.copied, self.skipped, self.failed, self.cancelled] {
            add(&mut sum, c.files, c.bytes);
        }
        sum
    }

    fn count(&mut self, entry: &FileReport) {
        let c = match entry.outcome.bucket() {
            Bucket::Copied => &mut self.copied,
            Bucket::Skipped => &mut self.skipped,
            Bucket::Failed => &mut self.failed,
            Bucket::Cancelled => &mut self.cancelled,
            Bucket::Ignored => &mut self.ignored,
        };
        add(c, 1, entry.file.size);
        match entry.outcome {
            FileOutcome::Failed(FailReason::NotFlushed { .. }) => add(&mut self.not_flushed, 1, entry.file.size),
            FileOutcome::DuplicateUnread { .. } => add(&mut self.unread, 1, entry.file.size),
            _ => {}
        }
    }
}

impl SourceReport {
    pub fn totals(&self) -> Tally {
        let mut tally = Tally::default();
        self.files.iter().for_each(|e| tally.count(e));
        tally
    }

    /// Totals per supported type. Ignored files have no type, so they are
    /// in `totals` only.
    pub fn by_type(&self) -> BTreeMap<MediaType, Tally> {
        let mut types = BTreeMap::new();
        add_by_type(&mut types, self);
        types
    }
}

/// Whether every supported file is on the drive and every entry of every
/// card was read: the only state in which a card is safe to format.
pub fn is_complete(totals: &Tally, sources: &[SourceReport]) -> bool {
    totals.failed.files == 0
        && totals.cancelled.files == 0
        && sources.iter().all(|s| s.unreadable.is_empty())
}

pub(crate) fn add_by_type(types: &mut BTreeMap<MediaType, Tally>, source: &SourceReport) {
    for entry in &source.files {
        if let Some(media) = entry.file.media_type {
            types.entry(media).or_default().count(entry);
        }
    }
}

/// Why a report cannot be built: the plan or outcome is not of these scans,
/// so INV-6 could not hold.
#[derive(Debug)]
pub enum ReportError {
    /// The plan or outcome lists a file the scans do not hold as a supported
    /// file, or lists it more often than they do.
    NotInScans(PathBuf),
    /// A supported file of the scans is in neither the plan nor the outcome.
    Unaccounted(PathBuf),
    /// This root names a source given earlier, in this or another spelling:
    /// its files would be listed twice.
    SourceTwice(PathBuf),
    /// The plan or outcome lists this file with another size or mtime than
    /// the scan: it describes another scan of the card.
    Changed(PathBuf),
    /// A duplicate in session whose original `of` is not copied, already
    /// synced, failed or cancelled, so nothing tells whether it is safe.
    NoOriginal { file: PathBuf, of: PathBuf },
    /// The outcome says `file` was read in place of `of`, but the plan does
    /// not list it as a twin of `of`, a file it planned to copy.
    NotATwin { file: PathBuf, of: PathBuf },
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportError::NotInScans(path) => {
                write!(f, "{} is reported but was not scanned", path.display())
            }
            ReportError::Unaccounted(path) => {
                write!(f, "{} was scanned but is not in the plan or the copy", path.display())
            }
            ReportError::SourceTwice(root) => {
                write!(f, "source {} is given twice", root.display())
            }
            ReportError::Changed(path) => {
                write!(f, "{} changed between the scan and the plan or copy", path.display())
            }
            ReportError::NoOriginal { file, of } => write!(
                f,
                "{} duplicates {}, which is not reported",
                file.display(),
                of.display()
            ),
            ReportError::NotATwin { file, of } => write!(
                f,
                "{} is reported as read instead of {}, but the plan has it as no twin of it",
                file.display(),
                of.display()
            ),
        }
    }
}

impl Error for ReportError {}

impl Report {
    /// The report of a sync: `scans` as given to `plan_sync`, its `plan`, and
    /// the `copy_plan` outcome. With nothing to copy (`sync_folder` gives
    /// `None`), pass `CopyOutcome::nothing_to_copy`. Checks that each
    /// supported file lands in exactly one entry (INV-6), as the scan listed
    /// it, and that no source is given twice.
    ///
    /// A duplicate in session is skipped only if its original is on the
    /// drive; otherwise it takes the original's bucket. A planned file the
    /// copy read from a twin instead (`CopyOutcome::fallbacks`) is a
    /// duplicate of that twin, and so are its other twins; the plan must
    /// list each twin the outcome names. One whose twin's copy is in place
    /// unflushed (`CopyOutcome::unflushed_fallbacks`) fails with its read
    /// error, the twin as not flushed, the other twins as its failed
    /// duplicates.
    pub fn new(
        scans: Vec<SourceScan>,
        plan: SyncPlan,
        outcome: CopyOutcome,
    ) -> Result<Report, ReportError> {
        Report::build(scans, plan, outcome, &HashSet::new())
    }

    /// `Report::new` within a session: `earlier` holds the files earlier
    /// syncs of the session put on the drive or found there, the only ones
    /// `plan.seen_earlier` may point to.
    pub(crate) fn build(
        scans: Vec<SourceScan>,
        plan: SyncPlan,
        outcome: CopyOutcome,
        earlier: &HashSet<&PathBuf>,
    ) -> Result<Report, ReportError> {
        let mut roots = HashSet::new();
        if let Some(twin) = scans.iter().find(|scan| !roots.insert(&scan.resolved_root)) {
            return Err(ReportError::SourceTwice(twin.root.clone()));
        }
        // Inserted safe first, so a path also listed as failed or cancelled
        // is taken as not safe.
        let mut originals = HashMap::new();
        let on_drive = plan.skipped.iter().map(|s| &s.file.path);
        let on_drive = on_drive.chain(plan.seen_earlier.iter().map(|d| &d.file.path));
        for path in on_drive.chain(outcome.copied.iter().map(|c| &c.file.path)) {
            originals.insert(path.clone(), Original::OnDrive);
        }
        let failed = plan.failures.iter().map(|e| &e.path);
        for path in failed.chain(outcome.failed.iter().map(|f| &f.file.path)) {
            originals.insert(path.clone(), Original::Failed);
        }
        for file in &outcome.cancelled {
            originals.insert(file.path.clone(), Original::Cancelled);
        }
        // F07: each planned file read from a twin, mapped to that twin, and
        // the twins the outcome reports itself, with the file they stood in
        // for. The plan must list each as a twin of that file.
        let mut read_from = HashMap::new();
        let mut stood_in = HashSet::new();
        let mut check_twins = |of: &PathBuf, files: Vec<&SourceFile>| {
            let twins = plan.to_copy.iter().find(|p| p.file.path == *of).map_or(&[][..], |p| p.twins());
            for file in files {
                if !twins.iter().any(|t| t.path == file.path) {
                    return Err(ReportError::NotATwin { file: file.path.clone(), of: of.clone() });
                }
                stood_in.insert((file.path.clone(), of.clone()));
            }
            Ok(())
        };
        for f in &outcome.fallbacks {
            let of = &f.original.path;
            check_twins(of, f.tried.iter().map(|t| &t.file).chain([&f.copied.file]).collect())?;
            read_from.insert(of.clone(), f.copied.file.path.clone());
        }
        // A twin's copy in place unflushed: the file's other duplicates
        // point to it, failed, as nothing confirms it is on the drive.
        let mut failed_over = HashMap::new();
        for f in &outcome.unflushed_fallbacks {
            let of = &f.original.path;
            check_twins(of, f.tried.iter().map(|t| &t.file).chain([&f.twin.file]).collect())?;
            failed_over.insert(of.clone(), f.twin.file.path.clone());
        }

        let mut slots = Slots::new(&scans);
        for s in plan.skipped {
            slots.put(s.file, FileOutcome::AlreadySynced { library_file: s.library_file })?;
        }
        for d in plan.seen_earlier {
            if !earlier.contains(&d.of) {
                return Err(ReportError::NoOriginal { file: d.file.path, of: d.of });
            }
            slots.put(d.file, FileOutcome::DuplicateInSession { of: d.of })?;
        }
        for d in plan.duplicates {
            if stood_in.contains(&(d.file.path.clone(), d.of.clone())) {
                continue; // the fallback's entry below reports it
            }
            if let Some(twin) = read_from.get(&d.of) {
                slots.put(d.file, FileOutcome::DuplicateInSession { of: twin.clone() })?;
                continue;
            }
            if let Some(twin) = failed_over.get(&d.of) {
                slots.put(d.file, FileOutcome::DuplicateOfFailed { of: twin.clone() })?;
                continue;
            }
            let entry = match originals.get(&d.of) {
                Some(Original::OnDrive) => FileOutcome::DuplicateInSession { of: d.of },
                Some(Original::Failed) => FileOutcome::DuplicateOfFailed { of: d.of },
                Some(Original::Cancelled) => FileOutcome::DuplicateOfCancelled { of: d.of },
                None => return Err(ReportError::NoOriginal { file: d.file.path, of: d.of }),
            };
            slots.put(d.file, entry)?;
        }
        for e in plan.failures {
            slots.put_path(e.path, FileOutcome::Failed(FailReason::Read(e.source)))?;
        }
        for c in outcome.copied {
            slots.put(c.file, FileOutcome::Copied { to: c.to })?;
        }
        for f in outcome.failed {
            slots.put(f.file, FileOutcome::Failed(f.reason))?;
        }
        for file in outcome.cancelled {
            slots.put(file, FileOutcome::Cancelled)?;
        }
        for f in outcome.fallbacks {
            let of = f.copied.file.path.clone();
            slots.put(f.original, FileOutcome::DuplicateUnread { of: of.clone(), error: f.error })?;
            for t in f.tried {
                let entry = match t.reason {
                    FailReason::Read(error) => FileOutcome::DuplicateUnread { of: of.clone(), error },
                    // Not what planning read, so not what is on the drive.
                    reason => FileOutcome::Failed(reason),
                };
                slots.put(t.file, entry)?;
            }
            slots.put(f.copied.file, FileOutcome::Copied { to: f.copied.to })?;
        }
        for f in outcome.unflushed_fallbacks {
            slots.put(f.original, FileOutcome::Failed(FailReason::Read(f.error)))?;
            for t in f.tried {
                slots.put(t.file, FileOutcome::Failed(t.reason))?;
            }
            slots.put(f.twin.file, FileOutcome::Failed(f.twin.reason))?;
        }
        let sources = slots.finish(scans)?;
        Ok(Report { sources, library_failures: plan.library_failures, temps: outcome.temps })
    }

    /// The report of a plan that was cancelled (`PlanError::Cancelled`):
    /// nothing was written, so every supported file is cancelled.
    pub fn plan_cancelled(scans: Vec<SourceScan>) -> Report {
        let sources = scans
            .into_iter()
            .map(|scan| {
                let Ok(source) = source_report(scan, |file| {
                    Ok::<_, Infallible>(match file.media_type {
                        Some(_) => FileOutcome::Cancelled,
                        None => FileOutcome::Ignored,
                    })
                });
                source
            })
            .collect();
        Report { sources, library_failures: Vec::new(), temps: TempCleanup::default() }
    }

    pub fn sources(&self) -> &[SourceReport] {
        &self.sources
    }

    /// Every supported file is on the drive and every entry was read: the
    /// cards of this sync are safe to format.
    pub fn is_complete(&self) -> bool {
        is_complete(&self.totals(), &self.sources)
    }

    /// Library entries that could not be read or hashed. They hide no source
    /// file: one whose only candidates are here was copied (DEC-015).
    pub fn library_failures(&self) -> &[ScanError] {
        &self.library_failures
    }

    /// Stale temp files removed, and temp files that could not be removed.
    pub fn temps(&self) -> &TempCleanup {
        &self.temps
    }

    /// Totals over all sources.
    pub fn totals(&self) -> Tally {
        let mut tally = Tally::default();
        self.sources.iter().flat_map(|s| &s.files).for_each(|e| tally.count(e));
        tally
    }

    /// Totals per supported type over all sources.
    pub fn by_type(&self) -> BTreeMap<MediaType, Tally> {
        let mut types = BTreeMap::new();
        self.sources.iter().for_each(|s| add_by_type(&mut types, s));
        types
    }
}

/// What became of a file a duplicate in session points to.
enum Original {
    OnDrive,
    Failed,
    Cancelled,
}

/// One outcome slot per file of each scan, found by path.
struct Slots {
    outcomes: Vec<Vec<Option<FileOutcome>>>,
    /// Supported files only, so a plan cannot claim an ignored one. A path
    /// can still be in several scans when one root is inside another's
    /// `DCIM`; each holds one slot.
    by_path: HashMap<PathBuf, Vec<Slot>>,
}

/// A supported file's place in the scans, with its size and mtime as listed.
#[derive(Clone, Copy)]
struct Slot {
    source: usize,
    file: usize,
    size: u64,
    modified: SystemTime,
}

impl Slots {
    fn new(scans: &[SourceScan]) -> Slots {
        let mut by_path: HashMap<PathBuf, Vec<Slot>> = HashMap::new();
        let outcomes = scans
            .iter()
            .enumerate()
            .map(|(s, scan)| {
                scan.files
                    .iter()
                    .enumerate()
                    .map(|(f, file)| match file.media_type {
                        Some(_) => {
                            let slot =
                                Slot { source: s, file: f, size: file.size, modified: file.modified };
                            by_path.entry(file.path.clone()).or_default().push(slot);
                            None
                        }
                        None => Some(FileOutcome::Ignored),
                    })
                    .collect()
            })
            .collect();
        Slots { outcomes, by_path }
    }

    fn put(&mut self, file: SourceFile, outcome: FileOutcome) -> Result<(), ReportError> {
        self.fill(file.path, Some((file.size, file.modified)), outcome)
    }

    /// For a plan failure, which carries a path only.
    fn put_path(&mut self, path: PathBuf, outcome: FileOutcome) -> Result<(), ReportError> {
        self.fill(path, None, outcome)
    }

    fn fill(
        &mut self,
        path: PathBuf,
        listed_as: Option<(u64, SystemTime)>,
        outcome: FileOutcome,
    ) -> Result<(), ReportError> {
        let free = self
            .by_path
            .get(&path)
            .and_then(|slots| slots.iter().find(|s| self.outcomes[s.source][s.file].is_none()));
        let Some(&slot) = free else { return Err(ReportError::NotInScans(path)) };
        if listed_as.is_some_and(|l| l != (slot.size, slot.modified)) {
            return Err(ReportError::Changed(path));
        }
        self.outcomes[slot.source][slot.file] = Some(outcome);
        Ok(())
    }

    fn finish(self, scans: Vec<SourceScan>) -> Result<Vec<SourceReport>, ReportError> {
        scans
            .into_iter()
            .zip(self.outcomes)
            .map(|(scan, outcomes)| {
                let mut outcomes = outcomes.into_iter();
                source_report(scan, |file| {
                    outcomes.next().flatten().ok_or_else(|| ReportError::Unaccounted(file.path.clone()))
                })
            })
            .collect()
    }
}

/// `scan` as a report, with `outcome` called once per listed file, in order.
fn source_report<E>(
    scan: SourceScan,
    mut outcome: impl FnMut(&SourceFile) -> Result<FileOutcome, E>,
) -> Result<SourceReport, E> {
    let files = scan
        .files
        .into_iter()
        .map(|file| Ok(FileReport { outcome: outcome(&file)?, file }))
        .collect::<Result<_, E>>()?;
    let folders = scan.failed_folders;
    let unreadable = scan
        .failures
        .into_iter()
        .map(|error| Unreadable { contents_unknown: folders.contains(&error.path), error })
        .collect();
    Ok(SourceReport { root: scan.root, files, unreadable })
}
