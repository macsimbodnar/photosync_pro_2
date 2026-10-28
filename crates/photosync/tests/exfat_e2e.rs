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
