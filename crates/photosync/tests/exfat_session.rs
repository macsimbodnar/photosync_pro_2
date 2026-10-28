//! S020: `photosync sync --one-by-one` end to end on the exFAT images of
//! `scripts/test_exfat.sh`: one card after another into one session, with
//! real copies, so the session's summary and exit code can be checked
//! against a failed sync followed by complete and refused ones. Its own
//! test binary, so it never runs beside `exfat_e2e.rs`, which counts the
//! files on the shared card image. Without the images it says so.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{TempDir, at, date_command, manifest, photosync_fed, write_file};

const TAKEN: u64 = 1_700_000_000;

/// Removes this test's photos from the shared card image, failing or not.
struct OffTheCard(PathBuf);

impl Drop for OffTheCard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn mounts() -> Option<(PathBuf, PathBuf)> {
    let (Some(card), Some(drive)) = (
        std::env::var_os("PHOTOSYNC_TEST_CARD"),
        std::env::var_os("PHOTOSYNC_TEST_LIBRARY"),
    ) else {
        eprintln!(
            "SKIPPED one_card_after_another_keeps_the_worst_outcome_on_exfat: \
             PHOTOSYNC_TEST_CARD or PHOTOSYNC_TEST_LIBRARY is not set \
             (scripts/test_exfat.sh sets them)"
        );
        return None;
    };
    Some((
        fs::canonicalize(card).unwrap(),
        fs::canonicalize(drive).unwrap(),
    ))
}

/// One reader, five cards in one session: a complete one, one whose photo
/// collides with the first's (failed), the mounted card found by Enter,
/// a card that is gone (refused), and the mounted card again, which is all
/// "already on drive". Later complete and refused syncs do not hide the
/// failed one: the summary names each sync that was not complete, and the
/// exit code is 1, not 0 or 2.
#[test]
fn one_card_after_another_keeps_the_worst_outcome_on_exfat() {
    let Some((card, drive)) = mounts() else {
        return;
    };
    let ours = card.join("DCIM/901_SESSION");
    let _cleanup = OffTheCard(ours);
    write_file(
        &card,
        "DCIM/901_SESSION/DSCF9101.RAF",
        b"card raw",
        at(TAKEN),
    );
    let card_before = manifest(&card);
    // Plain folders on the temp image: another device than the drive.
    let dir = TempDir::new();
    let (x, y, gone) = (dir.0.join("x"), dir.0.join("y"), dir.0.join("gone"));
    write_file(&x, "DCIM/100_FUJI/DSCF7001.RAF", b"x raw", at(TAKEN));
    write_file(&x, "DCIM/100_FUJI/DSCF7001.JPG", b"x jpeg", at(TAKEN));
    write_file(
        &y,
        "DCIM/100_FUJI/DSCF7001.RAF",
        b"another camera",
        at(TAKEN),
    );
    let library = drive.join("Session");
    fs::create_dir(&library).unwrap();
    let root = card.parent().unwrap();

    let args: [&std::ffi::OsStr; 9] = [
        "sync".as_ref(),
        "--source".as_ref(),
        x.as_os_str(),
        "--dest".as_ref(),
        library.as_os_str(),
        "--name".as_ref(),
        "Trip".as_ref(),
        "--one-by-one".as_ref(),
        "--search-root".as_ref(),
    ];
    let mut args: Vec<&std::ffi::OsStr> = args.to_vec();
    args.push(root.as_os_str());
    let input = format!("{}\n\n{}\n\nq\n", y.display(), gone.display());
    let run = photosync_fed(args, input.as_bytes());

    assert_eq!(run.code, 1, "{run:?}");
    let folder = library.join(format!("{} - Trip", date_command(TAKEN)));
    assert_eq!(fs::read(folder.join("RAW/DSCF7001.RAF")).unwrap(), b"x raw");
    assert_eq!(
        fs::read(folder.join("JPEG/DSCF7001.JPG")).unwrap(),
        b"x jpeg"
    );
    assert_eq!(
        fs::read(folder.join("RAW/DSCF9101.RAF")).unwrap(),
        b"card raw"
    );
    assert_eq!(
        fs::read_dir(&library).unwrap().count(),
        1,
        "one session folder"
    );
    assert_eq!(manifest(&card), card_before, "INV-1: the card changed");

    // Each sync's report as it ends, then the session's summary.
    assert_eq!(run.stdout.matches("\nResult: ").count(), 5, "{run:?}");
    assert!(
        run.stdout.contains(&format!(
            "  failed      {}: name collision with {}",
            y.join("DCIM/100_FUJI/DSCF7001.RAF").display(),
            folder.join("RAW/DSCF7001.RAF").display()
        )),
        "{run:?}"
    );
    assert_eq!(
        run.stderr
            .matches(&format!("found {}\n", card.display()))
            .count(),
        2,
        "{run:?}"
    );
    assert!(
        run.stderr
            .contains(&format!("Cannot read {}", gone.display())),
        "{run:?}"
    );
    let summary = [
        "Session of 5 syncs".to_string(),
        format!("Library {}", library.display()),
        format!("Folder  {}", folder.display()),
        format!("  sync 1 ({}): complete", x.display()),
        format!(
            "  sync 2 ({}): 1 failed, 0 cancelled, 0 unreadable",
            y.display()
        ),
        format!("  sync 3 ({}): complete", card.display()),
        format!("  sync 4 ({}): refused, nothing copied", gone.display()),
        format!("  sync 5 ({}): complete", card.display()),
        String::new(),
        "All syncs".to_string(),
    ];
    let at = run.stdout.find("Session of 5 syncs").expect("no summary");
    let tail = &run.stdout[at..];
    assert!(tail.starts_with(&(summary.join("\n") + "\n")), "{tail}");
    assert!(
        tail.ends_with(&format!(
            "\nResult: NOT COMPLETE: 2 of 5 syncs not complete: sync 2 ({}): 1 failed, 0 \
             cancelled, 0 unreadable; sync 4 ({}): refused, nothing copied. Do not format the \
             cards of those syncs. Fix the cause and sync them again: files already on the \
             drive are skipped.\n",
            y.display(),
            gone.display()
        )),
        "{tail}"
    );
    // The same card again: nothing copied, all already there.
    let last = &run.stdout[..at];
    let last = &last[last.rfind("Report\n").unwrap()..];
    assert!(last.contains(" (0 copied, "), "{last}");
}
