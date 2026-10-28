//! Progress events and cancel for the two long phases of a sync: planning
//! (hashing the already-synced candidates) and copying. The core reports
//! through an observer and never renders anything (INV-8); a front-end may
//! render in place or forward each event over a channel to another thread.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Asks a running `plan_sync` or `copy_plan` to stop. Clones share one flag,
/// so a front-end keeps one clone and hands another to the sync, on any
/// thread. Checked between files and before each chunk read.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// What a sync phase reports to and listens to: an observer for events, a
/// cancel flag and a clock. `Send`, so a GUI can run the sync on a worker
/// thread.
pub struct Monitor<'a> {
    observer: Box<dyn FnMut(Event) + Send + 'a>,
    pub(crate) cancel: Cancel,
    clock: Box<dyn FnMut() -> Duration + Send + 'a>,
}

impl<'a> Monitor<'a> {
    /// Events go to `observer`, in order, on the calling thread.
    pub fn new(cancel: Cancel, observer: impl FnMut(Event) + Send + 'a) -> Monitor<'a> {
        Monitor { observer: Box::new(observer), cancel, clock: real_clock() }
    }

    /// Replaces the clock: any monotonic time, read once per event. Elapsed
    /// time is measured from its reading at a phase's start, so tests need
    /// no real time.
    pub fn with_clock(mut self, clock: impl FnMut() -> Duration + Send + 'a) -> Monitor<'a> {
        self.clock = Box::new(clock);
        self
    }
}

/// No observer, a cancel nobody holds, the real clock.
impl Default for Monitor<'_> {
    fn default() -> Self {
        Monitor::new(Cancel::new(), |_| {})
    }
}

fn real_clock() -> Box<dyn FnMut() -> Duration + Send> {
    let origin = Instant::now();
    Box::new(move || origin.elapsed())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// `plan_sync` hashing source files that have a name + size candidate.
    Planning,
    /// `copy_plan` copying the plan's new files.
    Copying,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    /// The state right after this event.
    pub progress: Progress,
}

/// Per phase, in this order: `Started`, then for each file `FileStarted`,
/// any number of `Bytes`, `FileDone`; then `Finished` or `Cancelled`. A
/// file cut short by a cancel gets no `FileDone`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// The totals are known; nothing is done yet.
    Started,
    FileStarted(PathBuf),
    /// One more chunk of the current file was read.
    Bytes,
    /// The file is done, whatever its outcome (the return value says which).
    FileDone(PathBuf),
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,
    /// Overall. A file that failed counts as done with its whole size.
    pub done: Counts,
    pub total: Counts,
    /// One entry per source, in the order the sources were given.
    pub sources: Vec<SourceProgress>,
    /// Since the phase started.
    pub elapsed: Duration,
    /// `None` until some bytes are done.
    pub eta: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgress {
    pub root: PathBuf,
    pub done: Counts,
    pub total: Counts,
}

/// Time left at the rate so far: `elapsed * (total - done) / done`. `None`
/// while nothing is done, as there is no rate yet.
pub fn eta(done_bytes: u64, total_bytes: u64, elapsed: Duration) -> Option<Duration> {
    if done_bytes == 0 {
        return None;
    }
    let left = u128::from(total_bytes.saturating_sub(done_bytes));
    let nanos = elapsed.as_nanos().saturating_mul(left) / u128::from(done_bytes);
    // Beyond u64 nanoseconds is centuries: the cap is as good as exact.
    Some(Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX)))
}

/// The index of the source whose root holds `path`, the deepest if roots
/// nest.
pub(crate) fn source_of(roots: &[PathBuf], path: &Path) -> Option<usize> {
    (0..roots.len())
        .filter(|i| path.starts_with(&roots[*i]))
        .max_by_key(|i| roots[*i].components().count())
}

/// Keeps the counts for one phase and sends each event.
pub(crate) struct Tracker<'m, 'a> {
    monitor: &'m mut Monitor<'a>,
    progress: Progress,
    start: Duration,
    /// The current file: its source, scanned size and bytes read so far.
    current: Option<(Option<usize>, u64, u64)>,
}

impl<'m, 'a> Tracker<'m, 'a> {
    /// Sends `Started`. `files` is each file the phase will handle, as its
    /// source index and size.
    pub(crate) fn start(
        monitor: &'m mut Monitor<'a>,
        phase: Phase,
        roots: &[PathBuf],
        files: impl IntoIterator<Item = (Option<usize>, u64)>,
    ) -> Tracker<'m, 'a> {
        let mut sources: Vec<SourceProgress> = roots
            .iter()
            .map(|root| SourceProgress { root: root.clone(), done: Counts::default(), total: Counts::default() })
            .collect();
        let mut total = Counts::default();
        for (source, size) in files {
            add(&mut total, 1, size);
            if let Some(s) = source {
                add(&mut sources[s].total, 1, size);
            }
        }
        let start = (monitor.clock)();
        let progress =
            Progress { phase, done: Counts::default(), total, sources, elapsed: Duration::ZERO, eta: None };
        let mut tracker = Tracker { monitor, progress, start, current: None };
        tracker.send_at(EventKind::Started, start);
        tracker
    }

    pub(crate) fn cancel(&self) -> Cancel {
        self.monitor.cancel.clone()
    }

    pub(crate) fn file_started(&mut self, path: &Path, source: Option<usize>, size: u64) {
        self.current = Some((source, size, 0));
        self.send(EventKind::FileStarted(path.to_path_buf()));
    }

    /// `n` more bytes of the current file were read. Counted up to the
    /// file's scanned size, so a file that grew cannot push done past total.
    pub(crate) fn bytes(&mut self, n: u64) {
        let Some((source, size, read)) = self.current.as_mut() else { return };
        let before = (*read).min(*size);
        *read = read.saturating_add(n);
        let more = (*read).min(*size) - before;
        let source = *source;
        self.count(source, 0, more);
        self.send(EventKind::Bytes);
    }

    /// The current file is done: whatever of its size was not read counts
    /// as done too, so done meets total at the end.
    pub(crate) fn file_done(&mut self, path: &Path) {
        if let Some((source, size, read)) = self.current.take() {
            self.count(source, 1, size - read.min(size));
        }
        self.send(EventKind::FileDone(path.to_path_buf()));
    }

    pub(crate) fn finish(mut self, cancelled: bool) {
        self.send(if cancelled { EventKind::Cancelled } else { EventKind::Finished });
    }

    fn count(&mut self, source: Option<usize>, files: u64, bytes: u64) {
        add(&mut self.progress.done, files, bytes);
        if let Some(s) = source {
            add(&mut self.progress.sources[s].done, files, bytes);
        }
    }

    fn send(&mut self, kind: EventKind) {
        let now = (self.monitor.clock)();
        self.send_at(kind, now);
    }

    fn send_at(&mut self, kind: EventKind, now: Duration) {
        let p = &mut self.progress;
        p.elapsed = now.saturating_sub(self.start);
        p.eta = eta(p.done.bytes, p.total.bytes, p.elapsed);
        (self.monitor.observer)(Event { kind, progress: p.clone() });
    }
}

pub(crate) fn add(counts: &mut Counts, files: u64, bytes: u64) {
    counts.files += files;
    counts.bytes = counts.bytes.saturating_add(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eta_is_elapsed_times_remaining_over_done() {
        let s = Duration::from_secs;
        assert_eq!(eta(0, 100, s(10)), None);
        assert_eq!(eta(0, 0, s(10)), None);
        assert_eq!(eta(25, 100, s(10)), Some(s(30)));
        assert_eq!(eta(50, 100, s(10)), Some(s(10)));
        assert_eq!(eta(100, 100, s(10)), Some(Duration::ZERO));
        assert_eq!(eta(150, 100, s(10)), Some(Duration::ZERO));
        assert_eq!(eta(1, 3, Duration::from_nanos(1)), Some(Duration::from_nanos(2)));
        assert_eq!(eta(1, u64::MAX, s(u64::MAX)), Some(Duration::from_nanos(u64::MAX)));
    }

    #[test]
    fn source_of_picks_the_deepest_root_holding_the_path() {
        let roots = [PathBuf::from("/v/a"), PathBuf::from("/v/a/b"), PathBuf::from("/v/c")];
        assert_eq!(source_of(&roots, Path::new("/v/a/DCIM/x.RAF")), Some(0));
        assert_eq!(source_of(&roots, Path::new("/v/a/b/DCIM/x.RAF")), Some(1));
        assert_eq!(source_of(&roots, Path::new("/v/c/DCIM/x.RAF")), Some(2));
        assert_eq!(source_of(&roots, Path::new("/v/ab/DCIM/x.RAF")), None);
    }

    #[test]
    fn bytes_count_up_to_the_scanned_size_and_a_done_file_counts_whole() {
        let mut events = Vec::new();
        let mut monitor = Monitor::new(Cancel::new(), |e: Event| events.push(e.progress));
        let roots = [PathBuf::from("/card")];
        let mut t = Tracker::start(&mut monitor, Phase::Copying, &roots, [(Some(0), 10), (None, 4)]);
        t.file_started(Path::new("/card/a"), Some(0), 10);
        t.bytes(6);
        t.bytes(6); // the file grew since the scan
        t.file_done(Path::new("/card/a"));
        t.file_started(Path::new("/x/b"), None, 4);
        t.bytes(1);
        t.file_done(Path::new("/x/b")); // failed after one byte
        t.finish(false);
        drop(monitor);

        let c = |files, bytes| Counts { files, bytes };
        let done: Vec<Counts> = events.iter().map(|p| p.done).collect();
        assert_eq!(
            done,
            [c(0, 0), c(0, 0), c(0, 6), c(0, 10), c(1, 10), c(1, 10), c(1, 11), c(2, 14), c(2, 14)]
        );
        let last = events.last().unwrap();
        assert_eq!(last.total, c(2, 14));
        // The file under no root counts overall only.
        assert_eq!(last.sources, [SourceProgress { root: roots[0].clone(), done: c(1, 10), total: c(1, 10) }]);
    }
}
