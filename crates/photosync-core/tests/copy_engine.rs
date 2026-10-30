//! S004: executing a sync plan. Session folder naming and reuse, type
//! folders, name collisions, stale temp files, and INV-1, INV-2, INV-4,
//! INV-5 around real copies. Fault injection tests (INV-3, INV-7, the
//! rename race) are unit tests in src/copy.rs, where the test hook lives.

use std::fs;
use std::path::{Path, PathBuf};

use photosync_core::{
    CopyError, CopyOutcome, FailReason, FileOutcome, FolderError, Monitor, Report, SyncFolder,
    SyncPlan, copy_plan, plan_sync, remove_stale_temps, sync_folder,
};

mod common;
use common::{TempDir, at, date_command, manifest, scan_card, scan_elsewhere, write_file};
#[cfg(unix)]
use common::Mode;

const T0: u64 = 1_761_650_000; // 2026-10-28, mid-day in most zones

fn put(root: &Path, files: &[(&str, &[u8])]) {
    for (rel, bytes) in files {
        write_file(root, rel, bytes, at(T0));
    }
}

fn plan(card: &Path, lib: &Path) -> SyncPlan {
    plan_sync(lib, &[scan_elsewhere(card)], &mut Monitor::default()).unwrap()
}

fn folder(plan: &SyncPlan, name: Option<&str>) -> SyncFolder {
    sync_folder(plan, name).unwrap().expect("a folder")
}

/// Plans, fixes the folder and copies: one whole sync.
fn sync(card: &Path, lib: &Path, name: Option<&str>) -> (SyncPlan, CopyOutcome) {
    let plan = plan(card, lib);
    let outcome = match sync_folder(&plan, name).unwrap() {
        Some(folder) => copy_plan(&plan, &folder, &mut Monitor::default()).unwrap(),
        None => CopyOutcome::nothing_to_copy(remove_stale_temps(lib)),
    };
    (plan, outcome)
}

fn copied_to(outcome: &CopyOutcome) -> Vec<PathBuf> {
    outcome.copied.iter().map(|c| c.to.clone()).collect()
}

fn temps_under(dir: &Path) -> Vec<PathBuf> {
    manifest(dir)
        .into_keys()
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(".photosync-tmp-"))
        .collect()
}

#[test]
fn folder_is_the_earliest_new_file_date_with_the_name() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"old, synced", at(T0 - 30 * 86_400));
    write_file(&card, "DCIM/100_FUJI/DSCF0002.RAF", b"later", at(T0 + 86_400));
    write_file(&card, "DCIM/100_FUJI/DSCF0003.RAF", b"earliest new", at(T0));
    // Already synced, so its earlier date does not count.
    write_file(&lib, "2026-09-01/RAW/DSCF0001.RAF", b"old, synced", at(T0));
    let plan = plan(&card, &lib);
    let date = date_command(T0);

    assert_eq!(folder(&plan, Some("Tirana")).path(), lib.join(format!("{date} - Tirana")));
    assert_eq!(folder(&plan, None).path(), lib.join(&date));
    // Fixing the folder writes nothing.
    assert!(!lib.join(&date).exists() && !lib.join(format!("{date} - Tirana")).exists());

    let outcome = copy_plan(&plan, &folder(&plan, Some("Tirana")), &mut Monitor::default()).unwrap();
    let session = lib.join(format!("{date} - Tirana/RAW"));
    assert_eq!(copied_to(&outcome), [session.join("DSCF0002.RAF"), session.join("DSCF0003.RAF")]);
    assert_eq!(fs::read(session.join("DSCF0003.RAF")).unwrap(), b"earliest new");
}

#[test]
fn existing_folder_with_the_name_is_reused_whatever_its_date() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(lib.join("2026-10-30 - Tirana")).unwrap();
    fs::create_dir_all(lib.join("2026-10-01 - Tirana")).unwrap();
    fs::create_dir_all(lib.join("2026-09-01 - Tirana trip")).unwrap(); // another name
    fs::create_dir_all(lib.join("2026-9-1 - Tirana")).unwrap(); // not a date
    fs::write(lib.join("2026-08-01 - Tirana"), b"a file, not a folder").unwrap();
    let plan = plan(&card, &lib);

    // DEC-009, with the earliest of several (DEC-017).
    assert_eq!(folder(&plan, Some("Tirana")).path(), lib.join("2026-10-01 - Tirana"));
    assert_eq!(folder(&plan, None).path(), lib.join(date_command(T0)));
    assert_eq!(folder(&plan, Some("trip")).path(), lib.join(format!("{} - trip", date_command(T0))));
}

#[test]
fn rerun_with_later_new_files_lands_in_the_same_named_folder() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    fs::create_dir_all(&lib).unwrap();
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"day one", at(T0));
    let (_, first) = sync(&card, &lib, Some("Tirana"));
    // As if the first run stopped before the later files.
    write_file(&card, "DCIM/100_FUJI/DSCF0002.RAF", b"day three", at(T0 + 2 * 86_400));
    let (_, second) = sync(&card, &lib, Some("Tirana"));

    let session = lib.join(format!("{} - Tirana/RAW", date_command(T0)));
    assert_eq!(copied_to(&first), [session.join("DSCF0001.RAF")]);
    assert_eq!(copied_to(&second), [session.join("DSCF0002.RAF")]);
}

const NFC: &str = "Tiran\u{eb}";
const NFD: &str = "Tirane\u{308}";

/// The names in `lib`, as the filesystem lists them: HFS+ and exFAT on macOS
/// list `NFD` whichever form the folder was made in.
fn listed(lib: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(lib).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

/// F02. Under plain `cargo test` on APFS this passes either way, as APFS
/// lists the form it was given; `scripts/test_exfat.sh` runs it on exFAT and
/// HFS+, where the listing differs from the name.
#[test]
fn accented_name_rerun_on_a_later_date_lands_in_the_first_folder() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    fs::create_dir_all(&lib).unwrap();
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"day one", at(T0));
    let (_, first) = sync(&card, &lib, Some(NFC));
    assert_eq!(first.copied.len(), 1);
    write_file(&card, "DCIM/100_FUJI/DSCF0002.RAF", b"a month on", at(T0 + 30 * 86_400));
    let (_, second) = sync(&card, &lib, Some(NFC));
    assert_eq!(second.copied.len(), 1);

    let folders = listed(&lib);
    assert_eq!(folders.len(), 1, "one trip split across folders: {folders:?}");
    let date = date_command(T0);
    assert!(folders[0].starts_with(&date), "{folders:?}");
    let raw = lib.join(&folders[0]).join("RAW");
    assert_eq!(fs::read(raw.join("DSCF0001.RAF")).unwrap(), b"day one");
    assert_eq!(fs::read(raw.join("DSCF0002.RAF")).unwrap(), b"a month on");
}

/// The filesystem decides, not a table of ours: what it resolves to the
/// listed folder is that folder, what it does not is another name. A library
/// on Linux ext4 takes the second arm, macOS the first for the two forms.
#[test]
fn name_in_another_spelling_reuses_a_folder_only_if_the_filesystem_resolves_it() {
    for (made, given) in [(NFC, NFD), (NFD, NFC), ("Tirana", "tirana")] {
        let dir = TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
        fs::create_dir_all(lib.join(format!("2026-06-01 - {made}"))).unwrap();
        let resolves = fs::symlink_metadata(lib.join(format!("2026-06-01 - {given}"))).is_ok();
        #[cfg(target_os = "macos")]
        assert!(resolves || made == "Tirana", "macOS filesystems ignore the form: {given:?}");
        let plan = plan(&card, &lib);

        let want = if resolves {
            lib.join(&listed(&lib)[0])
        } else {
            lib.join(format!("{} - {given}", date_command(T0)))
        };
        assert_eq!(folder(&plan, Some(given)).path(), want, "{made:?} then {given:?}");
    }
}

/// The lookup must say which folder it found: same date is not same folder.
#[test]
fn only_the_folder_the_name_resolves_to_is_reused() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    for other in ["2026-06-01 - Other", "2026-05-01 - Tirana", "2026-05-02 - Tirane"] {
        fs::create_dir_all(lib.join(other)).unwrap();
    }
    fs::create_dir_all(lib.join(format!("2026-06-01 - {NFC}"))).unwrap();
    // Named as the sync is, in the other form, but not folders.
    fs::write(lib.join(format!("2026-03-01 - {NFD}")), b"a file").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("elsewhere", lib.join(format!("2026-04-01 - {NFD}"))).unwrap();
    let plan = plan(&card, &lib);
    let spelt_as_listed = listed(&lib)
        .into_iter()
        .find(|f| f.starts_with("2026-06-01 - Tiran"))
        .unwrap();

    assert_eq!(folder(&plan, Some(NFC)).path(), lib.join(spelt_as_listed));
}

/// A symlink under the sync's name points at another folder: following it
/// would put the sync in a folder of another name.
#[cfg(unix)]
#[test]
fn a_symlink_with_the_name_does_not_make_its_target_the_folder() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(lib.join("2026-06-01 - Other")).unwrap();
    std::os::unix::fs::symlink("2026-06-01 - Other", lib.join("2026-06-01 - Tirana")).unwrap();
    let plan = plan(&card, &lib);

    assert_eq!(
        folder(&plan, Some("Tirana")).path(),
        lib.join(format!("{} - Tirana", date_command(T0)))
    );
}

/// Too long for the filesystem is an answer, "no such folder", not an error:
/// S031 refuses such a name, and until then it fails as it did.
#[test]
fn name_too_long_for_a_lookup_reuses_nothing_and_is_not_an_error() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(lib.join("2026-06-01 - Tirana")).unwrap();
    let plan = plan(&card, &lib);
    let long = "x".repeat(300);

    assert_eq!(
        folder(&plan, Some(&long)).path(),
        lib.join(format!("{} - {long}", date_command(T0)))
    );
}

/// A lookup that fails says nothing about the folder, so the sync does not
/// go on as if the name were new: that would split the trip unseen.
#[cfg(unix)]
#[test]
fn a_lookup_the_library_refuses_is_an_error() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(lib.join("2026-06-01 - Other")).unwrap();
    let plan = plan(&card, &lib);
    // May list the library, not look anything up in it.
    let _locked = Mode::set(&lib, 0o400);

    let err = sync_folder(&plan, Some("Tirana")).unwrap_err();
    assert!(matches!(&err, FolderError::Io(e) if e.path == lib), "{err:?}");
}

#[test]
fn nothing_to_copy_and_no_named_folder_gives_no_folder() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    put(&lib, &[("2026-01-01/RAW/DSCF0001.RAF", b"raw")]);
    let plan = plan(&card, &lib);

    assert!(sync_folder(&plan, None).unwrap().is_none());
    assert!(sync_folder(&plan, Some("Tirana")).unwrap().is_none());
}

#[test]
fn sync_name_that_is_not_one_folder_name_is_refused() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(&lib).unwrap();
    let plan = plan(&card, &lib);

    for name in ["", "  ", "a/b", "../up", "nul\0"] {
        assert!(
            matches!(sync_folder(&plan, Some(name)), Err(FolderError::InvalidName(n)) if n == name),
            "{name:?}"
        );
    }
}

#[test]
fn inv5_each_type_lands_in_its_own_folder() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"raw"),
        ("DCIM/100_FUJI/DSCF0001.JPG", b"jpg"),
        ("DCIM/100_FUJI/DSCF0002.jpeg", b"jpeg"),
        ("DCIM/100_FUJI/DSCF0003.HIF", b"hif"),
        ("DCIM/100_FUJI/DSCF0004.heif", b"heif"),
        ("DCIM/100_FUJI/DSCF0005.MOV", b"mov"),
        ("DCIM/101_FUJI/DSCF0006.mp4", b"mp4"),
        ("DCIM/100_FUJI/NOTES.TXT", b"ignored"),
    ]);
    fs::create_dir_all(&lib).unwrap();

    let (_, outcome) = sync(&card, &lib, None);

    let session = lib.join(date_command(T0));
    let mut found: Vec<(String, PathBuf)> = manifest(&session)
        .into_iter()
        .filter_map(|(p, (_, bytes))| {
            bytes.map(|b| (String::from_utf8(b).unwrap(), p.strip_prefix(&session).unwrap().into()))
        })
        .collect();
    found.sort();
    let want: Vec<(String, PathBuf)> = [
        ("heif", "HEIF/DSCF0004.heif"),
        ("hif", "HEIF/DSCF0003.HIF"),
        ("jpeg", "JPEG/DSCF0002.jpeg"),
        ("jpg", "JPEG/DSCF0001.JPG"),
        ("mov", "VIDEO/DSCF0005.MOV"),
        ("mp4", "VIDEO/DSCF0006.mp4"),
        ("raw", "RAW/DSCF0001.RAF"),
    ]
    .into_iter()
    .map(|(b, p)| (b.to_string(), p.into()))
    .collect();
    assert_eq!(found, want);
    assert_eq!(outcome.copied.len(), 7);
    assert!(outcome.failed.is_empty());
}

#[test]
fn same_name_different_content_in_the_target_folder_fails_as_name_collision() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[("DCIM/101_FUJI/DSCF0001.JPG", b"new shot")]);
    // After 1980: exFAT stores no mtime before that (scripts/test_exfat.sh).
    let existing =
        write_file(&lib, "2026-01-01 - Tirana/JPEG/DSCF0001.JPG", b"old shot", at(1_600_000_000));

    let (plan, outcome) = sync(&card, &lib, Some("Tirana"));

    assert_eq!(plan.to_copy.len(), 1, "same size, other hash: not already synced");
    assert!(outcome.copied.is_empty());
    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert_eq!(failed.file.path, card.join("DCIM/101_FUJI/DSCF0001.JPG"));
    assert!(matches!(&failed.reason, FailReason::NameCollision { existing: e } if *e == existing));
    assert!(failed.reason.to_string().starts_with("name collision"));
    assert_eq!(fs::read(&existing).unwrap(), b"old shot");
    assert_eq!(fs::metadata(&existing).unwrap().modified().unwrap(), at(1_600_000_000));
    assert!(temps_under(&lib).is_empty());
}

#[test]
fn inv4_second_sync_copies_nothing_whatever_folder_is_picked() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.JPG", b"jpg"),
        ("DCIM/100_FUJI/DSCF0001.RAF", b"raw"),
        ("DCIM/101_FUJI/DSCF0002.MOV", b"mov"),
    ]);
    fs::create_dir_all(&lib).unwrap();
    let (_, first) = sync(&card, &lib, Some("Tirana"));
    assert_eq!(first.copied.len(), 3);
    let after_first = manifest(&lib);

    for name in [Some("Tirana"), Some("Other"), None] {
        let (plan, again) = sync(&card, &lib, name);
        assert!(again.copied.is_empty() && again.failed.is_empty(), "{name:?}: {again:?}");
        assert_eq!(plan.skipped.len(), 3, "{name:?}");
        assert_eq!(manifest(&lib), after_first, "{name:?}");
    }
}

/// Audit F01: a counter wrap leaves two different `DSCF0001.JPG` of one
/// size. Syncing again into the same folder must not copy anything.
#[test]
fn inv4_same_sync_with_a_name_collision_copies_nothing_the_second_time() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.JPG", b"before wrap"),
        ("DCIM/101_FUJI/DSCF0001.JPG", b"after  wrap"),
        ("DCIM/101_FUJI/DSCF0002.RAF", b"raw"),
    ]);
    fs::create_dir_all(&lib).unwrap();
    let (_, first) = sync(&card, &lib, Some("Tirana"));
    assert_eq!(first.copied.len(), 2);
    assert_eq!(first.failed.len(), 1);
    let after_first = manifest(&lib);

    for _ in 0..3 {
        let (plan, again) = sync(&card, &lib, Some("Tirana"));
        assert!(again.copied.is_empty(), "{:?}", again.copied);
        assert_eq!(plan.skipped.len(), 2);
        assert!(matches!(
            &again.failed[..],
            [f] if matches!(f.reason, FailReason::NameCollision { .. })
                && f.file.path == card.join("DCIM/101_FUJI/DSCF0001.JPG")
        ));
        assert_eq!(manifest(&lib), after_first);
    }

    // Layout: the user can re-sync the collided file under another name, and
    // only that file is copied (DEC-013, DEC-017).
    let (_, other) = sync(&card, &lib, Some("Other"));
    let date = date_command(T0);
    assert_eq!(copied_to(&other), [lib.join(format!("{date} - Other/JPEG/DSCF0001.JPG"))]);
    assert!(other.failed.is_empty());
}

#[test]
fn inv2_existing_files_survive_and_only_stale_temps_are_deleted() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"new raw"),
        ("DCIM/100_FUJI/DSCF0002.JPG", b"new jpg"),
        ("DCIM/100_FUJI/DSCF0009.JPG", b"same name, new"),
    ]);
    let date = date_command(T0);
    let session = format!("{date} - Tirana");
    put(&lib, &[
        (&format!("{session}/JPEG/DSCF0009.JPG"), b"same name, old"),
        (&format!("{session}/RAW/DSCF0100.RAF"), b"earlier sync"),
        ("2025-01-01/RAW/DSCF0001.RAF", b"other raw, same name"),
        ("2025-01-01/notes.txt", b"user notes"),
        ("2025-01-01/.DS_Store", b"finder"),
        (".Trashes/501/.photosync-tmp-1-0-DSCF0007.RAF", b"in trash, not walked"),
        ("2025-01-01/.hidden/.photosync-tmp-1-0-DSCF0008.RAF", b"in a dot folder"),
        ("2025-01-01/.photosync-tmp-dir/DSCF0006.RAF", b"inside a folder named like a temp"),
    ]);
    let stale = [
        write_file(&lib, &format!("{session}/RAW/.photosync-tmp-77-3-DSCF0001.RAF"), b"part", at(T0)),
        write_file(&lib, "2025-01-01/JPEG/.photosync-tmp-12-0-DSCF0002.JPG", b"par", at(T0)),
    ];
    let before = manifest(&lib);

    let (_, outcome) = sync(&card, &lib, Some("Tirana"));

    let mut want_removed = stale.to_vec();
    want_removed.sort();
    assert_eq!(outcome.temps.removed, want_removed);
    assert!(outcome.temps.failures.is_empty());
    assert_eq!(outcome.copied.len(), 2);
    assert_eq!(outcome.failed.len(), 1, "DSCF0009 collides");
    let after = manifest(&lib);
    for (path, (modified, bytes)) in &before {
        if stale.contains(path) {
            assert!(!after.contains_key(path), "{} kept", path.display());
        } else if bytes.is_some() {
            assert_eq!(after.get(path), Some(&(*modified, bytes.clone())), "{}", path.display());
        } else {
            assert!(after.contains_key(path), "{} gone", path.display());
        }
    }
    assert!(temps_under(&lib).iter().all(|t| before.contains_key(t)), "no new temp left");
}

#[test]
fn inv1_source_manifest_is_unchanged_by_a_sync() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"raw"),
        ("DCIM/100_FUJI/DSCF0001.JPG", b"jpg"),
        ("DCIM/100_FUJI/DSCF0002.JPG", b"synced"),
        ("DCIM/100_FUJI/NOTES.TXT", b"ignored"),
        ("DCIM/100_FUJI/._DSCF0001.RAF", b"apple double"),
    ]);
    put(&lib, &[("2025-01-01/JPEG/DSCF0002.JPG", b"synced")]);
    let before = manifest(&card);

    let (_, outcome) = sync(&card, &lib, Some("Tirana"));

    assert_eq!(outcome.copied.len(), 2);
    assert_eq!(manifest(&card), before);
}

#[cfg(unix)]
#[test]
fn read_error_on_a_source_file_fails_that_file_only() {
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", b"locked, unique"),
        ("DCIM/100_FUJI/DSCF0002.RAF", b"locked, has a candidate"),
        ("DCIM/100_FUJI/DSCF0003.RAF", b"fine"),
    ]);
    put(&lib, &[("2025-01-01/RAW/DSCF0002.RAF", b"other content, same sz!")]);
    let locked = card.join("DCIM/100_FUJI/DSCF0001.RAF");
    let hashed = card.join("DCIM/100_FUJI/DSCF0002.RAF");
    let _a = Mode::set(&locked, 0o000);
    let _b = Mode::set(&hashed, 0o000);

    let (plan, outcome) = sync(&card, &lib, None);

    // Read while hashing for the plan: fails there, with its path (S003).
    assert_eq!(plan.failures.len(), 1);
    assert_eq!(plan.failures[0].path, hashed);
    // Read while copying: fails here, with its path and error.
    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert_eq!(failed.file.path, locked);
    assert!(matches!(&failed.reason, FailReason::Read(e) if e.kind() == std::io::ErrorKind::PermissionDenied));
    assert!(failed.to_string().contains("DSCF0001.RAF"));
    let session = lib.join(date_command(T0));
    assert_eq!(copied_to(&outcome), [session.join("RAW/DSCF0003.RAF")]);
    assert!(!session.join("RAW/DSCF0001.RAF").exists());
    assert!(temps_under(&lib).is_empty());
}

#[test]
fn folder_from_another_library_is_refused() {
    let dir = TempDir::new();
    let (card, lib, other) = (dir.0.join("card"), dir.0.join("lib"), dir.0.join("other"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw")]);
    fs::create_dir_all(&lib).unwrap();
    fs::create_dir_all(&other).unwrap();
    let for_other = folder(&plan(&card, &other), None);

    let err = copy_plan(&plan(&card, &lib), &for_other, &mut Monitor::default()).unwrap_err();

    assert!(matches!(err, CopyError::LibraryMismatch { .. }));
    assert_eq!(manifest(&other), Default::default());
}

#[test]
fn stale_temps_are_removed_even_with_nothing_to_copy() {
    let dir = TempDir::new();
    let lib = dir.0.join("lib");
    let stale = write_file(&lib, "2026-01-01/RAW/.photosync-tmp-5-0-DSCF0001.RAF", b"x", at(T0));
    let keep = write_file(&lib, "2026-01-01/RAW/DSCF0001.RAF", b"x", at(T0));

    let cleanup = remove_stale_temps(&lib);

    assert_eq!(cleanup.removed, std::slice::from_ref(&stale));
    assert!(!stale.exists() && keep.exists());
}

/// A session or `<TYPE>` folder that is a symlink would send copies out of
/// the library (INV-9), maybe onto the card (INV-10): refused, never followed.
#[cfg(unix)]
#[test]
fn symlinked_session_or_type_folder_is_refused_never_followed() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new();
    let (card, lib, outside) = (dir.0.join("card"), dir.0.join("lib"), dir.0.join("outside"));
    put(&card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"raw"), ("DCIM/100_FUJI/DSCF0001.JPG", b"jpg")]);
    fs::create_dir_all(&lib).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let date = date_command(T0);

    // The session folder is a symlink: every file fails.
    symlink(&outside, lib.join(&date)).unwrap();
    let (_, outcome) = sync(&card, &lib, None);
    assert!(outcome.copied.is_empty(), "{:?}", outcome.copied);
    assert_eq!(outcome.failed.len(), 2);
    for failed in &outcome.failed {
        assert!(
            matches!(&failed.reason, FailReason::Write { path, error }
                if *path == lib.join(&date) && error.kind() == std::io::ErrorKind::NotADirectory),
            "{failed}"
        );
    }
    assert_eq!(manifest(&outside), Default::default());
    assert!(fs::symlink_metadata(lib.join(&date)).unwrap().file_type().is_symlink());
    fs::remove_file(lib.join(&date)).unwrap();

    // The RAW folder is a symlink: only the RAW file fails.
    fs::create_dir_all(lib.join(&date)).unwrap();
    symlink(&outside, lib.join(&date).join("RAW")).unwrap();
    let (_, outcome) = sync(&card, &lib, None);
    assert_eq!(copied_to(&outcome), [lib.join(&date).join("JPEG/DSCF0001.JPG")]);
    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert_eq!(failed.file.path, card.join("DCIM/100_FUJI/DSCF0001.RAF"));
    assert!(
        matches!(&failed.reason, FailReason::Write { path, error }
            if *path == lib.join(&date).join("RAW") && error.kind() == std::io::ErrorKind::NotADirectory),
        "{failed}"
    );
    assert_eq!(manifest(&outside), Default::default());
}

/// One file changes length after the scan, a healthy one stays. The changed
/// one fails before the rename, and the report counts only the healthy one.
fn file_resized_after_the_scan_fails(new_len: u64) {
    const SCANNED: u64 = 3000;
    let dir = TempDir::new();
    let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
    let resized = card.join("DCIM/100_FUJI/DSCF0001.RAF");
    put(&card, &[
        ("DCIM/100_FUJI/DSCF0001.RAF", &[7; SCANNED as usize]),
        ("DCIM/100_FUJI/DSCF0002.RAF", b"healthy"),
    ]);
    fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan_elsewhere(&card)];
    fs::File::options().write(true).open(&resized).unwrap().set_len(new_len).unwrap();
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    assert!(!plan.to_copy[0].is_hashed(), "this path has no planning hash to catch it");

    let outcome = copy_plan(&plan, &folder(&plan, None), &mut Monitor::default()).unwrap();

    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert_eq!(failed.file.path, resized);
    assert!(
        matches!(failed.reason, FailReason::SizeChanged { scanned: SCANNED, read } if read == new_len),
        "{:?}",
        failed.reason
    );
    let text = failed.reason.to_string();
    assert!(text.contains(&new_len.to_string()) && text.contains("3000"), "{text}");
    assert!(temps_under(&lib).is_empty());
    let on_drive: Vec<PathBuf> = manifest(&lib)
        .into_iter()
        .filter(|(_, (_, bytes))| bytes.is_some())
        .map(|(path, _)| path)
        .collect();
    assert_eq!(on_drive, copied_to(&outcome), "only the healthy file is on the drive");

    let report = Report::new(scans, plan, outcome).unwrap();
    assert!(!report.is_complete());
    let totals = report.totals();
    assert_eq!((totals.copied.files, totals.copied.bytes), (1, b"healthy".len() as u64));
    assert_eq!((totals.failed.files, totals.failed.bytes), (1, SCANNED));
}

/// 2026-10-30 audit F01: a short read gives no error, and the read-back
/// agrees with it, so nothing else stops it.
#[test]
fn a_file_that_reads_shorter_than_scanned_fails() {
    file_resized_after_the_scan_fails(1024);
}

#[test]
fn a_file_that_reads_empty_fails() {
    file_resized_after_the_scan_fails(0);
}

/// Bytes the scan did not see: the plan compared the file by its old size.
#[test]
fn a_file_that_reads_longer_than_scanned_fails() {
    file_resized_after_the_scan_fails(4000);
}

/// Shrunk before planning, so planning hashes the short bytes and the copy
/// reads as that hash. Only the size shows it, and its duplicate fails too.
#[test]
fn a_short_read_that_matches_the_planning_hash_still_fails() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    for card in [&a, &b] {
        put(card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"backup raw")]);
    }
    fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan_elsewhere(&a), scan_elsewhere(&b)];
    for card in [&a, &b] {
        let file = card.join("DCIM/100_FUJI/DSCF0001.RAF");
        fs::File::options().write(true).open(file).unwrap().set_len(4).unwrap();
    }
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    assert!(plan.to_copy[0].is_hashed());
    assert_eq!(plan.duplicates.len(), 1);

    let outcome = copy_plan(&plan, &folder(&plan, None), &mut Monitor::default()).unwrap();

    assert!(outcome.copied.is_empty(), "{:?}", outcome.copied);
    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert!(matches!(failed.reason, FailReason::SizeChanged { scanned: 10, read: 4 }), "{:?}", failed.reason);
    assert!(manifest(&lib).values().all(|(_, bytes)| bytes.is_none()), "a file was placed");
    let report = Report::new(scans, plan, outcome).unwrap();
    let twin = &report.sources()[1].files[0].outcome;
    assert!(matches!(twin, FileOutcome::DuplicateOfFailed { .. }), "{twin:?}");
    assert!(!report.is_complete());
}

/// DEC-042: a wrong size is not a read error, so it does not fall back to
/// the twin, as a file that read otherwise does not (DEC-036).
#[test]
fn a_short_read_after_planning_does_not_fall_back_to_the_twin() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    for card in [&a, &b] {
        put(card, &[("DCIM/100_FUJI/DSCF0001.RAF", b"backup raw")]);
    }
    fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan_card(&a, 1), scan_card(&b, 2)];
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    assert_eq!(plan.to_copy[0].twins().len(), 1);
    let file = a.join("DCIM/100_FUJI/DSCF0001.RAF");
    fs::File::options().write(true).open(file).unwrap().set_len(4).unwrap();

    let outcome = copy_plan(&plan, &folder(&plan, None), &mut Monitor::default()).unwrap();

    assert!(outcome.fallbacks.is_empty(), "{:?}", outcome.fallbacks);
    assert!(outcome.copied.is_empty(), "{:?}", outcome.copied);
    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert!(matches!(failed.reason, FailReason::SizeChanged { scanned: 10, read: 4 }), "{:?}", failed.reason);
    assert!(manifest(&lib).values().all(|(_, bytes)| bytes.is_none()), "a file was placed");
    let report = Report::new(scans, plan, outcome).unwrap();
    assert!(!report.is_complete());
}

/// S014 fast check: the copy is checked against the hash planning took. A
/// card file that changed since (same size) fails, so its twin on the
/// other card, planned as its duplicate, is not reported safe.
#[test]
fn a_file_changed_between_planning_and_copy_fails() {
    let dir = TempDir::new();
    let (a, b, lib) = (dir.0.join("a"), dir.0.join("b"), dir.0.join("lib"));
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"backup raw")]);
    put(&b, &[("DCIM/100_FUJI/DSCF0001.RAF", b"backup raw")]);
    fs::create_dir_all(&lib).unwrap();
    let scans = vec![scan_elsewhere(&a), scan_elsewhere(&b)];
    let plan = plan_sync(&lib, &scans, &mut Monitor::default()).unwrap();
    assert_eq!(plan.duplicates.len(), 1);
    assert!(plan.to_copy[0].is_hashed());
    put(&a, &[("DCIM/100_FUJI/DSCF0001.RAF", b"change raw")]);

    let outcome = copy_plan(&plan, &folder(&plan, None), &mut Monitor::default()).unwrap();

    assert!(outcome.copied.is_empty(), "{:?}", outcome.copied);
    let [failed] = &outcome.failed[..] else { panic!("{:?}", outcome.failed) };
    assert!(matches!(failed.reason, FailReason::SourceChanged), "{:?}", failed.reason);
    assert!(temps_under(&lib).is_empty());
    assert!(manifest(&lib).values().all(|(_, bytes)| bytes.is_none()), "a file was placed");
    let report = Report::new(scans, plan, outcome).unwrap();
    let twin = &report.sources()[1].files[0].outcome;
    assert!(matches!(twin, FileOutcome::DuplicateOfFailed { .. }), "{twin:?}");
    assert!(!report.is_complete());
}
