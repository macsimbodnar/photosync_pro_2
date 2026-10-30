//! S019: the cards of one sync copied at once, one thread per card. Each
//! test meets the copy threads at a stage of the shared hook, so what runs
//! at the same time is fixed by the test, not left to timing. A meeting
//! that does not happen fails its test after 10 s instead of hanging it.

use std::collections::{BTreeMap, HashSet};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Condvar;
use std::thread::ThreadId;
use std::time::Duration;

use super::*;
use crate::common::{TempDir, at, manifest, scan_card, write_file};
use crate::progress::{Event, EventKind, Phase};
use crate::report::{FileOutcome, Report};
use crate::session::{RunEnd, Session, SyncError, Verdict, Watch};
use crate::source::SourceScan;

const T0: u64 = 1_761_650_000;

fn put(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    write_file(root, rel, bytes, at(T0))
}

/// Two chunks, so a thread is still reading its file after the first.
fn big(seed: u8) -> Vec<u8> {
    (0..CHUNK + CHUNK / 2).map(|i| (i % 251) as u8 ^ seed).collect()
}

/// `n` threads meet: each waits until all have come.
struct Meet {
    n: usize,
    came: Mutex<usize>,
    all: Condvar,
}

impl Meet {
    fn new(n: usize) -> Arc<Meet> {
        Arc::new(Meet { n, came: Mutex::new(0), all: Condvar::new() })
    }

    /// `last` runs in the last thread to come, before any goes on. False
    /// if they have not all come within 10 s.
    fn meet(&self, last: impl FnOnce()) -> bool {
        let mut came = self.came.lock().unwrap();
        *came += 1;
        if *came == self.n {
            last();
            self.all.notify_all();
            return true;
        }
        let wait = self.all.wait_timeout_while(came, Duration::from_secs(10), |c| *c < self.n);
        !wait.unwrap().1.timed_out()
    }
}

/// Sets the shared hook on this thread, which the copy threads it starts
/// take on; cleared on drop.
struct Hooked;

fn hook(f: impl Fn(Stage, &Path) + Send + Sync + 'static) -> Hooked {
    SHARED_HOOK.with(|h| *h.borrow_mut() = Some(Arc::new(f)));
    Hooked
}

impl Drop for Hooked {
    fn drop(&mut self) {
        SHARED_HOOK.with(|h| *h.borrow_mut() = None);
    }
}

/// Every stage the hook saw, with its thread.
type Seen = Arc<Mutex<Vec<(ThreadId, Stage, PathBuf)>>>;

/// Records the stage, then says whether it is this path's first time.
fn first_time(seen: &Seen, stage: Stage, path: &Path) -> bool {
    let mut seen = seen.lock().unwrap();
    seen.push((thread::current().id(), stage, path.to_path_buf()));
    seen.iter().filter(|(_, s, p)| *s == stage && p == path).count() == 1
}

fn named(path: &Path, name: &str) -> bool {
    path.to_string_lossy().ends_with(name)
}

/// Every event; the copy threads call it one at a time.
#[derive(Default)]
struct Log(Vec<Event>);

impl Watch for Log {
    fn event(&mut self, event: Event) {
        self.0.push(event);
    }
}

/// Each card on a fake device of its own: one copy thread each.
fn on_own_cards(cards: &[&Path]) -> Vec<SourceScan> {
    cards.iter().zip(1..).map(|(c, n)| scan_card(c, n)).collect()
}

fn last_report(session: &Session) -> &Report {
    match &session.runs().last().expect("a run").end {
        RunEnd::Report(report) => report,
        other => panic!("no report: {other:?}"),
    }
}

fn outcome<'r>(report: &'r Report, path: &Path) -> &'r FileOutcome {
    let mut entries = report.sources().iter().flat_map(|s| &s.files);
    &entries.find(|e| e.file.path == path).expect("listed").outcome
}

/// INV-6 per card: each scanned file listed once, and the buckets add up.
fn assert_inv6(report: &Report, cards: &[&Path]) {
    assert_eq!(report.sources().len(), cards.len());
    for (source, card) in report.sources().iter().zip(cards) {
        let fresh = scan_card(card, 1);
        assert_eq!(source.totals().supported().files, fresh.supported().count() as u64, "{card:?}");
        let listed: Vec<&Path> = source.files.iter().map(|e| e.file.path.as_path()).collect();
        let scanned: Vec<&Path> = fresh.files.iter().map(|f| f.path.as_path()).collect();
        assert_eq!(listed, scanned, "{card:?}: each file once");
    }
}

/// The files under `root`, with their bytes.
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    manifest(root).into_iter().filter_map(|(p, (_, bytes))| bytes.map(|b| (p, b))).collect()
}

fn temps_in(root: &Path) -> Vec<PathBuf> {
    let temp = |p: &PathBuf| p.file_name().unwrap().to_string_lossy().starts_with(TEMP_PREFIX);
    files(root).into_keys().filter(temp).collect()
}

/// INV-2: every file there before is there after, with the same bytes.
fn assert_kept(before: &BTreeMap<PathBuf, Vec<u8>>, root: &Path) {
    let after = files(root);
    for (path, bytes) in before {
        assert_eq!(after.get(path), Some(bytes), "{path:?} changed or went");
    }
}

/// Two cards are read at the same time, each on its own thread: both
/// threads are inside their first file's read at the meeting. Progress
/// comes in one stream, each event a whole snapshot: the cards' counts add
/// up to the totals, nothing goes back, and each card has at most one file
/// in flight, both at once at the meeting. INV-1, INV-2, INV-3 and INV-6
/// hold.
#[test]
fn s019_two_cards_are_read_at_once_with_progress_per_card_in_one_stream() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let (raw_a, raw_b) = (big(1), big(2));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", &raw_a);
    put(&a, "DCIM/100_FUJI/DSCF0002.JPG", b"a jpeg");
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &raw_b);
    put(&b, "DCIM/100_FUJI/DSCF0102.JPG", b"b jpeg");
    put(&lib, "2026-01-01/RAW/DSCF0001.RAF", b"an older photo, same name");
    let cards = (manifest(&a), manifest(&b));
    let library = files(&lib);

    let (meet, seen) = (Meet::new(2), Seen::default());
    let log = seen.clone();
    let _hook = hook(move |stage, path| {
        if first_time(&log, stage, path) && stage == Stage::Written && named(path, ".RAF") {
            assert!(meet.meet(|| {}), "the two cards were not read at the same time");
        }
    });
    let mut events = Log::default();
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut events);

    let report = last_report(&session);
    assert!(report.is_complete(), "{report:?}");
    assert_inv6(report, &[&a, &b]);
    assert_eq!((manifest(&a), manifest(&b)), cards, "INV-1");
    assert_kept(&library, &lib);
    let folder = session.folder().unwrap().path();
    assert_eq!(fs::read(folder.join("RAW/DSCF0001.RAF")).unwrap(), raw_a);
    assert_eq!(fs::read(folder.join("RAW/DSCF0101.RAF")).unwrap(), raw_b);
    assert_eq!(fs::read(folder.join("JPEG/DSCF0002.JPG")).unwrap(), b"a jpeg");
    assert_eq!(fs::read(folder.join("JPEG/DSCF0102.JPG")).unwrap(), b"b jpeg");
    assert!(temps_in(&lib).is_empty());

    let seen = seen.lock().unwrap();
    let threads: HashSet<ThreadId> =
        seen.iter().filter(|(_, s, _)| *s == Stage::Written).map(|(t, _, _)| *t).collect();
    assert_eq!(threads.len(), 2, "one thread per card");
    assert!(!threads.contains(&thread::current().id()));
    // INV-3 in each thread: written, flushed, its folder flushed, verified,
    // renamed under the folder's lock, the new name flushed; file by file.
    use Stage::*;
    let one = [Written, FileSynced, DirSynced, Verified, RenameLocked, RenameUnlocking, Renamed, RenameSynced];
    for thread in threads {
        let mut steps: Vec<Stage> = seen
            .iter()
            .filter(|(t, s, _)| *t == thread && !matches!(s, Checking | FolderReady))
            .map(|(_, s, _)| *s)
            .collect();
        steps.dedup_by(|x, y| *x == Written && *y == Written);
        assert_eq!(steps, [one, one].concat(), "{thread:?}");
    }

    let copying: Vec<&Event> = events.0.iter().filter(|e| e.progress.phase == Phase::Copying).collect();
    let (first, last) = (copying[0], copying[copying.len() - 1]);
    assert!(matches!(first.kind, EventKind::Started) && matches!(last.kind, EventKind::Finished));
    let card_of = |p: &Path| [&a, &b].iter().position(|c| p.starts_with(c)).unwrap();
    let (mut in_flight, mut most) = (Vec::new(), 0);
    for (i, e) in copying.iter().enumerate() {
        let p = &e.progress;
        let sum = |f: fn(&crate::SourceProgress) -> u64| p.sources.iter().map(f).sum::<u64>();
        assert_eq!((sum(|s| s.done.files), sum(|s| s.done.bytes)), (p.done.files, p.done.bytes), "torn: {e:?}");
        assert_eq!((sum(|s| s.total.files), sum(|s| s.total.bytes)), (p.total.files, p.total.bytes));
        if i > 0 {
            let q = &copying[i - 1].progress;
            assert_eq!(p.total, q.total, "total never changes");
            assert!(p.done.files >= q.done.files && p.done.bytes >= q.done.bytes, "done went back");
        }
        match &e.kind {
            EventKind::FileStarted(path) => {
                assert!(!in_flight.contains(&card_of(path)), "two files of one card at once");
                in_flight.push(card_of(path));
                most = most.max(in_flight.len());
            }
            EventKind::FileDone(path) => in_flight.retain(|c| *c != card_of(path)),
            _ => {}
        }
    }
    assert_eq!(most, 2, "both cards had a file in flight at once");
    assert_eq!(last.progress.done, last.progress.total);
    for s in &last.progress.sources {
        assert_eq!(s.done, s.total, "{s:?}");
    }
}

/// The most files in flight at once. Fails if one card ever has two, cards
/// told apart by `card_of`.
fn most_in_flight(events: &Log, card_of: impl Fn(&Path) -> u8) -> usize {
    let (mut in_flight, mut most) = (Vec::new(), 0);
    for e in events.0.iter().filter(|e| e.progress.phase == Phase::Copying) {
        match &e.kind {
            EventKind::FileStarted(path) => {
                assert!(!in_flight.contains(&card_of(path)), "two files of one card at once: {path:?}");
                in_flight.push(card_of(path));
                most = most.max(in_flight.len());
            }
            EventKind::FileDone(path) => in_flight.retain(|c| *c != card_of(path)),
            _ => {}
        }
    }
    most
}

/// The threads that wrote files whose temp name holds one of `names`.
fn writers(seen: &Seen, names: &[&str]) -> HashSet<ThreadId> {
    let seen = seen.lock().unwrap();
    let wrote = |p: &PathBuf| names.iter().any(|n| p.to_string_lossy().contains(n));
    seen.iter().filter(|(_, s, p)| *s == Stage::Written && wrote(p)).map(|(t, _, _)| *t).collect()
}

/// S030 (M16): two sources on one card are read one file at a time (DEC-040
/// (1)). The first file waits at its first chunk, so a thread of the other
/// source would have its file in flight by then.
#[test]
fn s030_two_sources_on_one_card_are_read_one_file_at_a_time() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let (raw_a, raw_b) = (big(1), big(2));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", &raw_a);
    put(&a, "DCIM/100_FUJI/DSCF0002.JPG", b"a jpeg");
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &raw_b);
    put(&b, "DCIM/100_FUJI/DSCF0102.JPG", b"b jpeg");
    fs::create_dir_all(&lib).unwrap();

    let seen = Seen::default();
    let log = seen.clone();
    let _hook = hook(move |stage, path| {
        if first_time(&log, stage, path) && stage == Stage::Written && named(path, "DSCF0001.RAF") {
            thread::sleep(Duration::from_millis(200));
        }
    });
    let mut events = Log::default();
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(vec![scan_card(&a, 1), scan_card(&b, 1)], &Cancel::new(), &mut events);

    let report = last_report(&session);
    assert!(report.is_complete(), "{report:?}");
    assert_inv6(report, &[&a, &b]);
    let folder = session.folder().unwrap().path();
    assert_eq!(fs::read(folder.join("RAW/DSCF0001.RAF")).unwrap(), raw_a);
    assert_eq!(fs::read(folder.join("RAW/DSCF0101.RAF")).unwrap(), raw_b);
    assert_eq!(most_in_flight(&events, |_| 1), 1);
    let all = ["DSCF0001", "DSCF0002", "DSCF0101", "DSCF0102"];
    assert_eq!(writers(&seen, &all), HashSet::from([thread::current().id()]), "one card, this thread");
}

/// S030 (M16): beside another card, two sources on one card still share one
/// thread, and that thread runs at the same time as the other card's: they
/// meet inside a first file of each.
#[test]
fn s030_two_sources_on_one_card_share_a_thread_beside_another_card() {
    let dir = TempDir::new();
    let (a1, a2, b, lib) = (dir.0.join("a1"), dir.0.join("a2"), dir.0.join("b"), dir.0.join("lib"));
    put(&a1, "DCIM/100_FUJI/DSCF0001.RAF", &big(1));
    put(&a2, "DCIM/100_FUJI/DSCF0201.RAF", &big(3));
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &big(2));
    fs::create_dir_all(&lib).unwrap();

    let (meet, seen) = (Meet::new(2), Seen::default());
    let log = seen.clone();
    let _hook = hook(move |stage, path| {
        let meeting = named(path, "DSCF0001.RAF") || named(path, "DSCF0101.RAF");
        if first_time(&log, stage, path) && stage == Stage::Written && meeting {
            assert!(meet.meet(|| {}), "the two cards were not read at the same time");
        }
    });
    let mut events = Log::default();
    let mut session = Session::new(&lib, None).unwrap();
    let scans = vec![scan_card(&a1, 1), scan_card(&a2, 1), scan_card(&b, 2)];
    session.sync_scans(scans, &Cancel::new(), &mut events);

    let report = last_report(&session);
    assert!(report.is_complete(), "{report:?}");
    assert_inv6(report, &[&a1, &a2, &b]);
    assert_eq!(most_in_flight(&events, |p| u8::from(p.starts_with(&b))), 2);
    let (card_a, card_b) = (writers(&seen, &["DSCF0001", "DSCF0201"]), writers(&seen, &["DSCF0101"]));
    assert_eq!((card_a.len(), card_b.len()), (1, 1), "one thread per card");
    assert!(card_a.is_disjoint(&card_b));
}

/// Calls `then` once, when planning has ended: the last moment before the
/// session's own INV-10 check.
struct WhenPlanned<F>(F);

impl<F: FnMut() + Send> Watch for WhenPlanned<F> {
    fn event(&mut self, event: Event) {
        if event.progress.phase == Phase::Planning && matches!(event.kind, EventKind::Finished) {
            (self.0)();
        }
    }
}

/// S030 (M08): a library moved away after planning refuses the sync, by the
/// session's check (INV-10). Past that check it would run, and fail file by
/// file instead.
#[test]
fn s030_a_library_moved_after_planning_refuses_the_sync() {
    let dir = TempDir::new();
    let (a, lib, moved) = (dir.0.join("a"), dir.0.join("lib"), dir.0.join("moved"));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"a raw");
    put(&lib, "2026-01-01/RAW/.photosync-tmp-1-0-DSCF0009.RAF", b"partial");

    let mut session = Session::new(&lib, None).unwrap();
    let mut watch = WhenPlanned(|| fs::rename(&lib, &moved).unwrap());
    let run = session.sync_scans(vec![scan_card(&a, 1)], &Cancel::new(), &mut watch);

    match &run.end {
        RunEnd::Refused(SyncError::Copy(CopyError::Library(e))) => {
            assert_eq!(e.path, lib);
            assert_eq!(e.source.kind(), io::ErrorKind::NotFound);
        }
        other => panic!("expected the library refused, got {other:?}"),
    }
    assert_eq!(run.verdict(), Verdict::Refused);
    assert_eq!(files(&moved).len(), 1, "the moved library changed");
}

/// S030 (M08): the session's check is what guards its stale temp cleanup
/// (INV-10, DEC-027). `copy_files` checks again only for its own writes, so
/// past a missed check the cleanup deletes from a library that may have
/// moved. The device read fails as one of a moved drive would. The next
/// sync finds it back: the cleanup, not yet run, runs then.
#[test]
fn s030_the_session_checks_the_library_before_its_cleanup() {
    let dir = TempDir::new();
    let (a, lib) = (dir.0.join("a"), dir.0.join("lib"));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"a raw");
    put(&lib, "2026-01-01/RAW/.photosync-tmp-1-0-DSCF0009.RAF", b"partial");
    let before = files(&lib);

    let mut session = Session::new(&lib, None).unwrap();
    let mut watch = WhenPlanned(|| FAULT.with(|f| f.set(Some(Fault::Device))));
    let run = session.sync_scans(vec![scan_card(&a, 1)], &Cancel::new(), &mut watch);
    FAULT.with(|f| f.set(None));

    match &run.end {
        RunEnd::Refused(SyncError::Copy(CopyError::Library(e))) => {
            assert_eq!(e.source.raw_os_error(), Some(libc::EIO));
        }
        other => panic!("expected the library refused, got {other:?}"),
    }
    assert_eq!(files(&lib), before, "wrote or deleted before the check");

    session.sync_scans(vec![scan_card(&a, 1)], &Cancel::new(), &mut Log::default());
    assert!(last_report(&session).is_complete());
    assert!(temps_in(&lib).is_empty(), "the cleanup never ran");
    assert_eq!(session.verdict(), Verdict::Refused, "a later sync hid the refusal");
}

/// Two cards with a different photo of the same name (a counter reset),
/// renamed at the same instant: both are verified first, then rename into
/// one folder. The folder's lock lets one rename at a time (DEC-008): one
/// is copied, the other is a name collision, and nothing is overwritten.
#[test]
fn s019_same_name_photos_of_two_cards_give_one_copy_and_one_collision() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let ra = put(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"the photo on card a");
    let rb = put(&b, "DCIM/100_FUJI/DSCF0001.RAF", b"another photo on card b");
    put(&lib, "2026-01-01/RAW/DSCF0002.RAF", b"already there");
    let cards = (manifest(&a), manifest(&b));
    let library = files(&lib);

    let meet = Meet::new(2);
    // Threads holding the folder's lock now, the most at once, and how
    // many took it.
    let locks = Arc::new(Mutex::new((0, 0, 0)));
    let held = locks.clone();
    let _hook = hook(move |stage, _| match stage {
        Stage::Verified => assert!(meet.meet(|| {}), "the two copies did not meet before renaming"),
        Stage::RenameLocked => {
            let mut l = held.lock().unwrap();
            l.0 += 1;
            l.1 = l.1.max(l.0);
            l.2 += 1;
        }
        Stage::RenameUnlocking => held.lock().unwrap().0 -= 1,
        _ => {}
    });
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());

    let report = last_report(&session);
    let target = session.folder().unwrap().path().join("RAW/DSCF0001.RAF");
    let copied = |p: &Path| matches!(outcome(report, p), FileOutcome::Copied { to } if *to == target);
    let collided = |p: &Path| {
        matches!(outcome(report, p),
            FileOutcome::Failed(FailReason::NameCollision { existing }) if *existing == target)
    };
    let (winner, loser) = if copied(&ra) { (&ra, &rb) } else { (&rb, &ra) };
    assert!(copied(winner) && collided(loser), "{report:?}");
    assert_eq!(fs::read(&target).unwrap(), fs::read(winner).unwrap(), "the winner's bytes, whole");
    assert_eq!(*locks.lock().unwrap(), (0, 1, 2), "both renamed, one at a time");
    assert!(!report.is_complete());
    assert_inv6(report, &[&a, &b]);
    assert_eq!((manifest(&a), manifest(&b)), cards, "INV-1");
    assert_kept(&library, &lib);
    assert!(temps_in(&lib).is_empty());
}

/// The session removes stale temps once, before the first copy thread
/// starts: each thread finds it gone at its first write. A temp left
/// later stays through the session's next sync.
#[test]
fn s019_stale_temps_are_removed_once_before_any_copy_thread_starts() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"a raw");
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", b"b raw");
    let stale = put(&lib, "2026-01-01/RAW/.photosync-tmp-1-0-DSCF0009.RAF", b"partial");

    let (meet, found) = (Meet::new(2), Arc::new(Mutex::new(Vec::new())));
    let (gone, temp) = (found.clone(), stale.clone());
    let hook_set = hook(move |stage, _| {
        if stage == Stage::Written {
            gone.lock().unwrap().push((thread::current().id(), temp.exists()));
            assert!(meet.meet(|| {}), "the two cards were not copied at once");
        }
    });
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());
    drop(hook_set);

    let found = found.lock().unwrap();
    let threads: HashSet<ThreadId> = found.iter().map(|(t, _)| *t).collect();
    assert_eq!(threads.len(), 2, "{found:?}");
    assert!(found.iter().all(|(_, there)| !there), "a thread ran before the cleanup: {found:?}");
    let report = last_report(&session);
    assert!(report.is_complete());
    assert_eq!(report.temps().removed, [stale]);

    let later = put(&lib, "2026-01-01/RAW/.photosync-tmp-1-1-DSCF0010.RAF", b"partial");
    put(&a, "DCIM/100_FUJI/DSCF0002.RAF", b"a raw two");
    put(&b, "DCIM/100_FUJI/DSCF0102.RAF", b"b raw two");
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());
    let report = last_report(&session);
    assert!(report.is_complete());
    assert!(report.temps().removed.is_empty() && later.exists(), "cleaned twice in one session");
}

/// INV-10 is checked again before each folder is made, by every thread:
/// each thread checks before its first file into each type folder, even
/// one the other thread has made.
#[test]
fn s019_every_thread_checks_the_library_before_each_folder_it_makes() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let ja = put(&a, "DCIM/100_FUJI/DSCF0001.JPG", b"a jpeg");
    let ra = put(&a, "DCIM/100_FUJI/DSCF0002.RAF", b"a raw");
    let jb = put(&b, "DCIM/100_FUJI/DSCF0101.JPG", b"b jpeg");
    let rb = put(&b, "DCIM/100_FUJI/DSCF0102.RAF", b"b raw");
    fs::create_dir_all(&lib).unwrap();

    let (meet, seen) = (Meet::new(2), Seen::default());
    let log = seen.clone();
    let _hook = hook(move |stage, path| {
        if !matches!(stage, Stage::Checking | Stage::FolderReady) {
            return;
        }
        let me = thread::current().id();
        let mut seen = log.lock().unwrap();
        seen.push((me, stage, path.to_path_buf()));
        let first = seen.iter().filter(|(t, _, _)| *t == me).count() == 1;
        drop(seen);
        if first {
            assert!(meet.meet(|| {}), "the two cards were not copied at once");
        }
    });
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());

    assert!(last_report(&session).is_complete());
    let folder = session.folder().unwrap().path().to_path_buf();
    let seen = seen.lock().unwrap();
    for (jpeg, raw) in [(&ja, &ra), (&jb, &rb)] {
        let thread = seen.iter().find(|(_, _, p)| p == jpeg).unwrap().0;
        let steps: Vec<(Stage, &Path)> =
            seen.iter().filter(|(t, _, _)| *t == thread).map(|(_, s, p)| (*s, p.as_path())).collect();
        assert_eq!(
            steps,
            [
                (Stage::Checking, jpeg.as_path()),
                (Stage::FolderReady, folder.join("JPEG").as_path()),
                (Stage::Checking, raw.as_path()),
                (Stage::FolderReady, folder.join("RAW").as_path()),
            ]
        );
    }
}

/// Once the INV-10 check has failed in one thread, every thread checks
/// before every file. Card a's thread cannot read the library's device;
/// card b's thread starts only after that, and checks before each of its
/// three files, though they go into two folders.
#[test]
fn s019_once_a_check_fails_in_one_thread_every_thread_checks_before_every_file() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let lost = [put(&a, "DCIM/100_FUJI/DSCF0001.JPG", b"a jpeg"), put(&a, "DCIM/100_FUJI/DSCF0002.RAF", b"a raw")];
    put(&b, "DCIM/100_FUJI/DSCF0101.JPG", b"b jpeg");
    put(&b, "DCIM/100_FUJI/DSCF0102.JPG", b"b jpeg two");
    put(&b, "DCIM/100_FUJI/DSCF0103.RAF", b"b raw");
    fs::create_dir_all(&lib).unwrap();

    let meet = Meet::new(2);
    let checks = Arc::new(Mutex::new((0, 0)));
    let count = checks.clone();
    let (card_a, card_b) = (a.clone(), b.clone());
    let _hook = hook(move |stage, path| {
        if stage != Stage::Checking {
            return;
        }
        let (on_a, n) = {
            let mut c = count.lock().unwrap();
            let on_a = path.starts_with(&card_a);
            let n = if on_a { &mut c.0 } else { &mut c.1 };
            *n += 1;
            (on_a, *n)
        };
        assert!(on_a || path.starts_with(&card_b));
        match (on_a, n) {
            // The drive is gone, for this thread from now on.
            (true, 1) => FAULT.with(|f| f.set(Some(Fault::Device))),
            // Card a's first check has failed by its second; b's first
            // waits for it.
            (true, 2) | (false, 1) => assert!(meet.meet(|| {}), "the threads did not meet"),
            _ => {}
        }
    });
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());

    let report = last_report(&session);
    for file in &lost {
        assert!(
            matches!(outcome(report, file), FileOutcome::Failed(FailReason::Library(CopyError::Library(e)))
                if e.source.raw_os_error() == Some(libc::EIO)),
            "{:?}",
            outcome(report, file)
        );
    }
    let copied = report.sources()[1].files.iter().filter(|e| matches!(e.outcome, FileOutcome::Copied { .. }));
    assert_eq!(copied.count(), 3);
    assert_eq!(*checks.lock().unwrap(), (2, 3), "card b's thread checked before every file");
    assert_inv6(report, &[&a, &b]);
}

/// A cancel stops every thread at its next check: both are inside their
/// first file when it is set, and both cut it short. Every file is
/// cancelled and listed once (INV-6); no temp and no partial file is left,
/// nothing on the drive changed (INV-2, INV-7), nor on the cards (INV-1).
#[test]
fn s019_a_cancel_stops_every_thread_at_its_next_check() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", &big(1));
    put(&a, "DCIM/100_FUJI/DSCF0002.RAF", b"a raw two");
    put(&a, "DCIM/100_FUJI/DSCF0003.JPG", b"a jpeg");
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &big(2));
    put(&b, "DCIM/100_FUJI/DSCF0102.RAF", b"b raw two");
    put(&b, "DCIM/100_FUJI/DSCF0103.JPG", b"b jpeg");
    put(&lib, "2026-01-01/RAW/DSCF0001.RAF", b"an older photo, same name");
    let cards = (manifest(&a), manifest(&b));
    let library = files(&lib);

    let cancel = Cancel::new();
    let (meet, flag, seen) = (Meet::new(2), cancel.clone(), Seen::default());
    let log = seen.clone();
    let _hook = hook(move |stage, path| {
        if first_time(&log, stage, path) && stage == Stage::Written {
            assert!(meet.meet(|| flag.cancel()), "the two cards were not read at once");
        }
    });
    let mut events = Log::default();
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &cancel, &mut events);

    let report = last_report(&session);
    assert_inv6(report, &[&a, &b]);
    for source in report.sources() {
        assert_eq!(source.totals().cancelled.files, 3, "{source:?}");
    }
    let written = seen.lock().unwrap().iter().filter(|(_, s, _)| *s == Stage::Written).count();
    assert_eq!(written, 2, "a thread read on past the cancel");
    assert!(temps_in(&lib).is_empty(), "INV-7: a temp was left");
    assert_eq!(files(&lib), library, "INV-2, INV-7: the drive changed");
    assert_eq!((manifest(&a), manifest(&b)), cards, "INV-1");
    let last = events.0.last().unwrap();
    assert!(matches!(last.kind, EventKind::Cancelled), "{last:?}");
}

/// A twin on a card that its own thread is reading: card a's photo cannot
/// be read, so a's thread reads its twin on card b, while b's thread reads
/// b's own photo there. The twin is the plan's duplicate, never b's own
/// file to copy: the photo is copied once, by a's thread, and checked
/// against the hash planning took (INV-3).
#[test]
fn s019_a_twin_on_a_card_another_thread_reads_stands_in_and_is_copied_once() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let (photo, own) = (big(7), big(8));
    let x = put(&a, "DCIM/100_FUJI/DSCF0001.RAF", &photo);
    let twin = put(&b, "DCIM/100_FUJI/DSCF0001.RAF", &photo);
    let y = put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &own);
    fs::create_dir_all(&lib).unwrap();
    let cards = (manifest(&a), manifest(&b));

    let (meet, seen) = (Meet::new(2), Seen::default());
    let log = seen.clone();
    let _hook = hook(move |stage, path| {
        log.lock().unwrap().push((thread::current().id(), stage, path.to_path_buf()));
        let count = |name| {
            let seen = log.lock().unwrap();
            seen.iter().filter(|(_, s, p)| *s == stage && named(p, name)).count()
        };
        match stage {
            // The planned file's next read fails.
            Stage::Written if named(path, "DSCF0001.RAF") && count("DSCF0001.RAF") == 1 => {
                FAULT.with(|f| f.set(Some(Fault::Read)));
            }
            // The twin's first chunk, read off card b, meets b's own read.
            Stage::Written if named(path, "DSCF0001.RAF") && count("DSCF0001.RAF") == 2 => {
                assert!(meet.meet(|| {}), "the twin and card b's photo were not read at once");
            }
            Stage::Written if named(path, "DSCF0101.RAF") && count("DSCF0101.RAF") == 1 => {
                assert!(meet.meet(|| {}), "the twin and card b's photo were not read at once");
            }
            _ => {}
        }
    });
    let mut session = Session::new(&lib, None).unwrap();
    session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());

    let report = last_report(&session);
    let folder = session.folder().unwrap().path();
    assert!(matches!(outcome(report, &x), FileOutcome::DuplicateUnread { of, .. } if *of == twin));
    let target = folder.join("RAW/DSCF0001.RAF");
    assert!(matches!(outcome(report, &twin), FileOutcome::Copied { to } if *to == target));
    assert!(matches!(outcome(report, &y), FileOutcome::Copied { .. }));
    assert!(report.is_complete(), "{report:?}");
    assert_inv6(report, &[&a, &b]);
    let want = BTreeMap::from([(target.clone(), photo), (folder.join("RAW/DSCF0101.RAF"), own)]);
    assert_eq!(files(&lib), want, "each photo once, with the planned bytes");
    assert_eq!((manifest(&a), manifest(&b)), cards, "INV-1");

    let seen = seen.lock().unwrap();
    let thread_of = |name: &str, nth: usize| {
        seen.iter().filter(|(_, s, p)| *s == Stage::Written && named(p, name)).nth(nth).unwrap().0
    };
    assert_eq!(thread_of("DSCF0001.RAF", 1), thread_of("DSCF0001.RAF", 0), "the twin is read by a's thread");
    assert_ne!(thread_of("DSCF0001.RAF", 1), thread_of("DSCF0101.RAF", 0));
    let renamed = seen.iter().filter(|(_, s, p)| *s == Stage::RenameLocked && *p == target).count();
    assert_eq!(renamed, 1, "the photo was copied twice");
}

/// A panic in one copy thread stops the other at its next check, as a
/// cancel does, and goes on to the caller only once every thread has
/// ended. The front-end records the sync as crashed: not complete. The
/// panicked thread's temp is what a kill leaves (INV-7), and the next
/// session removes it and finishes the job.
#[test]
fn s019_a_panic_in_one_thread_stops_the_others_and_every_thread_is_joined() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", &big(1));
    put(&a, "DCIM/100_FUJI/DSCF0002.RAF", b"a raw two");
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &big(2));
    let next = put(&b, "DCIM/100_FUJI/DSCF0102.RAF", b"b raw two");
    fs::create_dir_all(&lib).unwrap();

    let (meet, halted, seen) = (Meet::new(2), Meet::new(2), Seen::default());
    let log = seen.clone();
    let hooked = hook(move |stage, path| {
        let first = first_time(&log, stage, path);
        match stage {
            Stage::Written if first && named(path, "DSCF0001.RAF") => {
                assert!(meet.meet(|| {}), "the two cards were not read at once");
                panic!("simulated crash in card a's thread");
            }
            Stage::Written if first && named(path, "DSCF0101.RAF") => {
                assert!(meet.meet(|| {}), "the two cards were not read at once");
                assert!(halted.meet(|| {}), "card a's thread never said it stopped");
            }
            // In the panicking thread's unwind: no assert here, a second
            // panic would abort.
            Stage::Halted => {
                halted.meet(|| {});
            }
            _ => {}
        }
    });
    let roots = [a.clone(), b.clone()];
    let mut events = Log::default();
    let mut session = Session::new(&lib, None).unwrap();
    let crashed = panic::catch_unwind(AssertUnwindSafe(|| {
        session.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut events);
    }));

    let payload = crashed.expect_err("the panic did not reach the caller");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"simulated crash in card a's thread"));
    // The copy threads' copies of the hook went with their threads.
    assert_eq!(SHARED_HOOK.with(|h| Arc::strong_count(h.borrow().as_ref().unwrap())), 1, "a thread runs on");
    let started: Vec<&Path> = events
        .0
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::FileStarted(p) => Some(p.as_path()),
            _ => None,
        })
        .collect();
    assert!(!started.contains(&next.as_path()), "card b's thread went on to its next file");
    let folder = session.folder().unwrap().path().to_path_buf();
    let left: Vec<PathBuf> = files(&lib).into_keys().collect();
    let temps = temps_in(&lib);
    assert_eq!(left, temps, "INV-7: a file under a final name");
    assert_eq!(temps.len(), 1, "only the panicked thread's temp: {temps:?}");
    assert!(named(&temps[0], "DSCF0001.RAF"), "{temps:?}");

    assert!(session.runs().is_empty());
    session.record_crash(&roots, "simulated crash".into());
    assert_eq!(session.verdict(), Verdict::NotComplete);

    drop(hooked);
    let mut again = Session::new(&lib, None).unwrap();
    again.sync_scans(on_own_cards(&[&a, &b]), &Cancel::new(), &mut Log::default());
    let report = last_report(&again);
    assert!(report.is_complete(), "{report:?}");
    assert_eq!(report.temps().removed, temps);
    assert_eq!(fs::read(folder.join("RAW/DSCF0102.RAF")).unwrap(), b"b raw two");
}
