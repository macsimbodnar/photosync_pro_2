//! A session as the front-ends run it: one core `Session` for the whole
//! run of the app, one card after another. The session scans, refuses a
//! source given twice, plans, fixes the folder, copies, reports and keeps
//! the worst outcome; every rule is in the core (INV-8). Rendering is the
//! caller's, through `Watch`.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use photosync_core::Session;
pub use photosync_core::{SyncError, Watch};

/// The interactive model keeps the session between cards and lends it to
/// the worker thread for each sync.
pub type Shared = Arc<Mutex<Session>>;

/// A session into `library`. `Err` only for a name that is not one folder
/// name, refused before any card is read; every other refusal is a run of
/// the session.
pub fn start(library: &Path, name: Option<&str>) -> Result<Session, SyncError> {
    Session::new(library, name).map_err(SyncError::Folder)
}

/// The session, also after a sync panicked while holding it: the session
/// stays whole (it marks its cleanup done before running it, and records
/// a run only at its end), and the crash is then recorded in it.
pub fn lock(shared: &Shared) -> MutexGuard<'_, Session> {
    shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
