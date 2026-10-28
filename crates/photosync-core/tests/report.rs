//! S006, S014: the sync report. INV-6 per source over every kind of outcome,
//! reasons per entry, scan failures, totals per source and per type, and
//! the report of a cancelled plan and of a sync with nothing to copy. A
//! duplicate is skipped only if its original is on the drive; not flushed
//! is counted apart; inputs that would make the report lie are refused.

use std::path::{Path, PathBuf};

use photosync_core::{
    Bucket, Cancel, Copied, Counts, CopyOutcome, EventKind, FailReason, Failed, FileOutcome, MediaType, Monitor,
    PlanError, Report, ReportError, SourceReport, SourceScan, SyncPlan, copy_plan, plan_sync,
    remove_stale_temps, sync_folder,
};

mod common;
use common::{TempDir, at, date_command, scan_elsewhere, write_file};
#[cfg(unix)]
use common::Mode;

const T0: u64 = 1_761_650_000; // 2026-10-28, mid-day in most zones

fn put(root: &Path, files: &[(&str, &[u8])]) {
    for (rel, bytes) in files {
        write_file(root, rel, bytes, at(T0));
    }
}

fn scan(root: &Path) -> SourceScan {
    scan_elsewhere(root)
}

/// Plans and copies `cards` into `lib`. The copy is cancelled once the file
/// `cancel_after` is done, if given.
fn sync(lib: &Path, cards: &[&Path], cancel_after: Option<PathBuf>) -> Report {
    let scans: Vec<SourceScan> = cards.iter().map(|c| scan(c)).collect();
    let plan = plan_sync(lib, &scans, &mut Monitor::default()).unwrap();
    let outcome = copy(&plan, cancel_after);
    Report::new(scans, plan, outcome).unwrap()
}

fn copy(plan: &SyncPlan, cancel_after: Option<PathBuf>) -> CopyOutcome {
    let Some(folder) = sync_folder(plan, None).unwrap() else {
        return CopyOutcome::nothing_to_copy(remove_stale_temps(&plan.library));
    };
    let cancel = Cancel::new();
    let flag = cancel.clone();
    let mut monitor = Monitor::new(cancel, move |e| {
        if matches!(&e.kind, EventKind::FileDone(p) if Some(p) == cancel_after.as_ref()) {
            flag.cancel();
        }
    });
    copy_plan(plan, &folder, &mut monitor).unwrap()
}

/// Each listed file's path under the source root, with its bucket.
fn buckets(source: &SourceReport) -> Vec<(String, Bucket)> {
    source
        .files
        .iter()
        .map(|e| {
            let rel = e.file.path.strip_prefix(&source.root).unwrap();
            (rel.to_string_lossy().into_owned(), e.outcome.bucket())
        })
        .collect()
}

fn outcome<'r>(report: &'r Report, path: &Path) -> &'r FileOutcome {
    let entries = report.sources().iter().flat_map(|s| &s.files);
    &entries.into_iter().find(|e| e.file.path == path).expect("listed").outcome
}

/// INV-6 per source: copied + skipped + failed + cancelled equals the
/// supported files the scan listed, and every listed file is in the report
/// exactly once. `supported` is counted from fresh scans, apart from the
/// report.
fn assert_inv6(report: &Report, cards: &[&Path]) {
    assert_eq!(report.sources().len(), cards.len());
    for (source, card) in report.sources().iter().zip(cards) {
        let fresh = scan(card);
        let t = source.totals();
        assert_eq!(t.supported().files, fresh.supported().count() as u64, "{card:?}");
        assert_eq!(t.supported().bytes, fresh.supported().map(|f| f.size).sum::<u64>());
        assert_eq!(t.ignored.files, fresh.ignored().count() as u64);
        let listed: Vec<&Path> = source.files.iter().map(|e| e.file.path.as_path()).collect();
        let scanned: Vec<&Path> = fresh.files.iter().map(|f| f.path.as_path()).collect();
        assert_eq!(listed, scanned, "each listed file once, in path order");
    }
}

/// One sync that holds every outcome: copied, skipped (already on drive and
/// duplicate in session), failed (read error while planning and while
/// copying, name collision), ignored, cancelled, and a failed folder.
#[cfg(unix)]
#[test]
fn inv6_holds_per_source_over_every_kind_of_outcome() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    let session = lib.join(date_command(T0));
    put(&a, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"new"),
        ("DCIM/100_FUJI/DSCF0002.JPG", b"on drive"),
        ("DCIM/100_FUJI/DSCF0003.RAF", b"locked, unique"),
        ("DCIM/100_FUJI/DSCF0004.RAF", b"collides"),
        ("DCIM/100_FUJI/DSCF0005.MOV", b"twin"),
        ("DCIM/100_FUJI/DSCF0006.RAF", b"locked, candidate"),
        ("DCIM/100_FUJI/notes.txt", b"ignored"),
        ("DCIM/100_FUJI/._DSCF0001.RAF", b"ignored"),
        ("DCIM/101_FUJI/DSCF0005.MOV", b"twin"),
        ("DCIM/102_FUJI/DSCF0100.RAF", b"in a locked folder"),
    ]);
    put(&b, &[
        ("DCIM/100_FUJI/DSCF0005.MOV", b"twin"),
        ("DCIM/100_FUJI/DSCF0101.RAF", b"b new"),
        ("DCIM/100_FUJI/DSCF0102.HIF", b"cut off"),
        ("DCIM/100_FUJI/DSCF0103.JPG", b"cut off"),
    ]);
    put(&lib, &[
        ("2025-01-01/JPEG/DSCF0002.JPG", b"on drive"),
        ("2025-01-01/RAW/DSCF0006.RAF", b"other, candidate"),
    ]);
    // A different file of that name already in the session's RAW folder.
    write_file(&session, "RAW/DSCF0004.RAF", b"older photo, other size", at(T0));
    let card = |rel: &str| a.join("DCIM").join(rel);
    let _a = Mode::set(&card("100_FUJI/DSCF0003.RAF"), 0o000);
    let _b = Mode::set(&card("100_FUJI/DSCF0006.RAF"), 0o000);
    let _c = Mode::set(&card("102_FUJI"), 0o000);

    let report = sync(&lib, &[&a, &b], Some(b.join("DCIM/100_FUJI/DSCF0101.RAF")));

    // The fresh scans run with the locks on, as the sync's did: the locked
    // folder's file is in neither, so it is outside the sum.
    assert_inv6(&report, &[&a, &b]);
    let ta = report.sources()[0].totals();
    assert_eq!(ta.supported().files, 7);
    assert_eq!(report.sources()[1].totals().supported().files, 4);

    use Bucket::*;
    assert_eq!(
        buckets(&report.sources()[0]),
        [
            ("DCIM/100_FUJI/._DSCF0001.RAF".into(), Ignored),
            ("DCIM/100_FUJI/DSCF0001.RAF".into(), Copied),
            ("DCIM/100_FUJI/DSCF0002.JPG".into(), Skipped),
            ("DCIM/100_FUJI/DSCF0003.RAF".into(), Failed),
            ("DCIM/100_FUJI/DSCF0004.RAF".into(), Failed),
            ("DCIM/100_FUJI/DSCF0005.MOV".into(), Copied),
            ("DCIM/100_FUJI/DSCF0006.RAF".into(), Failed),
            ("DCIM/100_FUJI/notes.txt".into(), Ignored),
            ("DCIM/101_FUJI/DSCF0005.MOV".into(), Skipped),
        ]
    );
    assert_eq!(
        buckets(&report.sources()[1]),
        [
            ("DCIM/100_FUJI/DSCF0005.MOV".into(), Skipped),
            ("DCIM/100_FUJI/DSCF0101.RAF".into(), Copied),
            ("DCIM/100_FUJI/DSCF0102.HIF".into(), Cancelled),
            ("DCIM/100_FUJI/DSCF0103.JPG".into(), Cancelled),
        ]
    );

    // Every entry carries its reason.
    let raw = session.join("RAW");
    assert!(matches!(outcome(&report, &card("100_FUJI/DSCF0001.RAF")), FileOutcome::Copied { to } if *to == raw.join("DSCF0001.RAF")));
    assert!(matches!(outcome(&report, &card("100_FUJI/DSCF0002.JPG")), FileOutcome::AlreadySynced { library_file } if *library_file == lib.join("2025-01-01/JPEG/DSCF0002.JPG")));
    let denied = |r: &FileOutcome| matches!(r, FileOutcome::Failed(FailReason::Read(e)) if e.kind() == std::io::ErrorKind::PermissionDenied);
    assert!(denied(outcome(&report, &card("100_FUJI/DSCF0003.RAF"))), "read error while copying");
    assert!(denied(outcome(&report, &card("100_FUJI/DSCF0006.RAF"))), "read error while planning");
    assert!(matches!(outcome(&report, &card("100_FUJI/DSCF0004.RAF")), FileOutcome::Failed(FailReason::NameCollision { existing }) if *existing == raw.join("DSCF0004.RAF")));
    let twin = card("100_FUJI/DSCF0005.MOV");
    assert!(matches!(outcome(&report, &card("101_FUJI/DSCF0005.MOV")), FileOutcome::DuplicateInSession { of } if *of == twin));
    assert!(matches!(outcome(&report, &b.join("DCIM/100_FUJI/DSCF0005.MOV")), FileOutcome::DuplicateInSession { of } if *of == twin));
    assert!(matches!(outcome(&report, &card("100_FUJI/notes.txt")), FileOutcome::Ignored));

    // The failed folder: listed with its error, contents unknown, outside the sum.
    let [failed] = &report.sources()[0].unreadable[..] else { panic!("{:?}", report.sources()[0].unreadable) };
    assert_eq!(failed.error.path, card("102_FUJI"));
    assert_eq!(failed.error.source.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(failed.contents_unknown);
    assert!(report.sources()[1].unreadable.is_empty());

    // Totals per source, per type, and overall, in files and bytes.
    let c = |files: u64, bytes: usize| Counts { files, bytes: bytes as u64 };
    assert_eq!((ta.copied, ta.skipped, ta.failed, ta.cancelled, ta.ignored), (
        c(2, "new".len() + "twin".len()),
        c(2, "on drive".len() + "twin".len()),
        c(3, "locked, unique".len() + "collides".len() + "locked, candidate".len()),
        c(0, 0),
        c(2, 2 * "ignored".len()),
    ));
    let tb = report.sources()[1].totals();
    assert_eq!((tb.copied.files, tb.skipped.files, tb.failed.files, tb.cancelled.files), (1, 1, 0, 2));
    let types = report.by_type();
    let t = |m| types[&m];
    assert_eq!((t(MediaType::Raw).copied.files, t(MediaType::Raw).failed.files), (2, 3));
    assert_eq!((t(MediaType::Jpeg).skipped.files, t(MediaType::Jpeg).cancelled.files), (1, 1));
    assert_eq!(t(MediaType::Heif).cancelled.files, 1);
    assert_eq!((t(MediaType::Video).copied.files, t(MediaType::Video).skipped.files), (1, 2));
    let all = report.totals();
    assert_eq!(all.supported().files, 7 + 4);
    let typed: u64 = types.values().map(|t| t.supported().files).sum();
    assert_eq!(typed, all.supported().files, "per type covers every supported file");
    assert_eq!(all.ignored.files, 2);
    assert_eq!(report.sources()[0].by_type()[&MediaType::Video].skipped.files, 1);
}

/// INV-6 without permission faults, so it also runs on exFAT: a name
/// collision, duplicates in session, ignored files and a cancelled copy.
#[test]
fn inv6_holds_with_collisions_duplicates_ignored_and_cancelled() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"collides"),
        ("DCIM/100_FUJI/DSCF0002.JPG", b"twin"),
        ("DCIM/100_FUJI/DSCF0003.JPG", b"new"),
        ("DCIM/100_FUJI/IMG.XMP", b"ignored"),
    ]);
    put(&b, &[
        ("DCIM/100_FUJI/DSCF0002.JPG", b"twin"),
        ("DCIM/100_FUJI/DSCF0004.RAF", b"cut off"),
    ]);
    write_file(&lib.join(date_command(T0)), "RAW/DSCF0001.RAF", b"another photo", at(T0));

    let report = sync(&lib, &[&a, &b], Some(a.join("DCIM/100_FUJI/DSCF0003.JPG")));

    assert_inv6(&report, &[&a, &b]);
    use Bucket::*;
    assert_eq!(
        buckets(&report.sources()[0]),
        [
            ("DCIM/100_FUJI/DSCF0001.RAF".into(), Failed),
            ("DCIM/100_FUJI/DSCF0002.JPG".into(), Copied),
            ("DCIM/100_FUJI/DSCF0003.JPG".into(), Copied),
            ("DCIM/100_FUJI/IMG.XMP".into(), Ignored),
        ]
    );
    assert_eq!(
        buckets(&report.sources()[1]),
        [("DCIM/100_FUJI/DSCF0002.JPG".into(), Skipped), ("DCIM/100_FUJI/DSCF0004.RAF".into(), Cancelled)]
    );
    assert!(matches!(
        outcome(&report, &a.join("DCIM/100_FUJI/DSCF0001.RAF")),
        FileOutcome::Failed(FailReason::NameCollision { .. })
    ));
    assert!(matches!(
        outcome(&report, &b.join("DCIM/100_FUJI/DSCF0002.JPG")),
        FileOutcome::DuplicateInSession { .. }
    ));
}

/// Scan failures are listed per entry with their error. Only a folder's
/// contents are unknown; an unreadable file is just that entry.
#[cfg(unix)]
#[test]
fn unreadable_scan_entries_are_listed_and_only_folders_have_unknown_contents() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"fine"),
        ("DCIM/101_FUJI/DSCF0002.RAF", b"no metadata"),
        ("DCIM/102_FUJI/DSCF0003.RAF", b"not listed"),
    ]);
    std::fs::create_dir_all(&lib).unwrap();
    // Listable but not searchable: names known, metadata not.
    let _a = Mode::set(&card.join("DCIM/101_FUJI"), 0o444);
    let _b = Mode::set(&card.join("DCIM/102_FUJI"), 0o000);
    assert_eq!(scan(&card).failed_folders, [card.join("DCIM/102_FUJI")]);

    let report = sync(&lib, &[&card], None);

    assert_inv6(&report, &[&card]);
    let got: Vec<(PathBuf, bool)> = report.sources()[0]
        .unreadable
        .iter()
        .map(|u| (u.error.path.clone(), u.contents_unknown))
        .collect();
    assert_eq!(
        got,
        [(card.join("DCIM/101_FUJI/DSCF0002.RAF"), false), (card.join("DCIM/102_FUJI"), true)]
    );
    assert_eq!(buckets(&report.sources()[0]), [("DCIM/100_FUJI/DSCF0001.RAF".into(), Bucket::Copied)]);
}

/// A cancelled plan wrote nothing: every supported file is cancelled, and
/// ignored files and scan failures are still listed.
#[test]
fn cancelled_plan_reports_every_supported_file_as_cancelled() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"one"), ("DCIM/100_FUJI/x.txt", b"ignored")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0002.JPG", b"two")]);
    std::fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan(&a), scan(&b)];
    let cancel = Cancel::new();
    cancel.cancel();
    let result = plan_sync(&lib, &scans, &mut Monitor::new(cancel, |_| {}));
    assert!(matches!(result, Err(PlanError::Cancelled)), "{result:?}");

    let report = Report::plan_cancelled(scans);

    assert_inv6(&report, &[&a, &b]);
    assert_eq!(
        buckets(&report.sources()[0]),
        [("DCIM/100_FUJI/DSCF0001.RAF".into(), Bucket::Cancelled), ("DCIM/100_FUJI/x.txt".into(), Bucket::Ignored)]
    );
    assert_eq!(report.totals().cancelled, Counts { files: 2, bytes: 6 });
    assert!(report.library_failures().is_empty() && report.temps().removed.is_empty());
}

/// With nothing to copy there is no session folder and no `copy_plan`; the
/// report still lists every file, and the stale temp cleanup.
#[test]
fn nothing_to_copy_still_reports_every_file_and_the_temp_cleanup() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"synced"), ("DCIM/100_FUJI/x.txt", b"ignored")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"synced"), ("2025-01-01/RAW/.photosync-tmp-1-0-a", b"stale")]);

    let report = sync(&lib, &[&card], None);

    assert_inv6(&report, &[&card]);
    assert_eq!(
        buckets(&report.sources()[0]),
        [("DCIM/100_FUJI/DSCF0001.RAF".into(), Bucket::Skipped), ("DCIM/100_FUJI/x.txt".into(), Bucket::Ignored)]
    );
    assert_eq!(report.temps().removed, [lib.join("2025-01-01/RAW/.photosync-tmp-1-0-a")]);
}

/// A plan or outcome of other scans would break INV-6, so it is refused.
#[test]
fn report_refuses_a_plan_or_outcome_that_does_not_match_the_scans() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"one")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0002.RAF", b"two")]);
    std::fs::create_dir_all(&lib).unwrap();
    let plan_of = |root: &Path| plan_sync(&lib, &[scan(root)], &mut Monitor::default()).unwrap();
    let empty = || CopyOutcome::nothing_to_copy(Default::default());

    // The plan's files are not copied: unaccounted.
    let err = Report::new(vec![scan(&a)], plan_of(&a), empty()).unwrap_err();
    assert!(matches!(&err, ReportError::Unaccounted(p) if *p == a.join("DCIM/100_FUJI/DSCF0001.RAF")), "{err}");

    // An outcome of another card's plan: not in the scans.
    let plan_b = plan_of(&b);
    let outcome_b = copy(&plan_b, None);
    let err = Report::new(vec![scan(&a)], plan_of(&a), outcome_b).unwrap_err();
    assert!(matches!(&err, ReportError::NotInScans(p) if *p == b.join("DCIM/100_FUJI/DSCF0002.RAF")), "{err}");
}

/// A duplicate is safe only if its original is. One whose original failed
/// to read while copying is failed too, never skipped: the card holds the
/// only copy.
#[cfg(unix)]
#[test]
fn duplicate_of_an_original_that_failed_to_read_is_failed_not_skipped() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin")]);
    std::fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan(&a), scan(&b)];
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    // Hashed while planning, so the twin is a duplicate; unreadable by the copy.
    let original = a.join("DCIM/100_FUJI/DSCF0001.RAF");
    let _lock = Mode::set(&original, 0o000);
    let copied = copy(&plan, None);

    let report = Report::new(scans, plan, copied).unwrap();

    assert_inv6(&report, &[&a, &b]);
    assert!(matches!(outcome(&report, &original), FileOutcome::Failed(FailReason::Read(_))));
    assert_eq!(buckets(&report.sources()[1]), [("DCIM/100_FUJI/DSCF0001.RAF".into(), Bucket::Failed)]);
    assert!(matches!(
        outcome(&report, &b.join("DCIM/100_FUJI/DSCF0001.RAF")),
        FileOutcome::DuplicateOfFailed { of } if *of == original
    ));
    assert_eq!(report.totals().skipped.files, 0);
}

/// The same for an original that collides by name or is cut off by a
/// cancel: the duplicate takes the original's bucket.
#[test]
fn duplicate_of_a_colliding_or_cancelled_original_takes_its_bucket() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"twin raw"),
        ("DCIM/100_FUJI/DSCF0002.JPG", b"new"),
        ("DCIM/100_FUJI/DSCF0003.JPG", b"twin jpeg"),
    ]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin raw"), ("DCIM/100_FUJI/DSCF0003.JPG", b"twin jpeg")]);
    write_file(&lib.join(date_command(T0)), "RAW/DSCF0001.RAF", b"another photo", at(T0));

    let report = sync(&lib, &[&a, &b], Some(a.join("DCIM/100_FUJI/DSCF0002.JPG")));

    assert_inv6(&report, &[&a, &b]);
    use Bucket::*;
    assert_eq!(
        buckets(&report.sources()[0]),
        [
            ("DCIM/100_FUJI/DSCF0001.RAF".into(), Failed),
            ("DCIM/100_FUJI/DSCF0002.JPG".into(), Copied),
            ("DCIM/100_FUJI/DSCF0003.JPG".into(), Cancelled),
        ]
    );
    assert_eq!(
        buckets(&report.sources()[1]),
        [("DCIM/100_FUJI/DSCF0001.RAF".into(), Failed), ("DCIM/100_FUJI/DSCF0003.JPG".into(), Cancelled)]
    );
    let card = |root: &Path, rel: &str| root.join("DCIM/100_FUJI").join(rel);
    assert!(matches!(
        outcome(&report, &card(&a, "DSCF0001.RAF")),
        FileOutcome::Failed(FailReason::NameCollision { .. })
    ));
    assert!(matches!(
        outcome(&report, &card(&b, "DSCF0001.RAF")),
        FileOutcome::DuplicateOfFailed { of } if *of == card(&a, "DSCF0001.RAF")
    ));
    assert!(matches!(
        outcome(&report, &card(&b, "DSCF0003.JPG")),
        FileOutcome::DuplicateOfCancelled { of } if *of == card(&a, "DSCF0003.JPG")
    ));
    assert_eq!(report.totals().skipped.files, 0);
}

/// A copy in place whose folder flush failed stays failed, and the tally
/// counts it apart, so a front-end can say "in place, flush not confirmed".
#[test]
fn not_flushed_is_failed_and_counted_apart() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"in place"), ("DCIM/100_FUJI/DSCF0002.JPG", b"not written")]);
    std::fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan(&card)];
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    // No hook can fail a folder flush from outside the core, so the outcome
    // is built by hand.
    let [raf, jpg] = [plan.to_copy[0].file.clone(), plan.to_copy[1].file.clone()];
    let error = || std::io::Error::other("injected");
    let failed = vec![
        Failed { file: raf, reason: FailReason::NotFlushed { to: lib.join("x/RAW/DSCF0001.RAF"), error: error() } },
        Failed { file: jpg, reason: FailReason::Write { path: lib.join("x/JPEG"), error: error() } },
    ];
    let copied = CopyOutcome { copied: Vec::new(), failed, cancelled: Vec::new(), temps: Default::default() };

    let report = Report::new(scans, plan, copied).unwrap();

    assert_inv6(&report, &[&card]);
    let all = report.totals();
    assert_eq!(all.failed, Counts { files: 2, bytes: ("in place".len() + "not written".len()) as u64 });
    assert_eq!(all.not_flushed, Counts { files: 1, bytes: "in place".len() as u64 });
    assert_eq!(all.supported().files, 2, "not flushed is inside failed, not added to it");
    let types = report.by_type();
    assert_eq!((types[&MediaType::Raw].not_flushed.files, types[&MediaType::Jpeg].not_flushed.files), (1, 0));
    assert_eq!(report.sources()[0].totals(), all);
}

/// A source given twice, in any spelling, would list its files twice and
/// let entries land on the other twin, so it is refused.
#[cfg(unix)]
#[test]
fn report_refuses_a_source_root_given_twice_in_any_spelling() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"one")]);
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::create_dir_all(dir.0.join("other")).unwrap();
    std::os::unix::fs::symlink(&card, dir.0.join("link")).unwrap();

    let mut twins = vec![card.clone(), dir.0.join("other/../card"), dir.0.join("link"), dir.0.join("link/")];
    // Another case is the same folder only where the file system ignores case.
    if dir.0.join("CARD").exists() {
        twins.push(dir.0.join("CARD"));
    }
    for twin in twins {
        let scans = vec![scan(&card), scan(&twin)];
        let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
        let copied = copy(&plan, None);
        let err = Report::new(scans, plan, copied).unwrap_err();
        assert!(matches!(&err, ReportError::SourceTwice(root) if *root == twin), "{twin:?}: {err}");
    }
}

/// A plan or outcome made from another scan of the card (a file changed in
/// between) is refused: its entries would not describe the listed files.
#[test]
fn report_refuses_a_plan_or_outcome_whose_size_or_mtime_differs_from_the_scan() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"on drive"), ("DCIM/100_FUJI/DSCF0002.RAF", b"new")]);
    put(&lib, &[("2025-01-01/RAW/DSCF0001.RAF", b"on drive")]);
    const SECOND: std::time::Duration = std::time::Duration::from_secs(1);
    type Tweak = dyn Fn(&mut SyncPlan, &mut CopyOutcome);
    let build = |tweak: &Tweak| {
        let scans = vec![scan(&card)];
        let mut plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
        assert_eq!((plan.skipped.len(), plan.to_copy.len()), (1, 1));
        // Built by hand, so nothing is written and every case starts alike.
        let copied = Copied { file: plan.to_copy[0].file.clone(), to: lib.join("x/RAW/DSCF0002.RAF") };
        let mut outcome = CopyOutcome::nothing_to_copy(Default::default());
        outcome.copied.push(copied);
        tweak(&mut plan, &mut outcome);
        Report::new(scans, plan, outcome)
    };

    assert!(build(&|_, _| {}).is_ok(), "untweaked, it is accepted");
    let cases: [(&str, &Tweak); 4] = [
        ("plan size", &|p, _| p.skipped[0].file.size += 1),
        ("plan mtime", &|p, _| p.skipped[0].file.modified += SECOND),
        ("outcome size", &|_, o| o.copied[0].file.size += 1),
        ("outcome mtime", &|_, o| o.copied[0].file.modified += SECOND),
    ];
    let skipped = card.join("DCIM/100_FUJI/DSCF0001.RAF");
    let copied = card.join("DCIM/100_FUJI/DSCF0002.RAF");
    for ((name, tweak), path) in cases.into_iter().zip([&skipped, &skipped, &copied, &copied]) {
        let err = build(tweak).unwrap_err();
        assert!(matches!(&err, ReportError::Changed(p) if p == path), "{name}: {err}");
    }

    // The real case: the card's file changed after planning, then rescanned.
    let scans = vec![scan(&card)];
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"on drive", at(T0 + 60));
    let mut outcome = CopyOutcome::nothing_to_copy(Default::default());
    outcome.copied.push(Copied { file: plan.to_copy[0].file.clone(), to: lib.join("x/RAW/DSCF0002.RAF") });
    let err = Report::new(vec![scan(&card)], plan, outcome).unwrap_err();
    assert!(matches!(&err, ReportError::Changed(p) if *p == skipped), "{err}");
}

/// A duplicate whose original the plan and outcome do not account for
/// cannot be called safe, so the report is refused.
#[test]
fn report_refuses_a_duplicate_whose_original_is_not_reported() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"twin")]);
    std::fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan(&a), scan(&b)];
    let mut plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    let original = a.join("DCIM/100_FUJI/DSCF0001.RAF");
    assert_eq!(plan.duplicates[0].of, original);
    // Points at a file no scan holds but nothing reports.
    plan.duplicates[0].of = lib.join("elsewhere.RAF");
    let copied = copy(&plan, None);

    let err = Report::new(scans, plan, copied).unwrap_err();

    let dup = b.join("DCIM/100_FUJI/DSCF0001.RAF");
    assert!(matches!(&err, ReportError::NoOriginal { file, of } if *file == dup && *of == lib.join("elsewhere.RAF")), "{err}");
}
