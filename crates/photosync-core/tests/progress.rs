//! S005: progress events, ETA and cancel, for planning and for copying;
//! INV-1 and INV-7 around a cancel. Cancel at each stage of one file's copy
//! is a unit test in src/copy.rs, where the stage hook lives.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use photosync_core::{
    Cancel, Counts, Event, EventKind, Monitor, Phase, PlanError, SourceProgress, SourceScan,
    SyncPlan, copy_plan, eta, plan_sync, scan_source, sync_folder,
};

mod common;
use common::{TempDir, at, manifest, write_file};

const MIB: usize = 1 << 20;

/// More than one chunk, so it gives several `Bytes` events.
fn big(seed: u8) -> Vec<u8> {
    (0..MIB + MIB / 2).map(|i| (i % 251) as u8 ^ seed).collect()
}

fn put(root: &Path, rel: &str, bytes: &[u8]) {
    write_file(root, rel, bytes, at(1_761_650_000));
}

/// A clock that moves one second per reading, so ETAs need no real time.
fn ticking() -> impl FnMut() -> Duration + Send {
    let mut t = Duration::ZERO;
    move || {
        t += Duration::from_secs(1);
        t
    }
}

/// Records every event, and runs `on` on each with the shared cancel.
fn recorder(
    cancel: &Cancel,
    mut on: impl FnMut(&Event, &Cancel) + Send + 'static,
) -> (Monitor<'static>, Arc<Mutex<Vec<Event>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let (log, flag) = (events.clone(), cancel.clone());
    let monitor = Monitor::new(cancel.clone(), move |e: Event| {
        on(&e, &flag);
        log.lock().unwrap().push(e);
    })
    .with_clock(ticking());
    (monitor, events)
}

fn quiet() -> Monitor<'static> {
    Monitor::default()
}

fn counts(files: &[&photosync_core::SourceFile]) -> Counts {
    Counts { files: files.len() as u64, bytes: files.iter().map(|f| f.size).sum() }
}

/// The event order every phase keeps, per `EventKind`'s doc. Returns the
/// files in the order they were started.
fn assert_order(events: &[Event], phase: Phase) -> Vec<PathBuf> {
    assert!(matches!(events[0].kind, EventKind::Started), "{:?}", events[0]);
    assert_eq!(events[0].progress.done, Counts::default());
    let last = events.last().unwrap();
    assert!(matches!(last.kind, EventKind::Finished | EventKind::Cancelled), "{last:?}");
    let mut started = Vec::new();
    let mut current: Option<&PathBuf> = None;
    for (prev, e) in events.iter().zip(&events[1..]) {
        assert_eq!(e.progress.phase, phase);
        assert_eq!(e.progress.total, events[0].progress.total, "total never changes");
        let (p, q) = (&prev.progress, &e.progress);
        assert!(q.done.files >= p.done.files && q.done.bytes >= p.done.bytes, "done went back");
        assert!(q.elapsed >= p.elapsed);
        match &e.kind {
            EventKind::Started => panic!("second Started"),
            EventKind::FileStarted(path) => {
                assert!(current.is_none(), "{path:?} started inside {current:?}");
                current = Some(path);
                started.push(path.clone());
            }
            EventKind::Bytes => assert!(current.is_some(), "Bytes outside a file"),
            EventKind::FileDone(path) => {
                assert_eq!(current.take(), Some(path), "FileDone for another file");
                assert_eq!(q.done.files, p.done.files + 1);
            }
            EventKind::Finished => assert!(current.is_none()),
            EventKind::Cancelled => {}
        }
        // Per source, the parts add up to the whole.
        let sum = |f: fn(&SourceProgress) -> u64| q.sources.iter().map(f).sum::<u64>();
        assert!(sum(|s| s.done.files) <= q.done.files && sum(|s| s.done.bytes) <= q.done.bytes);
        assert_eq!(q.eta, eta(q.done.bytes, q.total.bytes, q.elapsed));
    }
    started
}

/// Two cards, each with a new file and a big one; card A also has one that
/// collides with a different library file of the same name.
fn two_cards(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (a, b, lib) = (dir.join("a"), dir.join("b"), dir.join("lib"));
    put(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"a one");
    put(&a, "DCIM/100_FUJI/DSCF0002.RAF", &big(1));
    put(&a, "DCIM/100_FUJI/DSCF0003.JPG", b"collides");
    put(&b, "DCIM/100_FUJI/DSCF0101.RAF", &big(2));
    put(&b, "DCIM/100_FUJI/DSCF0102.JPG", b"b two");
    fs::create_dir_all(&lib).unwrap();
    (a, b, lib)
}

#[test]
fn copy_events_arrive_in_order_and_their_totals_match_the_plan() {
    let dir = TempDir::new();
    let (a, b, lib) = two_cards(&dir.0);
    let sources = [scan_source(&a).unwrap(), scan_source(&b).unwrap()];
    let plan = plan_sync(&lib, &sources, &mut quiet()).unwrap();
    let folder = sync_folder(&plan, Some("Trip")).unwrap().unwrap();
    // Taken after planning: the name collision fails at copy time.
    put(folder.path(), "JPEG/DSCF0003.JPG", b"other!!!");

    let cancel = Cancel::new();
    let (mut monitor, events) = recorder(&cancel, |_, _| {});
    let outcome = copy_plan(&plan, &folder, &mut monitor).unwrap();
    drop(monitor);
    let events = events.lock().unwrap();

    assert_eq!(outcome.copied.len(), 4);
    assert_eq!(outcome.failed.len(), 1, "the collision still counts as done");
    assert!(outcome.cancelled.is_empty());
    let started = assert_order(&events, Phase::Copying);
    let planned: Vec<PathBuf> = plan.to_copy.iter().map(|f| f.path.clone()).collect();
    assert_eq!(started, planned, "every planned file, in plan order");
    let of = |root: &Path| plan.to_copy.iter().filter(|f| f.path.starts_with(root)).collect::<Vec<_>>();
    let all: Vec<_> = plan.to_copy.iter().collect();
    let want = [
        SourceProgress { root: a.clone(), done: counts(&of(&a)), total: counts(&of(&a)) },
        SourceProgress { root: b.clone(), done: counts(&of(&b)), total: counts(&of(&b)) },
    ];
    assert_eq!(events[0].progress.total, counts(&all));
    assert_eq!(events[0].progress.sources[0].total, want[0].total);
    assert_eq!(events[0].progress.sources[1].total, want[1].total);
    let last = events.last().unwrap();
    assert!(matches!(last.kind, EventKind::Finished));
    assert_eq!(last.progress.done, counts(&all));
    assert_eq!(last.progress.sources, want);
    // One per chunk: both big files give two each, the small ones one.
    let chunks = events.iter().filter(|e| e.kind == EventKind::Bytes).count();
    assert_eq!(chunks, 2 + 2 + 1 + 1);
}

#[test]
fn eta_is_present_once_some_bytes_are_copied() {
    let dir = TempDir::new();
    let (a, b, lib) = two_cards(&dir.0);
    let plan = plan_sync(&lib, &[scan_source(&a).unwrap(), scan_source(&b).unwrap()], &mut quiet()).unwrap();
    let folder = sync_folder(&plan, None).unwrap().unwrap();

    let (mut monitor, events) = recorder(&Cancel::new(), |_, _| {});
    copy_plan(&plan, &folder, &mut monitor).unwrap();
    drop(monitor);
    let events = events.lock().unwrap();

    for e in events.iter() {
        assert_eq!(e.progress.eta.is_some(), e.progress.done.bytes > 0, "{e:?}");
    }
    // The ticking clock: the n-th event after Started is n seconds in.
    let first = events.iter().position(|e| e.kind == EventKind::Bytes).unwrap();
    let p = &events[first].progress;
    assert_eq!(p.elapsed, Duration::from_secs(first as u64));
    assert_eq!(p.eta, eta(p.done.bytes, p.total.bytes, p.elapsed));
    assert!(p.eta.unwrap() > Duration::ZERO);
    assert_eq!(events.last().unwrap().progress.eta, Some(Duration::ZERO));
}

/// Backup slot mode: two identical cards, so every file has a twin in the
/// session and is hashed on both cards. One is already in the library.
fn backup_cards(dir: &Path) -> ([PathBuf; 2], PathBuf, [SourceScan; 2]) {
    let (a, b, lib) = (dir.join("a"), dir.join("b"), dir.join("lib"));
    for card in [&a, &b] {
        put(card, "DCIM/100_FUJI/DSCF0001.RAF", &big(1));
        put(card, "DCIM/100_FUJI/DSCF0001.JPG", b"jpg one");
        put(card, "DCIM/100_FUJI/DSCF0002.RAF", b"raw two");
    }
    put(&lib, "2025-01-01/RAW/DSCF0002.RAF", b"raw two");
    let scans = [scan_source(&a).unwrap(), scan_source(&b).unwrap()];
    ([a, b], lib, scans)
}

#[test]
fn planning_events_cover_every_file_hashed_twice_in_backup_slot_mode() {
    let dir = TempDir::new();
    let ([a, b], lib, scans) = backup_cards(&dir.0);

    let (mut monitor, events) = recorder(&Cancel::new(), |_, _| {});
    let plan = plan_sync(&lib, &scans, &mut monitor).unwrap();
    drop(monitor);
    let events = events.lock().unwrap();

    assert_eq!((plan.to_copy.len(), plan.duplicates.len(), plan.skipped.len()), (2, 3, 1));
    let started = assert_order(&events, Phase::Planning);
    let every: Vec<PathBuf> = scans.iter().flat_map(|s| s.supported()).map(|f| f.path.clone()).collect();
    assert_eq!(started, every, "each file on both cards is hashed once");
    let card = |s: &SourceScan| counts(&s.supported().collect::<Vec<_>>());
    let want = [
        SourceProgress { root: a, done: card(&scans[0]), total: card(&scans[0]) },
        SourceProgress { root: b, done: card(&scans[1]), total: card(&scans[1]) },
    ];
    let all = Counts { files: 6, bytes: want[0].total.bytes * 2 };
    assert_eq!(events[0].progress.total, all);
    let last = events.last().unwrap();
    assert!(matches!(last.kind, EventKind::Finished));
    assert_eq!(last.progress.done, all);
    assert_eq!(last.progress.sources, want);
    assert!(events.iter().any(|e| e.kind == EventKind::Bytes && e.progress.eta.is_some()));
}

#[test]
fn planning_unique_files_are_not_counted_and_nothing_to_hash_still_starts_and_finishes() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"unique");
    fs::create_dir_all(&lib).unwrap();

    let (mut monitor, events) = recorder(&Cancel::new(), |_, _| {});
    let plan = plan_sync(&lib, &[scan_source(&card).unwrap()], &mut monitor).unwrap();
    drop(monitor);
    let kinds: Vec<EventKind> = events.lock().unwrap().iter().map(|e| e.kind.clone()).collect();

    assert_eq!(plan.to_copy.len(), 1);
    assert_eq!(kinds, [EventKind::Started, EventKind::Finished]);
}

#[test]
fn cancel_during_planning_returns_cancelled_and_writes_nothing() {
    let dir = TempDir::new();
    let ([a, b], lib, scans) = backup_cards(&dir.0);
    let before = (manifest(&a), manifest(&b), manifest(&lib));

    // Cancelled after the first chunk of the first file.
    let (mut monitor, events) = recorder(&Cancel::new(), |e, cancel| {
        if e.kind == EventKind::Bytes {
            cancel.cancel();
        }
    });
    let result = plan_sync(&lib, &scans, &mut monitor);
    drop(monitor);
    let events = events.lock().unwrap();

    assert!(matches!(result, Err(PlanError::Cancelled)), "{result:?}");
    assert_eq!(assert_order(&events, Phase::Planning).len(), 1, "stopped inside the first file");
    assert_eq!(events.last().unwrap().kind, EventKind::Cancelled);
    assert_eq!(events.iter().filter(|e| e.kind == EventKind::Bytes).count(), 1);
    assert_eq!((manifest(&a), manifest(&b), manifest(&lib)), before, "INV-1, and nothing written");

    // A cancel set before the call stops it before any hashing.
    let cancel = Cancel::new();
    cancel.cancel();
    let result = plan_sync(&lib, &scans, &mut Monitor::new(cancel, |_| {}));
    assert!(matches!(result, Err(PlanError::Cancelled)), "{result:?}");
}

fn temps_under(dir: &Path) -> Vec<PathBuf> {
    manifest(dir)
        .into_keys()
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(".photosync-tmp-"))
        .collect()
}

fn plan_one(card: &Path, lib: &Path) -> SyncPlan {
    plan_sync(lib, &[scan_source(card).unwrap()], &mut quiet()).unwrap()
}

#[test]
fn inv7_cancel_mid_file_removes_its_temp_and_a_rerun_finishes_the_job() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"first");
    put(&card, "DCIM/100_FUJI/DSCF0002.RAF", &big(3));
    put(&card, "DCIM/100_FUJI/DSCF0003.RAF", b"third");
    fs::create_dir_all(&lib).unwrap();
    let source = manifest(&card);
    let plan = plan_one(&card, &lib);
    let folder = sync_folder(&plan, Some("Trip")).unwrap().unwrap();

    // Cancelled after the first chunk of the big file, with its partial temp
    // on disk at that moment.
    let lib_seen = lib.clone();
    let partial = Arc::new(Mutex::new(Vec::new()));
    let seen = partial.clone();
    let (mut monitor, events) = recorder(&Cancel::new(), move |e, cancel| {
        if e.kind == EventKind::Bytes && e.progress.done.files == 1 {
            *seen.lock().unwrap() = temps_under(&lib_seen);
            cancel.cancel();
        }
    });
    let outcome = copy_plan(&plan, &folder, &mut monitor).unwrap();
    drop(monitor);
    let events = events.lock().unwrap();

    let temps = partial.lock().unwrap();
    assert_eq!(temps.len(), 1, "a partial temp existed");
    assert_eq!(fs::metadata(&temps[0]).ok().map(|m| m.len()), None, "and is gone now");
    assert!(temps_under(&lib).is_empty());
    let raw = folder.path().join("RAW");
    assert_eq!(fs::read(raw.join("DSCF0001.RAF")).unwrap(), b"first");
    assert!(!raw.join("DSCF0002.RAF").exists() && !raw.join("DSCF0003.RAF").exists());
    assert_eq!(outcome.copied.len(), 1);
    assert!(outcome.failed.is_empty());
    assert_eq!(outcome.cancelled, plan.to_copy[1..]);
    assert_eq!(assert_order(&events, Phase::Copying).len(), 2, "nothing started after the cancel");
    assert_eq!(events.last().unwrap().kind, EventKind::Cancelled);
    assert_eq!(manifest(&card), source, "INV-1 around a cancelled sync");

    let plan = plan_one(&card, &lib);
    assert_eq!(plan.skipped.len(), 1);
    let rerun = copy_plan(&plan, &sync_folder(&plan, Some("Trip")).unwrap().unwrap(), &mut quiet()).unwrap();
    assert_eq!(rerun.copied.len(), 2);
    assert_eq!(fs::read(raw.join("DSCF0002.RAF")).unwrap(), big(3));
    assert_eq!(manifest(&card), source);
}

#[test]
fn cancel_between_files_stops_at_the_file_boundary() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"first");
    put(&card, "DCIM/100_FUJI/DSCF0002.RAF", b"second");
    fs::create_dir_all(&lib).unwrap();
    let source = manifest(&card);
    let plan = plan_one(&card, &lib);
    let folder = sync_folder(&plan, None).unwrap().unwrap();

    let (mut monitor, events) = recorder(&Cancel::new(), |e, cancel| {
        if matches!(e.kind, EventKind::FileDone(_)) {
            cancel.cancel();
        }
    });
    let outcome = copy_plan(&plan, &folder, &mut monitor).unwrap();
    drop(monitor);
    let events = events.lock().unwrap();

    assert_eq!(outcome.copied.len(), 1);
    assert_eq!(outcome.cancelled, plan.to_copy[1..]);
    assert!(!folder.path().join("RAW/DSCF0002.RAF").exists());
    assert!(temps_under(&lib).is_empty());
    let kinds: Vec<&EventKind> = events.iter().map(|e| &e.kind).collect();
    let first = plan.to_copy[0].path.clone();
    assert_eq!(
        kinds,
        [
            &EventKind::Started,
            &EventKind::FileStarted(first.clone()),
            &EventKind::Bytes,
            &EventKind::FileDone(first),
            &EventKind::Cancelled,
        ]
    );
    assert_eq!(manifest(&card), source);
}

#[test]
fn monitor_events_and_cancel_can_cross_threads() {
    fn send<T: Send>() {}
    fn sync<T: Sync>() {}
    send::<Monitor<'static>>();
    send::<Event>();
    send::<Cancel>();
    sync::<Cancel>();

    // As a GUI would: the sync on a worker thread, events over a channel,
    // cancel from the other side.
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"first");
    fs::create_dir_all(&lib).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = Cancel::new();
    let worker = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            let mut monitor = Monitor::new(cancel, move |e| tx.send(e).unwrap());
            let plan = plan_sync(&lib, &[scan_source(&card).unwrap()], &mut monitor).unwrap();
            let folder = sync_folder(&plan, None).unwrap().unwrap();
            copy_plan(&plan, &folder, &mut monitor).unwrap()
        })
    };
    let outcome = worker.join().unwrap();
    let phases: Vec<(Phase, EventKind)> = rx
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Started | EventKind::Finished))
        .map(|e| (e.progress.phase, e.kind))
        .collect();
    assert_eq!(
        phases,
        [
            (Phase::Planning, EventKind::Started),
            (Phase::Planning, EventKind::Finished),
            (Phase::Copying, EventKind::Started),
            (Phase::Copying, EventKind::Finished),
        ]
    );
    assert_eq!(outcome.copied.len(), 1);
    assert!(!cancel.is_cancelled());
}

#[test]
fn plan_cancelled_before_it_starts_sends_no_file_or_bytes_events() {
    let dir = TempDir::new();
    let (_, lib, scans) = backup_cards(&dir.0);
    let cancel = Cancel::new();
    cancel.cancel();
    let (mut monitor, events) = recorder(&cancel, |_, _| {});

    let result = plan_sync(&lib, &scans, &mut monitor);
    drop(monitor);

    assert!(matches!(result, Err(PlanError::Cancelled)), "{result:?}");
    let kinds: Vec<EventKind> = events.lock().unwrap().iter().map(|e| e.kind.clone()).collect();
    assert_eq!(kinds, [EventKind::Started, EventKind::Cancelled]);
}
