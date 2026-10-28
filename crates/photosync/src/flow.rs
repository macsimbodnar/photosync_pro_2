//! One sync, start to report, as the front-ends run it: a core `Session`
//! of one sync. The session scans, refuses a source given twice, plans,
//! fixes the folder, copies and reports; every rule is in the core
//! (INV-8). Rendering is the caller's, through `Watch`.

use std::path::{Path, PathBuf};

use photosync_core::{Cancel, Report, RunEnd, Session};
pub use photosync_core::{SyncError, Watch};

pub struct SyncRequest<'a> {
    pub sources: &'a [PathBuf],
    pub library: &'a Path,
    pub name: Option<&'a str>,
}

/// A session of one sync of `request`. `Err` only for a name that is not
/// one folder name, refused before any card is read; any other refusal is
/// the session's one run.
pub fn session(
    request: &SyncRequest,
    cancel: &Cancel,
    watch: &mut dyn Watch,
) -> Result<Session, SyncError> {
    let mut session =
        Session::new(request.library, request.name).map_err(SyncError::Folder)?;
    session.sync(request.sources, cancel, watch);
    Ok(session)
}

/// Runs one sync, for a front-end that shows one report. A cancel during
/// planning gives the cancelled plan's report, as no file was written.
pub fn sync(
    request: &SyncRequest,
    cancel: &Cancel,
    watch: &mut dyn Watch,
) -> Result<Report, SyncError> {
    let mut runs = session(request, cancel, watch)?.into_runs();
    match runs.pop().map(|run| run.end) {
        Some(RunEnd::Report(report)) => Ok(report),
        Some(RunEnd::Refused(e)) => Err(e),
        // `Session::sync` records exactly one run, and never a crash.
        Some(RunEnd::Crashed(_)) | None => unreachable!("a session sync ended with no result"),
    }
}
