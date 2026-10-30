//! The binary end to end on real mounts. `scripts/test_exfat.sh` mounts
//! three exFAT images side by side: the temp dir (TMPDIR), a stand-in card
//! with `DCIM` at its top (`PHOTOSYNC_TEST_CARD`) and a library drive
//! (`PHOTOSYNC_TEST_LIBRARY`). Three devices, so strict INV-10 lets a sync
//! through, as with a real card and drive. Without the variables, nothing
//! is checked and each test says so.

#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use common::{TempDir, at, date_command, manifest, photosync, write_file};

const TAKEN: u64 = 1_700_000_000;

/// The card and library mounts, resolved, or `None` outside the script.
fn mounts(test: &str) -> Option<(PathBuf, PathBuf)> {
    let (Some(card), Some(library)) = (
        std::env::var_os("PHOTOSYNC_TEST_CARD"),
        std::env::var_os("PHOTOSYNC_TEST_LIBRARY"),
    ) else {
        eprintln!(
            "SKIPPED {test}: PHOTOSYNC_TEST_CARD or PHOTOSYNC_TEST_LIBRARY is not set \
                   (scripts/test_exfat.sh sets them)"
        );
        return None;
    };
    let card = fs::canonicalize(card).unwrap();
    let library = fs::canonicalize(library).unwrap();
    assert!(
        card.join("DCIM").is_dir(),
        "{} has no DCIM folder",
        card.display()
    );
    // The script mounts both in one folder, which stands in for /Volumes.
    assert_eq!(
        card.parent(),
        library.parent(),
        "card and library are not side by side"
    );
    Some((card, library))
}

fn device(path: &Path) -> u64 {
    fs::metadata(path).unwrap().dev()
}

fn sync(sources: &[&Path], library: &Path, name: &str) -> common::Run {
    let mut args = vec!["sync".into()];
    for s in sources {
        args.extend(["--source".into(), s.as_os_str().to_owned()]);
    }
    args.extend([
        "--dest".into(),
        library.as_os_str().to_owned(),
        "--name".into(),
        name.into(),
    ]);
    photosync::<_, std::ffi::OsString>(args)
}

#[test]
fn sources_and_drives_list_the_mounted_card_and_drives() {
    let Some((card, library)) = mounts("sources_and_drives_list_the_mounted_card_and_drives")
    else {
        return;
    };
    let root = card.parent().unwrap();

    let sources = photosync([
        "sources".as_ref(),
        "--search-root".as_ref(),
        root.as_os_str(),
    ]);
    let drives = photosync([
        "drives".as_ref(),
        "--search-root".as_ref(),
        root.as_os_str(),
    ]);

    assert_eq!(sources.code, 0, "{sources:?}");
    assert_eq!(sources.stdout, format!("{}\n", card.display()));
    assert_eq!(drives.code, 0, "{drives:?}");
    let listed: Vec<&str> = drives.stdout.lines().collect();
    assert!(listed.contains(&library.to_str().unwrap()), "{drives:?}");
    assert!(
        !listed.contains(&card.to_str().unwrap()),
        "the card is offered as a drive: {drives:?}"
    );
}

/// A full sync from a mounted card, a re-sync that copies nothing, a second
/// source in the same session folder, a failure, and a refused card
/// library: exit codes, files on the drive, and the report text.
#[test]
fn sync_from_a_mounted_card_to_a_drive_end_to_end() {
    let Some((card, drive)) = mounts("sync_from_a_mounted_card_to_a_drive_end_to_end") else {
        return;
    };
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw one", at(TAKEN));
    write_file(&card, "DCIM/100_FUJI/DSCF0001.JPG", b"jpeg one", at(TAKEN));
    write_file(
        &card,
        "DCIM/100_FUJI/DSCF0002.MOV",
        b"video two",
        at(TAKEN + 60),
    );
    write_file(&card, "DCIM/100_FUJI/notes.txt", b"not a photo", at(TAKEN));
    let card_before = manifest(&card);
    let library = drive.join("Photos");
    fs::create_dir(&library).unwrap();

    // First sync: everything is copied into one named folder.
    let first = sync(&[&card], &library, "Tirana");
    assert_eq!(first.code, 0, "{first:?}");
    let folder = library.join(format!("{} - Tirana", date_command(TAKEN)));
    for (rel, bytes) in [
        ("RAW/DSCF0001.RAF", &b"raw one"[..]),
        ("JPEG/DSCF0001.JPG", b"jpeg one"),
        ("VIDEO/DSCF0002.MOV", b"video two"),
    ] {
        assert_eq!(fs::read(folder.join(rel)).unwrap(), bytes, "{rel}");
    }
    assert_eq!(
        fs::read_dir(&library).unwrap().count(),
        1,
        "one session folder"
    );
    assert!(!folder.join("notes.txt").exists() && !folder.join("RAW/notes.txt").exists());
    // The scan line names the card's own device, not the drive's.
    assert_ne!(device(&card), device(&drive));
    assert!(
        first.stderr.contains(&format!(
            "scanned {}: 3 photos or videos (24 B), 1 ignored, 0 unreadable, device {}",
            card.display(),
            device(&card)
        )),
        "{first:?}"
    );
    assert!(first.stderr.contains("copying 0/3 files"), "{first:?}");
    assert!(
        first
            .stderr
            .contains("copying 3/3 files, 24 B of 24 B (100%)"),
        "{first:?}"
    );
    let table = [
        format!("Source {}", card.display()),
        "           copied   skipped   failed   cancelled".to_string(),
        "  RAW           1         0        0           0".to_string(),
        "  JPEG          1         0        0           0".to_string(),
        "  VIDEO         1         0        0           0".to_string(),
        "  all           3         0        0           0".to_string(),
        "  size       24 B       0 B      0 B         0 B".to_string(),
        "  ignored 1 file (.txt 1)".to_string(),
    ];
    assert!(first.stdout.contains(&table.join("\n")), "{first:?}");
    // S010: the report says where the photos went.
    assert!(
        first.stdout.starts_with(&format!(
            "Report\nLibrary {}\nFolder  {}\n",
            library.display(),
            folder.display()
        )),
        "{first:?}"
    );
    assert!(
        first.stdout.ends_with(
            "\nResult: all 3 photos or videos on the drive (3 copied, 0 already there).\n"
        ),
        "{first:?}"
    );
    assert_eq!(manifest(&card), card_before, "INV-1: the card changed");

    // Re-sync: nothing copied, nothing on the drive touched (INV-4, INV-2).
    let drive_before = manifest(&library);
    let again = sync(&[&card], &library, "Tirana");
    assert_eq!(again.code, 0, "{again:?}");
    assert!(
        again
            .stdout
            .contains("  all           0         3        0           0"),
        "{again:?}"
    );
    assert!(
        again.stdout.ends_with(
            "Result: all 3 photos or videos on the drive (0 copied, 3 already there).\n"
        ),
        "{again:?}"
    );
    assert_eq!(manifest(&library), drive_before);

    // A second source, a plain folder on the temp image: copied in sequence
    // with the card, into the reused named folder.
    let dir = TempDir::new();
    let second = dir.0.join("card2");
    write_file(
        &second,
        "DCIM/101_FUJI/DSCF0003.RAF",
        b"raw three",
        at(TAKEN + 120),
    );
    write_file(&second, "DCIM/101_FUJI/DSCF0001.RAF", b"raw one", at(TAKEN));
    let both = sync(&[&card, &second], &library, "Tirana");
    assert_eq!(both.code, 0, "{both:?}");
    assert_eq!(
        fs::read(folder.join("RAW/DSCF0003.RAF")).unwrap(),
        b"raw three"
    );
    assert!(
        both.stdout
            .contains(&format!("Source {}", second.display())),
        "{both:?}"
    );
    assert!(both.stdout.contains("\nAll sources\n"), "{both:?}");
    assert!(
        both.stdout.ends_with(
            "Result: all 5 photos or videos on the drive (1 copied, 4 already there).\n"
        ),
        "{both:?}"
    );

    // Another camera's DSCF0002.MOV: the name is taken, so it fails, the
    // drive's file stays, and the exit code says the card is not safe.
    let third = dir.0.join("card3");
    write_file(
        &third,
        "DCIM/100_FUJI/DSCF0002.MOV",
        b"another video",
        at(TAKEN),
    );
    let clash = sync(&[&third], &library, "Tirana");
    assert_eq!(clash.code, 1, "{clash:?}");
    let mov = folder.join("VIDEO/DSCF0002.MOV");
    assert!(
        clash.stdout.contains(&format!(
            "  failed      {}: name collision with {}",
            third.join("DCIM/100_FUJI/DSCF0002.MOV").display(),
            mov.display()
        )),
        "{clash:?}"
    );
    assert!(
        clash
            .stdout
            .contains("Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable."),
        "{clash:?}"
    );
    assert!(
        clash.stdout.contains("Do not format the card."),
        "{clash:?}"
    );
    assert_eq!(fs::read(&mov).unwrap(), b"video two");

    // A library on the card: refused with the card's sentence, no report.
    let on_card = card.join("Library");
    fs::create_dir(&on_card).unwrap();
    let refused = sync(&[&second], &on_card, "Tirana");
    assert_eq!(refused.code, 2, "{refused:?}");
    assert!(
        refused.last_err_line().starts_with(&format!(
            "The library {} is on {}, which is a camera card (it has a DCIM folder).",
            on_card.display(),
            card.display()
        )),
        "{refused:?}"
    );
    assert!(refused.stdout.is_empty(), "{refused:?}");
    assert_eq!(fs::read_dir(&on_card).unwrap().count(), 0);
}

/// Runs the binary with stdout, and stderr too if `close_stderr`, on a
/// pipe whose reader is already gone, as `photosync ... | head` ends up.
/// Rust ignores SIGPIPE, so each write fails with EPIPE; `print!` would
/// panic on that and exit 101.
fn closed_run(args: &[&std::ffi::OsStr], close_stderr: bool) -> (i32, String) {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_photosync"));
    cmd.args(args).stdout(writer.try_clone().unwrap());
    if close_stderr {
        cmd.stderr(writer);
    } else {
        cmd.stderr(std::process::Stdio::piped());
    }
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// S008 fast check: a closed stdout never panics. The sync still finishes,
/// its exit code still says whether the card is safe, and a list that
/// could not be written is not a success.
#[test]
fn closed_stdout_never_panics_and_keeps_the_exit_code() {
    let Some((card, drive)) = mounts("closed_stdout_never_panics_and_keeps_the_exit_code") else {
        return;
    };
    // A plain folder on the temp image, not the card the other test counts.
    let dir = TempDir::new();
    let source = dir.0.join("card");
    write_file(
        &source,
        "DCIM/100_FUJI/DSCF0101.RAF",
        b"closed raw",
        at(TAKEN),
    );
    let library = drive.join("Closed");
    fs::create_dir(&library).unwrap();
    let args = [
        "sync".as_ref(),
        "--source".as_ref(),
        source.as_os_str(),
        "--dest".as_ref(),
        library.as_os_str(),
    ];

    let (code, stderr) = closed_run(&args, false);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stderr.contains("Cannot write the report to stdout: "),
        "{stderr}"
    );
    let copied = library.join(date_command(TAKEN)).join("RAW/DSCF0101.RAF");
    assert_eq!(fs::read(&copied).unwrap(), b"closed raw");

    // Nothing to write to at all: the re-sync still runs and exits 0.
    let (code, stderr) = closed_run(&args, true);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(fs::read(&copied).unwrap(), b"closed raw");

    let root = card.parent().unwrap();
    let (code, stderr) = closed_run(
        &[
            "sources".as_ref(),
            "--search-root".as_ref(),
            root.as_os_str(),
        ],
        false,
    );
    assert_eq!(code, 1, "{stderr}");
    assert!(
        stderr.contains("Cannot write the list to stdout: "),
        "{stderr}"
    );
}

/// S019: two cards on two devices, as in two card readers, copied at once
/// (one thread per card). Each photo lands once with its bytes, one on
/// both cards is copied once, the progress line counts each card, and the
/// cards are left as they were (INV-1). Then a different photo of a
/// taken name, on the other card, is a name collision: the drive's file
/// stays (INV-2).
#[test]
fn two_cards_on_two_devices_copy_at_once() {
    let test = "two_cards_on_two_devices_copy_at_once";
    let (Some(drive), Some((twin_a, twin_b))) =
        (common::test_drive(test), common::twin_cards(test))
    else {
        return;
    };
    let (a, b) = (
        common::OnDrive::new(&twin_a, "s019-a"),
        common::OnDrive::new(&twin_b, "s019-b"),
    );
    let library = common::OnDrive::new(&drive, "s019-lib");
    let raw_a: Vec<u8> = (0..3 << 20).map(|i| (i % 251) as u8).collect();
    let raw_b: Vec<u8> = (0..3 << 20).map(|i| (i % 241) as u8).collect();
    write_file(&a.0, "DCIM/100_FUJI/DSCF0001.RAF", &raw_a, at(TAKEN));
    write_file(&a.0, "DCIM/100_FUJI/DSCF0002.JPG", b"on both", at(TAKEN));
    write_file(&b.0, "DCIM/100_FUJI/DSCF0101.RAF", &raw_b, at(TAKEN));
    write_file(&b.0, "DCIM/100_FUJI/DSCF0002.JPG", b"on both", at(TAKEN));
    let cards = (manifest(&a.0), manifest(&b.0));

    let run = sync(&[&a.0, &b.0], &library.0, "Two");
    assert_eq!(run.code, 0, "{run:?}");
    let folder = library.0.join(format!("{} - Two", date_command(TAKEN)));
    let want = [
        (folder.join("JPEG/DSCF0002.JPG"), b"on both".to_vec()),
        (folder.join("RAW/DSCF0001.RAF"), raw_a),
        (folder.join("RAW/DSCF0101.RAF"), raw_b),
    ];
    let on_drive: Vec<(PathBuf, Vec<u8>)> = manifest(&library.0)
        .into_iter()
        .filter_map(|(p, (_, bytes))| bytes.map(|b| (p, b)))
        .collect();
    assert_eq!(on_drive, want, "each photo once");
    let name = |d: &common::OnDrive| d.0.file_name().unwrap().to_string_lossy().into_owned();
    let line = format!(
        "copying 3/3 files, 6.3 MB of 6.3 MB (100%), ETA 0s [{} 2/2, {} 1/1]",
        name(&a),
        name(&b)
    );
    assert!(run.stderr.contains(&line), "{line}\n{run:?}");
    assert!(
        run.stdout.contains("(3 copied, 1 already there)."),
        "{run:?}"
    );
    assert_eq!((manifest(&a.0), manifest(&b.0)), cards, "INV-1");

    write_file(
        &b.0,
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"another camera",
        at(TAKEN),
    );
    write_file(
        &a.0,
        "DCIM/100_FUJI/DSCF0003.RAF",
        b"a raw three",
        at(TAKEN),
    );
    let clash = sync(&[&a.0, &b.0], &library.0, "Two");
    assert_eq!(clash.code, 1, "{clash:?}");
    assert!(
        clash.stdout.contains(&format!(
            "  failed      {}: name collision with {}",
            b.0.join("DCIM/100_FUJI/DSCF0001.RAF").display(),
            folder.join("RAW/DSCF0001.RAF").display()
        )),
        "{clash:?}"
    );
    assert_eq!(
        fs::read(folder.join("RAW/DSCF0001.RAF")).unwrap(),
        want[1].1
    );
    assert_eq!(
        fs::read(folder.join("RAW/DSCF0003.RAF")).unwrap(),
        b"a raw three"
    );
}

/// F03, specs, Sources: with a backup of a card's RAF on a second card, and
/// a JPEG twice on the first, the report shows "duplicate in session" apart
/// from "already on drive" for each card and names the card on the drive.
/// A re-run of the first card alone: what is on the drive is "already on
/// drive", and only the second of the JPEGs stays a duplicate, of the first.
#[test]
fn the_report_shows_duplicates_and_names_the_card() {
    let test = "the_report_shows_duplicates_and_names_the_card";
    let (Some(drive), Some((twin_a, twin_b))) =
        (common::test_drive(test), common::twin_cards(test))
    else {
        return;
    };
    let (a, b) = (
        common::OnDrive::new(&twin_a, "s028-a"),
        common::OnDrive::new(&twin_b, "s028-b"),
    );
    let library = common::OnDrive::new(&drive, "s028-lib");
    write_file(&a.0, "DCIM/100_FUJI/DSCF0001.RAF", b"backed up", at(TAKEN));
    write_file(&a.0, "DCIM/100_FUJI/DSCF0002.JPG", b"twice", at(TAKEN));
    write_file(&a.0, "DCIM/101_FUJI/DSCF0002.JPG", b"twice", at(TAKEN));
    write_file(&b.0, "DCIM/100_FUJI/DSCF0001.RAF", b"backed up", at(TAKEN));

    let run = sync(&[&a.0, &b.0], &library.0, "Dup");
    assert_eq!(run.code, 0, "{run:?}");
    let want = [
        "  of the skipped, 0 already on drive, 1 duplicate in session".to_string(),
        format!(
            "  duplicate in session  101_FUJI/DSCF0002.JPG, on drive from {}",
            a.0.display()
        ),
        format!(
            "  duplicate in session  100_FUJI/DSCF0001.RAF, on drive from {}",
            a.0.display()
        ),
    ];
    for part in &want {
        assert!(run.stdout.contains(part.as_str()), "{part}\n{run:?}");
    }
    // One count line for each card, and each file in its own card's section.
    assert_eq!(run.stdout.matches(want[0].as_str()).count(), 2, "{run:?}");
    let of_b = run
        .stdout
        .find(&format!("Source {}", b.0.display()))
        .expect("no section for the second card");
    assert!(run.stdout[..of_b].contains(&want[1]), "{run:?}");
    assert!(run.stdout[of_b..].contains(&want[2]), "{run:?}");

    let again = sync(&[&a.0], &library.0, "Dup");
    assert_eq!(again.code, 0, "{again:?}");
    for part in [
        "  of the skipped, 2 already on drive, 1 duplicate in session",
        want[1].as_str(),
    ] {
        assert!(again.stdout.contains(part), "{part}\n{again:?}");
    }
    assert!(!again.stdout.contains("100_FUJI/DSCF0001.RAF"), "{again:?}");
}

/// S040: on a plain terminal the problems can sit hundreds of lines up. The
/// last lines, what `| tail` shows, say how many there are.
#[test]
fn the_last_lines_point_back_at_problems_above_the_duplicates() {
    let test = "the_last_lines_point_back_at_problems_above_the_duplicates";
    let Some(_drive) = common::test_drive(test) else {
        return;
    };
    let dir = TempDir::new();
    let (a, b) = (dir.0.join("a"), dir.0.join("b"));
    let library = common::OnDrive::new(&common::disk_of_the_binary(), "s040-cli");
    write_duplicates(&a);
    write_file(&a, "DCIM/100_FUJI/DSCF8000.RAF", b"from a", at(TAKEN));
    write_file(&b, "DCIM/100_FUJI/DSCF8000.RAF", b"from b", at(TAKEN));
    write_file(&a, "DCIM/100_FUJI/DSCF9000.RAF", b"new", at(TAKEN));
    common::lock_in_library(&library.0, "old/DSCF9000.RAF", b"old");

    let run = sync(&[&a, &b], &library.0, "Back");

    assert_eq!(run.code, 1, "{run:?}");
    // One name collision and one library entry.
    let last: Vec<&str> = run.stdout.lines().rev().take(3).collect();
    assert_eq!(
        last[2], "2 problem lines above the duplicates: read them.",
        "{run:?}"
    );
    assert!(
        last[0].starts_with("Result: NOT COMPLETE: 1 failed"),
        "{run:?}"
    );
}

/// S040: a library problem alone leaves "all on the drive" and exit 0, so
/// the line above the result is the only sign of it at the end.
#[test]
fn a_library_problem_alone_is_in_the_last_lines() {
    let test = "a_library_problem_alone_is_in_the_last_lines";
    let Some(_drive) = common::test_drive(test) else {
        return;
    };
    let dir = TempDir::new();
    let card = dir.0.join("card");
    let library = common::OnDrive::new(&common::disk_of_the_binary(), "s040-lib");
    write_duplicates(&card);
    write_file(&card, "DCIM/100_FUJI/DSCF9000.RAF", b"new", at(TAKEN));
    common::lock_in_library(&library.0, "old/DSCF9000.RAF", b"old");

    let run = sync(&[&card], &library.0, "Back");

    assert_eq!(run.code, 0, "{run:?}");
    let last: Vec<&str> = run.stdout.lines().rev().take(3).collect();
    assert_eq!(
        last[2], "1 problem line above the duplicates: read it.",
        "{run:?}"
    );
    assert!(last[0].starts_with("Result: all "), "{run:?}");
}

/// S040: with no problem the report does not cry wolf.
#[test]
fn a_report_of_duplicates_with_no_problem_points_at_nothing() {
    let test = "a_report_of_duplicates_with_no_problem_points_at_nothing";
    let Some(_drive) = common::test_drive(test) else {
        return;
    };
    let dir = TempDir::new();
    let card = dir.0.join("card");
    let library = common::OnDrive::new(&common::disk_of_the_binary(), "s040-clean");
    write_duplicates(&card);

    let run = sync(&[&card], &library.0, "Back");

    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("  duplicate in session  "), "{run:?}");
    assert!(!run.stdout.contains("problem line"), "{run:?}");
}

/// A backup card's kind of list: 30 photos, each on the card twice.
fn write_duplicates(card: &Path) {
    for n in 0..30 {
        let bytes = format!("photo {n:04}");
        for folder in ["100_FUJI", "101_FUJI"] {
            write_file(
                card,
                &format!("DCIM/{folder}/DSCF{n:04}.RAF"),
                bytes.as_bytes(),
                at(TAKEN),
            );
        }
    }
}

/// S039, the CLI: the first card has many duplicates, the second a name
/// collision, and the library a file that cannot be read. All that comes
/// before the first duplicate line, however long the list. The library is
/// off the images, as chmod does nothing on exFAT.
#[test]
fn duplicates_never_bury_a_failure_or_a_library_problem_in_the_cli() {
    let test = "duplicates_never_bury_a_failure_or_a_library_problem_in_the_cli";
    let Some(_drive) = common::test_drive(test) else {
        return;
    };
    let dir = TempDir::new();
    let (a, b) = (dir.0.join("a"), dir.0.join("b"));
    let library = common::OnDrive::new(&common::disk_of_the_binary(), "s039-cli");
    for n in 0..30 {
        let bytes = format!("photo {n:04}");
        for folder in ["100_FUJI", "101_FUJI"] {
            write_file(
                &a,
                &format!("DCIM/{folder}/DSCF{n:04}.RAF"),
                bytes.as_bytes(),
                at(TAKEN),
            );
        }
    }
    write_file(&a, "DCIM/100_FUJI/DSCF8000.RAF", b"from a", at(TAKEN));
    write_file(&b, "DCIM/100_FUJI/DSCF8000.RAF", b"from b", at(TAKEN));
    write_file(&b, "DCIM/100_FUJI/DSCF0000.RAF", b"photo 0000", at(TAKEN));
    write_file(&a, "DCIM/100_FUJI/DSCF9000.RAF", b"new", at(TAKEN));
    let locked = common::lock_in_library(&library.0, "old/DSCF9000.RAF", b"old");

    let run = sync(&[&a, &b], &library.0, "Bury");
    assert_eq!(run.code, 1, "{run:?}");

    let lines: Vec<&str> = run.stdout.lines().collect();
    let is_duplicate = |l: &&&str| l.starts_with("  duplicate in session  ");
    let first_duplicate = lines
        .iter()
        .position(|l| is_duplicate(&l))
        .expect("no duplicate");
    assert_eq!(lines.iter().filter(is_duplicate).count(), 31, "{run:?}");
    for part in [
        format!(
            "  failed      {}",
            b.join("DCIM/100_FUJI/DSCF8000.RAF").display()
        ),
        "Library entries that could not be read".to_string(),
        format!("  {}: ", locked.display()),
    ] {
        let at = lines.iter().position(|l| l.starts_with(&part));
        assert!(at.is_some_and(|i| i < first_duplicate), "{part}\n{run:?}");
    }
    // Each list under its card's name, with the card it is on the drive from.
    let of_a = format!("Duplicates in session on {}", a.display());
    let of_b = format!("Duplicates in session on {}", b.display());
    let at = |heading: &str| lines.iter().position(|l| *l == heading).expect(heading);
    assert_eq!(first_duplicate, at(&of_a) + 1, "{run:?}");
    assert_eq!(
        lines[at(&of_b) + 1],
        format!(
            "  duplicate in session  100_FUJI/DSCF0000.RAF, on drive from {}",
            a.display()
        ),
        "{run:?}"
    );
    assert!(
        lines
            .last()
            .unwrap()
            .starts_with("Result: NOT COMPLETE: 1 failed"),
        "{run:?}"
    );
}
