//! Runs one `app::Job`, one sync of the job's session, on its own thread,
//! so the screen stays live while the core copies. Every message goes to
//! `send`: the TUI forwards it over a channel, a GUI can do the same. No
//! terminal here (`tests/ui_split.rs`).

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use photosync_core::{Cancel, Event, Session, SourceScan};

use crate::app::{Job, Msg};
use crate::flow::{self, Watch};
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
/// order, then one `Finished` (the sync's end is the session's last run),
/// or `Crashed` if the sync panicked, which the model records in the
/// session. Either is sent after the session is unlocked. When the thread
/// ends, `send` is dropped: a channel's receiver then sees it gone,
/// `Finished` or not.
pub fn spawn(job: Job, send: impl FnMut(Msg) + Send + 'static) -> JoinHandle<()> {
    spawn_with(job, send, |session, sources, cancel, watch| {
        session.sync(sources, cancel, watch);
    })
}

/// `spawn` with the sync injected, so a test can make it panic, or end
/// it as no real sync does (`Session::record_refusal`, DEC-035).
pub(crate) fn spawn_with<F, S>(job: Job, send: F, sync: S) -> JoinHandle<()>
where
    F: FnMut(Msg) + Send + 'static,
    S: FnOnce(&mut Session, &[PathBuf], &Cancel, &mut dyn Watch) + Send + 'static,
{
    thread::spawn(move || {
        let mut watch = Forward(send);
        // The TUI's panic hook skips this thread, to keep the screen in raw
        // mode (`tui`): the panic's text reaches the user as the sync's end.
        // Unwind safety: after a panic only `send` is used here, whose
        // channel a panic cannot leave half-written; the session is
        // unlocked while unwinding, and `flow::lock` says why it is whole.
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut session = flow::lock(&job.session);
            sync(&mut session, &job.sources, &job.cancel, &mut watch)
        }));
        (watch.0)(match result {
            Ok(()) => Msg::Finished,
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
    use std::path::Path;
    use std::sync::{Arc, Mutex, mpsc};

    use photosync_core::{RunEnd, SyncError};

    use super::*;

    fn job() -> Job {
        let session = Session::new(Path::new("/V/SSD/Photos"), None).unwrap();
        Job {
            session: Arc::new(Mutex::new(session)),
            sources: vec![PathBuf::from("/V/CARD")],
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
            |_, _, _, _| panic!("index out of bounds"),
        );
        let got: Vec<Msg> = rx.iter().collect();
        assert!(handle.join().is_ok(), "the thread itself must not die");
        match &got[..] {
            [Msg::Crashed(text)] => assert!(text.contains("index out of bounds"), "{text}"),
            other => panic!("{} messages, not one Crashed", other.len()),
        }
    }

    /// The sync runs on the job's session, which holds its end once
    /// `Finished` comes; after a panic the session is still there for the
    /// model to record the crash in.
    #[test]
    fn the_sync_runs_in_the_jobs_session_which_outlives_a_panic() {
        let job = job();
        let session = job.session.clone();
        let (tx, rx) = mpsc::channel();
        spawn(job, move |m| tx.send(m).unwrap()).join().unwrap();
        assert!(matches!(rx.iter().last(), Some(Msg::Finished)));
        assert!(matches!(
            flow::lock(&session).runs(),
            [run] if matches!(run.end, RunEnd::Refused(SyncError::Scan(_)))
        ));

        let job = Job {
            session: session.clone(),
            ..self::job()
        };
        spawn_with(job, |_| {}, |_, _, _, _| panic!("boom"))
            .join()
            .unwrap();
        let mut after = flow::lock(&session);
        after.record_crash(&[PathBuf::from("/V/CARD")], "boom".into());
        assert_eq!(after.runs().len(), 2);
    }
}
