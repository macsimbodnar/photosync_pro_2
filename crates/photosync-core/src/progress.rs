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
    /// Events go to `observer`, in order, one call at a time. `plan_sync`
    /// and `copy_plan` call it on the calling thread. A `Session` copying
    /// several cards at once calls it from each card's copy thread too,
    /// under one lock, so calls never overlap: `Started` comes from the
    /// calling thread before any copy thread starts, and `Finished` or
    /// `Cancelled` from it once every copy thread has ended (S019).
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
/// file cut short by a cancel gets no `FileDone`. When a `Session` copies
/// several cards at once, their files' events interleave: each card keeps
/// this order and has at most one file in flight, a `Bytes` event is of
/// one of the files in flight, and each event's `progress` is a whole
/// snapshot, its sources adding up to its totals (S019).
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

/// Keeps the counts for one phase and sends each event. Several files can
/// be in flight, at most one per source (S019): a `Session` copies each
/// card on its own thread, each through this tracker behind one lock, so
/// every event is a snapshot of one moment, never half of an update.
pub(crate) struct Tracker<'m, 'a> {
    monitor: &'m mut Monitor<'a>,
    progress: Progress,
    start: Duration,
    /// The files in flight: each one's source, scanned size, bytes counted
    /// and bytes read by this read of it.
    current: Vec<Current>,
}

struct Current {
    source: Option<usize>,
    size: u64,
    counted: u64,
    read: u64,
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
        let mut tracker = Tracker { monitor, progress, start, current: Vec::new() };
        tracker.send_at(EventKind::Started, start);
        tracker
    }

    pub(crate) fn cancel(&self) -> Cancel {
        self.monitor.cancel.clone()
    }

    /// A file of `source` starts; it replaces any file of `source` still
    /// in flight, as one source has one file in flight at a time.
    pub(crate) fn file_started(&mut self, path: &Path, source: Option<usize>, size: u64) {
        self.current.retain(|c| c.source != source);
        self.current.push(Current { source, size, counted: 0, read: 0 });
        self.send(EventKind::FileStarted(path.to_path_buf()));
    }

    /// `bytes_of` the file in flight, when files go one at a time.
    pub(crate) fn bytes(&mut self, n: u64) {
        if let Some(source) = self.last() {
            self.bytes_of(source, n);
        }
    }

    /// `n` more bytes of `source`'s file in flight were read. Counted up to
    /// the file's scanned size, so a file that grew cannot push done past
    /// total.
    pub(crate) fn bytes_of(&mut self, source: Option<usize>, n: u64) {
        let Some(c) = self.current.iter_mut().find(|c| c.source == source) else { return };
        c.read = c.read.saturating_add(n);
        let now = c.read.min(c.size).max(c.counted);
        let more = now - c.counted;
        c.counted = now;
        self.count(source, 0, more);
        self.send(EventKind::Bytes);
    }

    /// `source`'s file in flight is read again from its start, from a twin
    /// (F07). It stays one file of its own source: the bytes read again
    /// count only past what the failed read counted, so done never goes
    /// back.
    pub(crate) fn reread_of(&mut self, source: Option<usize>) {
        if let Some(c) = self.current.iter_mut().find(|c| c.source == source) {
            c.read = 0;
        }
    }

    /// `file_done_of` the file in flight, when files go one at a time.
    pub(crate) fn file_done(&mut self, path: &Path) {
        let source = self.last().flatten();
        self.file_done_of(source, path);
    }

    /// `source`'s file in flight is done: whatever of its size was not
    /// read counts as done too, so done meets total at the end.
    pub(crate) fn file_done_of(&mut self, source: Option<usize>, path: &Path) {
        if let Some(i) = self.current.iter().position(|c| c.source == source) {
            let c = self.current.remove(i);
            self.count(c.source, 1, c.size - c.counted);
        }
        self.send(EventKind::FileDone(path.to_path_buf()));
    }

    /// The source of the file started last and still in flight.
    fn last(&self) -> Option<Option<usize>> {
        self.current.last().map(|c| c.source)
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
