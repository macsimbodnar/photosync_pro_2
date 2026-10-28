//! Runs one `app::Job` through `flow::sync` on its own thread, so the
//! screen stays live while the core copies. Every message goes to `send`:
//! the TUI forwards it over a channel, a GUI can do the same. No terminal
//! here (`tests/ui_split.rs`).

use std::thread::{self, JoinHandle};

use photosync_core::{Event, SourceScan};

use crate::app::{Job, Msg};
use crate::flow::{self, SyncRequest, Watch};
use crate::text;

struct Forward<F>(F);

impl<F: FnMut(Msg) + Send> Watch for Forward<F> {
    fn scanned(&mut self, scan: &SourceScan) {
        (self.0)(Msg::Scanned(text::scanned(scan)));
    }

    fn event(&mut self, event: Event) {
        (self.0)(Msg::Progress(event));
    }
}

/// Starts the sync. `send` gets `Scanned` and `Progress` messages in
/// order, then one `Finished`. When the thread ends, `send` is dropped:
/// a channel's receiver then sees it gone, `Finished` or not.
pub fn spawn(job: Job, send: impl FnMut(Msg) + Send + 'static) -> JoinHandle<()> {
    thread::spawn(move || {
        let request = SyncRequest {
            sources: &job.sources,
            library: &job.library,
            name: job.name.as_deref(),
        };
        let mut watch = Forward(send);
        let result = flow::sync(&request, &job.cancel, &mut watch);
        (watch.0)(Msg::Finished(result));
    })
}
