//! Sync logic for photosync. UI-free by contract (INV-8): front-ends get
//! everything through this API, never through stdout or stderr.

mod copy;
mod hash;
mod library;
mod plan;
mod progress;
mod report;
mod session;
mod source;
mod volume;

// One instance for all unit tests: `TempDir` numbers its folders from a
// static, and two copies of it would hand out the same paths.
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod common;
// Lets `common` name this crate as the integration tests do.
#[cfg(test)]
extern crate self as photosync_core;

pub use copy::{
    Copied, CopyError, CopyOutcome, FailReason, Failed, Fallback, FolderError, MAX_NAME_BYTES,
    SyncFolder, TempCleanup, UnflushedFallback, copy_plan, remove_stale_temps, sync_folder,
};
pub use plan::{Duplicate, PlanError, Planned, Skipped, SyncPlan, check_library_found, plan_sync};
pub use progress::{Cancel, Counts, Event, EventKind, Monitor, Phase, Progress, SourceProgress, eta};
pub use report::{
    Bucket, FileOutcome, FileReport, Report, ReportError, SourceReport, Tally, Unreadable,
    is_complete,
};
pub use session::{Run, RunEnd, Session, SyncError, Verdict, Watch};
pub use source::{MediaType, ScanError, SourceFile, SourceScan, scan_source};
pub use volume::{Volumes, default_search_roots, discover_volumes};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_manifest() {
        assert_eq!(VERSION, "0.1.0");
    }
}
