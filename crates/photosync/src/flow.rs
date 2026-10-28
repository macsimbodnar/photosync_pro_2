//! One sync, start to report, as the front-ends run it: scan the sources,
//! refuse a source given twice, plan, fix the folder, copy, report. Only
//! calls into the core, which holds every rule (INV-8); rendering is the
//! caller's, through `Watch`, so the TUI (S009) can reuse this as is.

use std::path::{Path, PathBuf};

use photosync_core::{
    Cancel, CopyError, CopyOutcome, Event, FolderError, Monitor, PlanError, Report, ReportError,
    ScanError, SourceScan, copy_plan, plan_sync, remove_stale_temps, scan_source, sync_folder,
};

pub struct SyncRequest<'a> {
    pub sources: &'a [PathBuf],
    pub library: &'a Path,
    pub name: Option<&'a str>,
}

/// What a front-end is told while a sync runs, on the calling thread.
pub trait Watch {
    /// A source was scanned, before anything is planned.
    fn scanned(&mut self, scan: &SourceScan);
    /// Planning or copying progress.
    fn event(&mut self, event: Event);
}

/// Why a sync gave no report. Every variant but `Report` stopped before
/// anything was written.
#[derive(Debug)]
pub enum SyncError {
    /// A source could not be scanned: it is gone, or has no readable `DCIM`.
    Scan(ScanError),
    /// `again` is `first` in another spelling (or the same).
    SourceTwice {
        first: PathBuf,
        again: PathBuf,
    },
    Plan(PlanError),
    Folder(FolderError),
    Copy(CopyError),
    /// The copy ran but its report was refused, so nothing tells which
    /// files are safe (DEC-022 (5)).
    Report(ReportError),
}

/// Runs one sync. A cancel during planning gives the cancelled plan's
/// report, as no file was written (`Report::plan_cancelled`).
pub fn sync(
    request: &SyncRequest,
    cancel: &Cancel,
    watch: &mut (impl Watch + Send),
) -> Result<Report, SyncError> {
    let mut scans: Vec<SourceScan> = Vec::new();
    for root in request.sources {
        let scan = scan_source(root).map_err(SyncError::Scan)?;
        // Before planning: a twin would plan, and copy, every file twice.
        if let Some(first) = scans.iter().find(|s| s.resolved_root == scan.resolved_root) {
            return Err(SyncError::SourceTwice {
                first: first.root.clone(),
                again: scan.root,
            });
        }
        watch.scanned(&scan);
        scans.push(scan);
    }
    let planned = {
        let mut monitor = Monitor::new(cancel.clone(), |e| watch.event(e));
        plan_sync(request.library, &scans, &mut monitor)
    };
    let plan = match planned {
        Ok(plan) => plan,
        Err(PlanError::Cancelled) => return Ok(Report::plan_cancelled(scans)),
        Err(e) => return Err(SyncError::Plan(e)),
    };
    let outcome = match sync_folder(&plan, request.name).map_err(SyncError::Folder)? {
        Some(folder) => {
            let mut monitor = Monitor::new(cancel.clone(), |e| watch.event(e));
            copy_plan(&plan, &folder, &mut monitor).map_err(SyncError::Copy)?
        }
        // DEC-022 (2): `copy_plan` needs a folder, so the cleanup runs here.
        None => CopyOutcome::nothing_to_copy(remove_stale_temps(&plan.library)),
    };
    Report::new(scans, plan, outcome).map_err(SyncError::Report)
}
