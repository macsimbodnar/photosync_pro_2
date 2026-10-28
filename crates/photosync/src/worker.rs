//! Runs one `app::Job` through `flow::sync` on its own thread, so the
//! screen stays live while the core copies. Every message goes to `send`:
//! the TUI forwards it over a channel, a GUI can do the same. No terminal
//! here (`tests/ui_split.rs`).

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::thread::{self, JoinHandle};

use photosync_core::{Cancel, Event, Report, SourceScan};

use crate::app::{Job, Msg};
use crate::flow::{self, SyncError, SyncRequest, Watch};
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
/// order, then one `Finished`, or `Crashed` if the sync panicked. When the thread ends, `send` is dropped:
/// a channel's receiver then sees it gone, `Finished` or not.
pub fn spawn(job: Job, send: impl FnMut(Msg) + Send + 'static) -> JoinHandle<()> {
    spawn_with(job, send, |request, cancel, watch| {
        flow::sync(request, cancel, watch)
    })
}

/// `spawn` with the sync injected, so a test can make it panic.
fn spawn_with<F, S>(job: Job, send: F, sync: S) -> JoinHandle<()>
where
    F: FnMut(Msg) + Send + 'static,
    S: FnOnce(&SyncRequest, &Cancel, &mut Forward<F>) -> Result<Report, SyncError> + Send + 'static,
{
    thread::spawn(move || {
        let request = SyncRequest {
            sources: &job.sources,
            library: &job.library,
            name: job.name.as_deref(),
        };
        let mut watch = Forward(send);
        // The TUI's panic hook skips this thread, to keep the screen in raw
        // mode (`tui`): the panic's text reaches the user as the sync's end.
        // Unwind safety: nothing caught here is used after a panic but
        // `send`, whose channel a panic cannot leave half-written.
        let result =
            panic::catch_unwind(AssertUnwindSafe(|| sync(&request, &job.cancel, &mut watch)));
        (watch.0)(match result {
            Ok(result) => Msg::Finished(result),
            Err(payload) => Msg::Crashed(panic_text(payload.as_ref())),
        });
    })
}

/// A panic's text: `panic!` gives a `&str` or a `String`.
fn panic_text(payload: &(dyn Any + Send)) -> String {
    match (
        payload.downcast_ref::<&str>(),
        payload.downcast_ref::<String>(),
    ) {
        (Some(text), _) => text.to_string(),
        (_, Some(text)) => text.clone(),
        _ => "a panic with no message".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;

    use super::*;

    fn job() -> Job {
        Job {
            sources: vec![PathBuf::from("/V/CARD")],
            library: PathBuf::from("/V/SSD/Photos"),
            name: None,
            cancel: Cancel::new(),
        }
    }

    /// S018: a panic in the sync is the sync's end, with its text, not a
    /// silent death: the TUI's panic hook keeps it off the screen.
    #[test]
    fn a_panic_in_the_sync_is_sent_as_crashed() {
        let (tx, rx) = mpsc::channel();
        let handle = spawn_with(
            job(),
            move |m| tx.send(m).unwrap(),
            |_, _, _| panic!("index out of bounds"),
        );
        let got: Vec<Msg> = rx.iter().collect();
        assert!(handle.join().is_ok(), "the thread itself must not die");
        match &got[..] {
            [Msg::Crashed(text)] => assert!(text.contains("index out of bounds"), "{text}"),
            other => panic!("{} messages, not one Crashed", other.len()),
        }
    }
}
