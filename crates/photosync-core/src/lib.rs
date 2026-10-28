//! Sync logic for photosync. UI-free by contract (INV-8): front-ends get
//! everything through this API, never through stdout or stderr.

mod copy;
mod hash;
mod library;
mod plan;
mod progress;
mod source;

pub use copy::{
    Copied, CopyError, CopyOutcome, FailReason, Failed, FolderError, SyncFolder, TempCleanup,
    copy_plan, remove_stale_temps, sync_folder,
};
pub use plan::{Duplicate, PlanError, Skipped, SyncPlan, plan_sync};
pub use progress::{Cancel, Counts, Event, EventKind, Monitor, Phase, Progress, SourceProgress, eta};
pub use source::{MediaType, ScanError, SourceFile, SourceScan, scan_source};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_manifest() {
        assert_eq!(VERSION, "0.1.0");
    }
}
