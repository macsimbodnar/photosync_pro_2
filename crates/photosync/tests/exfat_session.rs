//! S020: `photosync sync --one-by-one` end to end on exFAT images: one card
//! after another into one session, with
//! real copies, so the session's summary and exit code can be checked
//! against a failed sync followed by complete and refused ones. Its own
//! test binary, so it never runs beside `exfat_e2e.rs`, which counts the
//! files on the shared card image. The cards are typed at the prompt on a
//! pseudo-terminal, as it asks only on a terminal. On macOS
//! `ignored_image_tests_pass_on_real_mounts` below mounts the images and runs
//! these tests. Without the images each test fails.

#![cfg(unix)]

mod common;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use common::{TempDir, at, date_command, manifest, photosync_tty, write_file};

const TAKEN: u64 = 1_700_000_000;

/// Removes this test's photos from the shared card image, failing or not.
struct OffTheCard(PathBuf);

impl Drop for OffTheCard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn mounts() -> (PathBuf, PathBuf) {
    (
        common::image_path(common::images::CARD),
        common::image_path(common::images::LIBRARY),
    )
}

/// F04: the gate itself proves the worst-outcome exit code (M29).
#[cfg(target_os = "macos")]
#[test]
fn ignored_image_tests_pass_on_real_mounts() {
    use common::images::{CARD, LIBRARY, ignored_tests_pass_on_images};
    ignored_tests_pass_on_images(
        &[CARD, LIBRARY],
        &[
            "one_card_after_another_keeps_the_worst_outcome_on_exfat",
            "a_refused_sync_fixed_and_retried_exits_0",
            "a_refused_sync_of_two_cards_names_the_one_not_completed",
            "a_swapped_card_does_not_clear_a_refused_sync",
        ],
    );
}

/// S067: variables that name what is no image delete nothing, and each test here says so.
#[cfg(unix)]
#[test]
fn ignored_image_tests_delete_nothing_outside_images() {
    use common::images::{CARD, LIBRARY, ignored_tests_delete_nothing_outside_images};
    ignored_tests_delete_nothing_outside_images(&[CARD, LIBRARY]);
}

/// One reader, five cards in one session: a complete one, one whose photo
/// collides with the first's (failed), the mounted card found by Enter,
/// a card that is gone (refused), and the mounted card again, which is all
/// "already on drive". Later complete and refused syncs do not hide the
/// failed one: the summary names each sync that was not complete, and the
/// exit code is 1, not 0 or 2.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn one_card_after_another_keeps_the_worst_outcome_on_exfat() {
    let (card, drive) = mounts();
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
    let run = photosync_tty(args, input.as_bytes());

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
             cards of those syncs. After a name collision, fix any other cause, then start \
             photosync again to sync them under another name: files already on the drive \
             are skipped.\n",
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

/// S040: one card after another. The first sync's library problem leaves it
/// "complete", and the second's report pushes it off the screen: the summary
/// still points back at it, right above the result.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn the_session_summary_points_back_at_a_library_problem_of_an_earlier_sync() {
    mounts();
    let dir = TempDir::new();
    let (first, backup) = (dir.0.join("first"), dir.0.join("backup"));
    let library = common::OnDrive::new(&common::disk_of_the_binary(), "s040-session");
    write_file(&first, "DCIM/100_FUJI/DSCF0001.RAF", b"twice", at(TAKEN));
    write_file(&first, "DCIM/100_FUJI/DSCF9000.RAF", b"new", at(TAKEN));
    common::lock_in_library(&library.0, "old/DSCF9000.RAF", b"old");
    write_file(&backup, "DCIM/100_FUJI/DSCF0001.RAF", b"twice", at(TAKEN));

    let args: [&std::ffi::OsStr; 6] = [
        "sync".as_ref(),
        "--source".as_ref(),
        first.as_os_str(),
        "--dest".as_ref(),
        library.0.as_os_str(),
        "--one-by-one".as_ref(),
    ];
    let input = format!("{}\nq\n", backup.display());
    let run = photosync_tty(args, input.as_bytes());

    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout
            .contains(&format!("  sync 1 ({}): complete\n", first.display())),
        "{run:?}"
    );
    let last: Vec<&str> = run.stdout.lines().rev().take(3).collect();
    assert_eq!(
        last[2], "1 problem line in the reports above: read it.",
        "{run:?}"
    );
    assert!(last[0].starts_with("Result: all 3 "), "{run:?}");
}

/// A session on the terminal, `--one-by-one` from the given cards into a
/// library that is missing at first. At each prompt, `answer` runs and says
/// what to type: so a test can make the library, or swap a card, when the
/// user would.
fn session_at_prompts(
    cards: &[&Path],
    library: &Path,
    mut answer: impl FnMut(usize) -> String,
) -> common::Run {
    let mut pty = common::Pty::open();
    let mut command = Command::new(env!("CARGO_BIN_EXE_photosync"));
    command.arg("sync");
    for card in cards {
        command.arg("--source").arg(card);
    }
    let mut child = command
        .arg("--dest")
        .arg(library)
        .arg("--one-by-one")
        .stdin(pty.stdin())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let stderr = child.stderr.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut all = String::new();
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap();
            all.push_str(&line);
            all.push('\n');
            if line.starts_with("Insert the next card") {
                let _ = tx.send(());
            }
        }
        all
    });
    let mut prompt = 0;
    while rx.recv_timeout(Duration::from_secs(60)).is_ok() {
        prompt += 1;
        writeln!(pty.keys, "{}", answer(prompt)).unwrap();
    }
    let stderr = reader.join().unwrap();
    let out = child.wait_with_output().unwrap();
    drop(pty);
    common::Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr,
    }
}

/// S032 (F07), the finding's own case: the library is missing, the user
/// makes it at the prompt, the same card then completes. Every photo is on
/// the drive: exit 0, and no "do not format". The summary keeps the refusal.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn a_refused_sync_fixed_and_retried_exits_0() {
    let (_, drive) = mounts();
    let dir = TempDir::new();
    let x = dir.0.join("x");
    write_file(&x, "DCIM/100_FUJI/DSCF7301.RAF", b"x raw", at(TAKEN));
    let ours = common::OnDrive::new(&drive, "s032-fixed");
    let library = ours.0.join("Lib");

    let run = session_at_prompts(&[&x], &library, |prompt| match prompt {
        1 => {
            fs::create_dir(&library).unwrap();
            x.display().to_string()
        }
        _ => "q".to_string(),
    });

    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stderr.contains("does not exist."), "{run:?}");
    assert!(
        run.stdout.contains(&format!(
            "  sync 1 ({0}): refused, nothing copied; its cards completed later\n  sync 2 ({0}): complete\n",
            x.display()
        )),
        "{run:?}"
    );
    let result = run.stdout.lines().last().unwrap();
    assert_eq!(
        result, "Result: all 1 photo or video on the drive (1 copied, 0 already there), over 2 syncs.",
        "{run:?}"
    );
    assert!(!run.stdout.contains("NOT COMPLETE"), "{run:?}");
    assert!(!run.stdout.to_lowercase().contains("do not format"), "{run:?}");
}

/// S032: x and y refused, then x alone complete. y is not on the drive: the
/// warning stays, says which card, and the exit code stays 2.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn a_refused_sync_of_two_cards_names_the_one_not_completed() {
    let (_, drive) = mounts();
    let dir = TempDir::new();
    let (x, y) = (dir.0.join("x"), dir.0.join("y"));
    write_file(&x, "DCIM/100_FUJI/DSCF7401.RAF", b"x raw", at(TAKEN));
    write_file(&y, "DCIM/100_FUJI/DSCF7402.RAF", b"y raw", at(TAKEN));
    let ours = common::OnDrive::new(&drive, "s032-two");
    let library = ours.0.join("Lib");

    let run = session_at_prompts(&[&x, &y], &library, |prompt| match prompt {
        1 => {
            fs::create_dir(&library).unwrap();
            x.display().to_string()
        }
        _ => "q".to_string(),
    });

    assert_eq!(run.code, 2, "{run:?}");
    let state = format!(
        "sync 1 ({}, {}): refused, nothing copied; still to sync: {}",
        x.display(),
        y.display(),
        y.display()
    );
    assert!(run.stdout.contains(&format!("  {state}\n")), "{run:?}");
    assert!(
        run.stdout.ends_with(&format!(
            "\nResult: NOT COMPLETE: 1 of 2 syncs not complete: {state}. Do not format the \
             cards of those syncs. Fix the cause and sync them again: files already on the \
             drive are skipped.\n"
        )),
        "{run:?}"
    );
}

/// S032: one reader: the refused card is swapped for another, at the same
/// path, and that one completes. The refused card's photos are not on the
/// drive, so the warning stays.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn a_swapped_card_does_not_clear_a_refused_sync() {
    let (_, drive) = mounts();
    let dir = TempDir::new();
    let x = dir.0.join("x");
    write_file(&x, "DCIM/100_FUJI/DSCF7501.RAF", b"first card", at(TAKEN));
    let ours = common::OnDrive::new(&drive, "s032-swap");
    let library = ours.0.join("Lib");

    let run = session_at_prompts(&[&x], &library, |prompt| match prompt {
        1 => {
            fs::create_dir(&library).unwrap();
            fs::remove_dir_all(x.join("DCIM")).unwrap();
            write_file(&x, "DCIM/100_FUJI/DSCF7502.RAF", b"second card", at(TAKEN));
            x.display().to_string()
        }
        _ => "q".to_string(),
    });

    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.stdout.contains(&format!(
            "Result: NOT COMPLETE: 1 of 2 syncs not complete: sync 1 ({}): refused, nothing copied.",
            x.display()
        )),
        "{run:?}"
    );
}
