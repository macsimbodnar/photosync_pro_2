//! S010: a sync session. One card after another into one folder, several
//! cards at once, a later card's twin of an earlier one, the temp cleanup
//! once per session, a card given twice, the merged report and the worst
//! outcome of every sync. INV-1 and INV-6 hold for every card.

use std::fs;
use std::path::{Path, PathBuf};

use photosync_core::{
    Cancel, Duplicate, Event, EventKind, FailReason, FileOutcome, FolderError, Monitor, Phase,
    Report, ReportError, Run, RunEnd, Session, SourceScan, SyncError, Verdict, Watch, plan_sync,
};

mod common;
use common::{TempDir, at, date_command, manifest, scan_card, scan_elsewhere, write_file};

const T0: u64 = 1_761_650_000; // 2026-10-28, mid-day in most zones
const DAY: u64 = 86_400;

fn put(root: &Path, files: &[(&str, &[u8])]) {
    for (rel, bytes) in files {
        write_file(root, rel, bytes, at(T0));
    }
}

/// Keeps every scanned root and event.
#[derive(Default)]
struct Log {
    scanned: Vec<PathBuf>,
    events: Vec<Event>,
}

impl Watch for Log {
    fn scanned(&mut self, scan: &SourceScan) {
        self.scanned.push(scan.root.clone());
    }

    fn event(&mut self, event: Event) {
        self.events.push(event);
    }
}

/// One sync of `cards` at once, scanned as on another device than the
/// library (`scan_elsewhere`), as a real card is.
fn sync<'s>(session: &'s mut Session, cards: &[&Path]) -> &'s Run {
    let scans = cards.iter().map(|c| scan_elsewhere(c)).collect();
    session.sync_scans(scans, &Cancel::new(), &mut Log::default())
}

fn report(run: &Run) -> &Report {
    match &run.end {
        RunEnd::Report(report) => report,
        other => panic!("no report: {other:?}"),
    }
}

fn outcome<'r>(report: &'r Report, path: &Path) -> &'r FileOutcome {
    let mut entries = report.sources().iter().flat_map(|s| &s.files);
    &entries.find(|e| e.file.path == path).expect("listed").outcome
}

/// INV-6 per source, against fresh scans: every listed file once, and
/// copied + skipped + failed + cancelled = the supported files.
fn assert_inv6(report: &Report, cards: &[&Path]) {
    assert_eq!(report.sources().len(), cards.len());
    for (source, card) in report.sources().iter().zip(cards) {
        let fresh = scan_elsewhere(card);
        let t = source.totals();
        assert_eq!(t.supported().files, fresh.supported().count() as u64, "{card:?}");
        assert_eq!(t.supported().bytes, fresh.supported().map(|f| f.size).sum::<u64>());
        let listed: Vec<&Path> = source.files.iter().map(|e| e.file.path.as_path()).collect();
        let scanned: Vec<&Path> = fresh.files.iter().map(|f| f.path.as_path()).collect();
        assert_eq!(listed, scanned, "{card:?}: each listed file once");
    }
}

fn library_files(lib: &Path) -> Vec<PathBuf> {
    manifest(lib).into_iter().filter(|(_, (_, bytes))| bytes.is_some()).map(|(p, _)| p).collect()
}

/// One reader, two cards: the second card's photos go to the folder the
/// first sync fixed, though they are older (DEC-009), and both syncs are
/// in the session's runs.
#[test]
fn a_session_takes_another_card_after_the_first_into_the_same_folder() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    write_file(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"a raw", at(T0 + DAY));
    write_file(&b, "DCIM/100_FUJI/DSCF0101.RAF", b"b raw", at(T0 - 3 * DAY));
    write_file(&b, "DCIM/100_FUJI/DSCF0101.JPG", b"b jpeg", at(T0 - 3 * DAY));
    fs::create_dir_all(&lib).unwrap();
    let mut session = Session::new(&lib, None).unwrap();
    assert!(session.folder().is_none());

    assert!(report(sync(&mut session, &[&a])).is_complete());
    let folder = lib.join(date_command(T0 + DAY));
    assert_eq!(session.folder().map(|f| f.path()), Some(folder.as_path()));
    let second = report(sync(&mut session, &[&b]));
    assert!(second.is_complete());

    assert_eq!(fs::read(folder.join("RAW/DSCF0101.RAF")).unwrap(), b"b raw");
    assert_eq!(fs::read(folder.join("JPEG/DSCF0101.JPG")).unwrap(), b"b jpeg");
    assert_eq!(fs::read_dir(&lib).unwrap().count(), 1, "one session folder");
    let runs: Vec<&[PathBuf]> = session.runs().iter().map(|r| r.sources.as_slice()).collect();
    assert_eq!(runs, [[a.clone()], [b.clone()]]);
    assert_eq!(session.verdict(), Verdict::Complete);
}

/// The session remembers what earlier cards put on the drive or found
/// there: the same photo on a later card is a duplicate in session of the
/// earlier card's file, not "already on drive". A photo an earlier card
/// did not get onto the drive (cancelled) is copied from the later one.
#[test]
fn a_twin_on_a_later_card_is_a_duplicate_in_session_not_already_on_drive() {
    let dir = TempDir::new();
    let (a, b, c, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("c"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"backup"), ("DCIM/100_FUJI/DSCF0002.RAF", b"old one")]);
    put(&b, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"backup"),
        ("DCIM/100_FUJI/DSCF0002.RAF", b"old one"),
        ("DCIM/100_FUJI/DSCF0003.RAF", b"only b"),
    ]);
    put(&c, &[("DCIM/100_FUJI/DSCF0004.RAF", b"cancelled")]);
    put(&lib, &[("2026-01-01/RAW/DSCF0002.RAF", b"old one")]);
    let mut session = Session::new(&lib, Some("Trip")).unwrap();

    let cancel = Cancel::new();
    cancel.cancel();
    let cut = session.sync_scans(vec![scan_elsewhere(&c)], &cancel, &mut Log::default());
    assert_eq!(report(cut).totals().cancelled.files, 1);
    let first = report(sync(&mut session, &[&a]));
    let card_a = |name: &str| a.join("DCIM/100_FUJI").join(name);
    assert!(matches!(outcome(first, &card_a("DSCF0002.RAF")), FileOutcome::AlreadySynced { .. }));
    put(&c, &[("DCIM/100_FUJI/DSCF0003.RAF", b"only b")]);
    let second = report(sync(&mut session, &[&b, &c]));

    let card_b = |name: &str| b.join("DCIM/100_FUJI").join(name);
    for name in ["DSCF0001.RAF", "DSCF0002.RAF"] {
        match outcome(second, &card_b(name)) {
            FileOutcome::DuplicateInSession { of } => assert_eq!(*of, card_a(name)),
            other => panic!("{name}: {other:?}"),
        }
    }
    assert!(matches!(outcome(second, &card_b("DSCF0003.RAF")), FileOutcome::Copied { .. }));
    // In this sync's own plan, c's copy of DSCF0003 is b's twin.
    assert!(matches!(
        outcome(second, &c.join("DCIM/100_FUJI/DSCF0003.RAF")),
        FileOutcome::DuplicateInSession { of } if *of == card_b("DSCF0003.RAF")
    ));
    assert!(matches!(outcome(second, &c.join("DCIM/100_FUJI/DSCF0004.RAF")), FileOutcome::Copied { .. }));
    assert!(second.is_complete());
    let folder = session.folder().unwrap().path().to_path_buf();
    let mut want = vec![
        lib.join("2026-01-01/RAW/DSCF0002.RAF"),
        folder.join("RAW/DSCF0001.RAF"),
        folder.join("RAW/DSCF0003.RAF"),
        folder.join("RAW/DSCF0004.RAF"),
    ];
    want.sort();
    assert_eq!(library_files(&lib), want, "each photo once");
}

/// Two readers: two cards in one sync. A photo on both is copied once, the
/// other card's copy is a duplicate in session, and progress is counted
/// per card.
#[test]
fn two_cards_at_once_copy_a_photo_on_both_once_with_progress_per_card() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw on both"), ("DCIM/100_FUJI/DSCF0002.JPG", b"a")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw on both"), ("DCIM/100_FUJI/DSCF0003.MOV", b"b video")]);
    fs::create_dir_all(&lib).unwrap();
    let mut session = Session::new(&lib, None).unwrap();
    let mut log = Log::default();

    let run = session.sync_scans(vec![scan_elsewhere(&a), scan_elsewhere(&b)], &Cancel::new(), &mut log);

    let report = report(run);
    assert_inv6(report, &[&a, &b]);
    assert!(report.is_complete());
    let raw = a.join("DCIM/100_FUJI/DSCF0001.RAF");
    assert!(matches!(outcome(report, &raw), FileOutcome::Copied { .. }));
    assert!(matches!(
        outcome(report, &b.join("DCIM/100_FUJI/DSCF0001.RAF")),
        FileOutcome::DuplicateInSession { of } if *of == raw
    ));
    assert_eq!(library_files(&lib).len(), 3);
    assert_eq!(log.scanned, [a.clone(), b.clone()]);
    let last = log.events.iter().rev().find(|e| e.progress.phase == Phase::Copying).unwrap();
    assert!(matches!(last.kind, EventKind::Finished));
    let each: Vec<(u64, u64)> =
        last.progress.sources.iter().map(|s| (s.done.files, s.total.files)).collect();
    assert_eq!(each, [(2, 2), (1, 1)]);
}

/// INV-1 and INV-6 for every card of a session: two syncs, one of two cards
/// at once, leave every card as it was and report each card whole.
#[test]
fn inv1_and_inv6_hold_for_every_card_of_a_session() {
    let dir = TempDir::new();
    let (a, b, c, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("c"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin"), ("DCIM/100_FUJI/notes.txt", b"x")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin"), ("DCIM/101_FUJI/DSCF0002.HIF", b"heif")]);
    put(&c, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin"), ("DCIM/100_FUJI/DSCF0005.MP4", b"mp4")]);
    put(&c, &[("DCIM/100_FUJI/._DSCF0005.MP4", b"apple double")]);
    fs::create_dir_all(&lib).unwrap();
    let before: Vec<_> = [&a, &b, &c].iter().map(|card| manifest(card)).collect();
    let mut session = Session::new(&lib, Some("Trip")).unwrap();

    assert_inv6(report(sync(&mut session, &[&a, &b])), &[&a, &b]);
    assert_inv6(report(sync(&mut session, &[&c])), &[&c]);

    for (card, before) in [&a, &b, &c].iter().zip(before) {
        assert_eq!(manifest(card), before, "INV-1: {card:?} changed");
    }
    assert_eq!(session.verdict(), Verdict::Complete);
}

/// Stale temp files are removed once, before the session's first copy. A
/// temp file that shows up later may be a copy in flight: no later sync of
/// the session touches it.
#[test]
fn stale_temps_are_removed_once_per_session_before_its_first_copy() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"a raw")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0002.RAF", b"b raw")]);
    let stale = write_file(&lib, "2026-01-01/RAW/.photosync-tmp-1-0-DSCF0009.RAF", b"x", at(T0));
    let mut session = Session::new(&lib, None).unwrap();

    let first = report(sync(&mut session, &[&a]));
    assert_eq!(first.temps().removed, std::slice::from_ref(&stale));
    assert!(!stale.exists());
    let in_flight = write_file(&lib, "2026-01-01/RAW/.photosync-tmp-1-1-DSCF0010.RAF", b"y", at(T0));
    let second = report(sync(&mut session, &[&b]));

    assert!(second.temps().removed.is_empty(), "{:?}", second.temps());
    assert_eq!(fs::read(&in_flight).unwrap(), b"y");
    assert!(second.is_complete());
}

/// S005 fast check: the core refuses one card given twice, in any
/// spelling, before planning anything, scanned by the session or not.
#[test]
fn a_card_given_twice_in_any_spelling_is_refused_before_planning() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(&lib).unwrap();
    let mut slash = card.clone().into_os_string();
    slash.push("/");
    let slash = PathBuf::from(slash);
    let before = manifest(&lib);
    let mut session = Session::new(&lib, None).unwrap();

    let mut log = Log::default();
    let scanned = session.sync(&[card.clone(), slash.clone()], &Cancel::new(), &mut log);
    let twice = |run: &Run| match &run.end {
        RunEnd::Refused(SyncError::SourceTwice { first, again }) => (first.clone(), again.clone()),
        other => panic!("{other:?}"),
    };
    assert_eq!(twice(scanned), (card.clone(), slash.clone()));
    assert_eq!(scanned.verdict(), Verdict::Refused);
    assert!(log.events.is_empty(), "planning started");
    let given = session.sync_scans(
        vec![scan_elsewhere(&card), scan_elsewhere(&slash)],
        &Cancel::new(),
        &mut Log::default(),
    );
    assert_eq!(twice(given), (card.clone(), slash));

    assert_eq!(manifest(&lib), before);
    assert_eq!(session.verdict(), Verdict::Refused);
}

/// S006 fast check: the session's report carries the library and the
/// folder, and its totals merge every sync's report.
#[test]
fn the_session_report_carries_library_and_folder_and_merges_every_sync() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"a raw"), ("DCIM/100_FUJI/DSCF0001.JPG", b"a jpg")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"a raw"), ("DCIM/100_FUJI/DSCF0002.MOV", b"b mov")]);
    fs::create_dir_all(&lib).unwrap();
    let mut session = Session::new(&lib, Some("Tirana")).unwrap();

    sync(&mut session, &[&a]);
    sync(&mut session, &[&b]);

    assert_eq!(session.library(), lib);
    assert_eq!(session.name(), Some("Tirana"));
    let folder = lib.join(format!("{} - Tirana", date_command(T0)));
    assert_eq!(session.folder().unwrap().path(), folder);
    let mut sum = (0, 0, 0);
    for run in session.runs() {
        let t = report(run).totals();
        sum = (sum.0 + t.copied.files, sum.1 + t.skipped.files, sum.2 + t.supported().bytes);
    }
    let totals = session.totals();
    assert_eq!((totals.copied.files, totals.skipped.files, totals.supported().bytes), sum);
    assert_eq!((totals.copied.files, totals.skipped.files), (3, 1));
    let types = session.by_type();
    let raw = types[&photosync_core::MediaType::Raw];
    assert_eq!((raw.copied.files, raw.skipped.files), (1, 1));
    assert_eq!(types[&photosync_core::MediaType::Video].copied.files, 1);
}

/// S018 note: the session's verdict is the worst of every sync, so a later
/// sync, refused or complete, never hides an earlier one that was not
/// complete; each such sync can be named.
#[test]
fn the_session_keeps_the_worst_outcome_of_every_sync() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"a raw")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0002.RAF", b"b raw")]);
    fs::create_dir_all(&lib).unwrap();
    let mut session = Session::new(&lib, None).unwrap();

    sync(&mut session, &[&a]);
    assert_eq!(session.verdict(), Verdict::Complete);
    let gone = dir.0.join("gone");
    let refused = session.sync(std::slice::from_ref(&gone), &Cancel::new(), &mut Log::default());
    assert!(matches!(refused.end, RunEnd::Refused(SyncError::Scan(_))));
    assert_eq!(session.verdict(), Verdict::Refused, "worse than complete");
    // Another photo of that name in the session folder: b's copy fails.
    put(session.folder().unwrap().path(), &[("RAW/DSCF0002.RAF", b"other!")]);
    let failed = report(sync(&mut session, &[&b]));
    assert!(matches!(
        outcome(failed, &b.join("DCIM/100_FUJI/DSCF0002.RAF")),
        FileOutcome::Failed(FailReason::NameCollision { .. })
    ));
    assert_eq!(session.verdict(), Verdict::NotComplete);
    session.sync(std::slice::from_ref(&gone), &Cancel::new(), &mut Log::default());
    sync(&mut session, &[&a]);
    session.record_crash(std::slice::from_ref(&a), "index out of bounds".into());

    assert_eq!(session.verdict(), Verdict::NotComplete, "a later sync hid a failure");
    let named: Vec<(usize, Verdict)> = session.not_complete().map(|(i, run)| (i, run.verdict())).collect();
    assert_eq!(
        named,
        [(1, Verdict::Refused), (2, Verdict::NotComplete), (3, Verdict::Refused), (5, Verdict::NotComplete)]
    );
    assert!(matches!(&session.runs()[5].end, RunEnd::Crashed(what) if what == "index out of bounds"));
}

/// Only a session may point a file at an earlier sync's file: a plan that
/// claims one, handed to `Report::new`, is refused, as nothing there says
/// whether that file is on the drive.
#[test]
fn a_plan_pointing_at_an_earlier_sync_is_refused_outside_a_session() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("2026-01-01/RAW/DSCF0001.RAF", b"raw")]);
    let scans = vec![scan_elsewhere(&card)];
    let mut plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    let skipped = plan.skipped.pop().unwrap();
    let elsewhere = dir.0.join("old/DCIM/100_FUJI/DSCF0001.RAF");
    plan.seen_earlier.push(Duplicate { file: skipped.file, of: elsewhere.clone() });

    let outcome = photosync_core::CopyOutcome::nothing_to_copy(Default::default());
    match Report::new(scans, plan, outcome) {
        Err(ReportError::NoOriginal { of, .. }) => assert_eq!(of, elsewhere),
        other => panic!("expected NoOriginal, got {other:?}"),
    }
}

/// A name that is not one folder name is refused when the session is
/// made, before any card is read.
#[test]
fn an_invalid_name_is_refused_before_anything_runs() {
    let dir = TempDir::new();
    for name in ["", "  ", "a/b", "nul\0"] {
        match Session::new(&dir.0, Some(name)) {
            Err(FolderError::InvalidName(n)) => assert_eq!(n, name),
            other => panic!("{name:?}: {other:?}"),
        }
    }
    assert!(Session::new(&dir.0, Some("Tirana")).is_ok());
}

/// A cancel while planning gives that sync's cancelled report, writes
/// nothing, and the session goes on.
#[test]
fn a_sync_cancelled_while_planning_is_reported_and_the_session_goes_on() {
    let dir = TempDir::new();
    let (a, lib) = (dir.0.join("a"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("old/DSCF0001.RAF", b"raw")]);
    let before = manifest(&lib);
    let mut session = Session::new(&lib, None).unwrap();
    let cancel = Cancel::new();
    cancel.cancel();

    let cut = session.sync_scans(vec![scan_elsewhere(&a)], &cancel, &mut Log::default());
    assert_eq!(report(cut).totals().cancelled.files, 1);
    assert_eq!(manifest(&lib), before);
    assert!(session.folder().is_none());
    let again = report(sync(&mut session, &[&a]));
    assert_eq!(again.totals().skipped.files, 1);
    assert_eq!(session.verdict(), Verdict::NotComplete, "the cancel stays in the session");
}

/// S010 fast check: the same card synced again in one session is "already
/// on drive", not a "duplicate in session" of itself: its files are the
/// files that put the photos there.
#[test]
fn the_same_card_synced_again_is_already_on_drive_not_a_duplicate_of_itself() {
    let dir = TempDir::new();
    let (a, lib) = (dir.0.join("a"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw"), ("DCIM/100_FUJI/DSCF0001.JPG", b"jpg")]);
    put(&lib, &[("2026-01-01/JPEG/DSCF0001.JPG", b"jpg")]);
    let mut session = Session::new(&lib, None).unwrap();

    assert!(report(sync(&mut session, &[&a])).is_complete());
    let folder = session.folder().unwrap().path().to_path_buf();
    let again = report(sync(&mut session, &[&a]));

    for (name, on_drive) in [
        ("DSCF0001.RAF", folder.join("RAW/DSCF0001.RAF")),
        ("DSCF0001.JPG", lib.join("2026-01-01/JPEG/DSCF0001.JPG")),
    ] {
        match outcome(again, &a.join("DCIM/100_FUJI").join(name)) {
            FileOutcome::AlreadySynced { library_file } => assert_eq!(*library_file, on_drive),
            other => panic!("{name}: {other:?}"),
        }
    }
    assert!(again.is_complete());
}

/// S010 fast check: the session's memory names the right original. A
/// library file a later sync copied to a path an earlier sync had used
/// (the earlier copy was removed in between) holds the later sync's photo,
/// so a twin of it on a third card is a duplicate of the later card's file.
#[test]
fn a_library_path_reused_by_a_later_sync_names_that_sync_as_the_original() {
    let dir = TempDir::new();
    let (a, b, c, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("c"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"first camera")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"other camera")]);
    put(&c, &[("DCIM/100_FUJI/DSCF0001.RAF", b"other camera")]);
    fs::create_dir_all(&lib).unwrap();
    let mut session = Session::new(&lib, None).unwrap();

    assert!(report(sync(&mut session, &[&a])).is_complete());
    let copy = session.folder().unwrap().path().join("RAW/DSCF0001.RAF");
    fs::remove_file(&copy).unwrap();
    let second = report(sync(&mut session, &[&b]));
    assert!(matches!(outcome(second, &b.join("DCIM/100_FUJI/DSCF0001.RAF")), FileOutcome::Copied { to } if *to == copy));
    let third = report(sync(&mut session, &[&c]));

    match outcome(third, &c.join("DCIM/100_FUJI/DSCF0001.RAF")) {
        FileOutcome::DuplicateInSession { of } => assert_eq!(*of, b.join("DCIM/100_FUJI/DSCF0001.RAF")),
        other => panic!("{other:?}"),
    }
    assert!(third.is_complete());
}

/// S022: a front-end may record a sync's refusal itself, as its tests do
/// for a report that does not add up, which no real sync gives. Such a run
/// is never complete, so recording one cannot make the cards look safe.
#[test]
fn a_recorded_refusal_is_never_complete() {
    let card = PathBuf::from("/V/CARD");
    let mut session = Session::new(Path::new("/V/SSD/Photos"), None).unwrap();
    let report = SyncError::Report(ReportError::Unaccounted(card.join("DCIM/DSCF0001.RAF")));
    let run = session.record_refusal(std::slice::from_ref(&card), report);
    assert!(matches!(run.end, RunEnd::Refused(SyncError::Report(_))), "{run:?}");
    assert_eq!(run.sources, std::slice::from_ref(&card));
    assert_eq!(run.verdict(), Verdict::NotComplete);
    let twice = SyncError::SourceTwice { first: card.clone(), again: card.clone() };
    assert_eq!(session.record_refusal(std::slice::from_ref(&card), twice).verdict(), Verdict::Refused);
    assert_eq!(session.verdict(), Verdict::NotComplete);
    assert_eq!(session.not_complete().count(), 2);
}

/// F07 in a session: the twin the fallback copied is what the session
/// remembers, so the same photo on a later card is a duplicate in session
/// of it, the card whose file is on the drive now.
#[cfg(unix)]
#[test]
fn a_twin_the_fallback_copied_is_the_original_for_later_cards() {
    let dir = TempDir::new();
    let (a, b, c, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("c"), dir.0.join("lib"));
    for card in [&a, &b, &c] {
        put(card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"backup")]);
    }
    fs::create_dir_all(&lib).unwrap();
    let raf = |card: &Path| card.join("DCIM/100_FUJI/DSCF0001.RAF");
    let mut session = Session::new(&lib, None).unwrap();

    // Readable while planning, which scans and hashes first; not after.
    let scans = vec![scan_card(&a, 1), scan_card(&b, 2)];
    let unread = raf(&a);
    let mut lock = None;
    let mut watch = LockOnCopy { path: unread.clone(), lock: &mut lock };
    let first = session.sync_scans(scans, &Cancel::new(), &mut watch);
    drop(lock);
    let first = report(first);
    assert!(matches!(outcome(first, &raf(&b)), FileOutcome::Copied { .. }));
    assert!(matches!(outcome(first, &unread), FileOutcome::DuplicateUnread { of, .. } if *of == raf(&b)));
    assert!(first.is_complete());

    let later = report(sync(&mut session, &[&c]));
    assert!(matches!(outcome(later, &raf(&c)), FileOutcome::DuplicateInSession { of } if *of == raf(&b)));
    assert_eq!(session.verdict(), Verdict::Complete);
}

/// Makes `path` unreadable once copying starts.
#[cfg(unix)]
struct LockOnCopy<'l> {
    path: PathBuf,
    lock: &'l mut Option<common::Mode>,
}

#[cfg(unix)]
impl Watch for LockOnCopy<'_> {
    fn event(&mut self, event: Event) {
        if event.progress.phase == Phase::Copying && event.kind == EventKind::Started {
            *self.lock = Some(common::Mode::set(&self.path, 0o000));
        }
    }
}
