//! The sync report: every file each source scan listed, with what happened
//! to it and why, and totals per source and per type (INV-6). Data only:
//! front-ends render it (INV-8).

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use crate::copy::{CopyOutcome, FailReason, TempCleanup};
use crate::plan::SyncPlan;
use crate::progress::{Counts, add};
use crate::source::{MediaType, ScanError, SourceFile, SourceScan};

/// What a sync did, per source, in the order the sources were given.
#[derive(Debug)]
pub struct Report {
    pub sources: Vec<SourceReport>,
    /// Library entries that could not be read or hashed. They hide no source
    /// file: one whose only candidates are here was copied (DEC-015).
    pub library_failures: Vec<ScanError>,
    /// Stale temp files removed, and temp files that could not be removed.
    pub temps: TempCleanup,
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
    /// was planned instead ("duplicate in session").
    DuplicateInSession { of: PathBuf },
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
            FileOutcome::AlreadySynced { .. } | FileOutcome::DuplicateInSession { .. } => {
                Bucket::Skipped
            }
            FileOutcome::Failed(_) => Bucket::Failed,
            FileOutcome::Cancelled => Bucket::Cancelled,
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
}

impl Tally {
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

fn add_by_type(types: &mut BTreeMap<MediaType, Tally>, source: &SourceReport) {
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
        }
    }
}

impl Error for ReportError {}

impl Report {
    /// The report of a sync: `scans` as given to `plan_sync`, its `plan`, and
    /// the `copy_plan` outcome. With nothing to copy (`sync_folder` gives
    /// `None`), pass `CopyOutcome::nothing_to_copy`. Checks that each
    /// supported file lands in exactly one entry (INV-6).
    pub fn new(
        scans: Vec<SourceScan>,
        plan: SyncPlan,
        outcome: CopyOutcome,
    ) -> Result<Report, ReportError> {
        let mut slots = Slots::new(&scans);
        for s in plan.skipped {
            slots.put(s.file.path, FileOutcome::AlreadySynced { library_file: s.library_file })?;
        }
        for d in plan.duplicates {
            slots.put(d.file.path, FileOutcome::DuplicateInSession { of: d.of })?;
        }
        for e in plan.failures {
            slots.put(e.path, FileOutcome::Failed(FailReason::Read(e.source)))?;
        }
        for c in outcome.copied {
            slots.put(c.file.path, FileOutcome::Copied { to: c.to })?;
        }
        for f in outcome.failed {
            slots.put(f.file.path, FileOutcome::Failed(f.reason))?;
        }
        for file in outcome.cancelled {
            slots.put(file.path, FileOutcome::Cancelled)?;
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

/// One outcome slot per file of each scan, found by path.
struct Slots {
    outcomes: Vec<Vec<Option<FileOutcome>>>,
    /// Supported files only, so a plan cannot claim an ignored one. A path
    /// can be in several scans (a source given twice); each holds one slot.
    by_path: HashMap<PathBuf, Vec<(usize, usize)>>,
}

impl Slots {
    fn new(scans: &[SourceScan]) -> Slots {
        let mut by_path: HashMap<PathBuf, Vec<(usize, usize)>> = HashMap::new();
        let outcomes = scans
            .iter()
            .enumerate()
            .map(|(s, scan)| {
                scan.files
                    .iter()
                    .enumerate()
                    .map(|(f, file)| match file.media_type {
                        Some(_) => {
                            by_path.entry(file.path.clone()).or_default().push((s, f));
                            None
                        }
                        None => Some(FileOutcome::Ignored),
                    })
                    .collect()
            })
            .collect();
        Slots { outcomes, by_path }
    }

    fn put(&mut self, path: PathBuf, outcome: FileOutcome) -> Result<(), ReportError> {
        let free = self
            .by_path
            .get(&path)
            .and_then(|slots| slots.iter().find(|(s, f)| self.outcomes[*s][*f].is_none()));
        let Some(&(s, f)) = free else { return Err(ReportError::NotInScans(path)) };
        self.outcomes[s][f] = Some(outcome);
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
