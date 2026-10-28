//! The TUI's loop on a `TestBackend` with scripted keys: the real worker
//! thread and the real core, no terminal. The full sync needs the exFAT
//! images of `scripts/test_exfat.sh` (strict INV-10 needs three devices);
//! without them it says it was skipped.

use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use photosync_core::{Volumes, discover_volumes};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::app::{Outcome, Screen};
use crate::args::{EXIT_INCOMPLETE, EXIT_OK, EXIT_REFUSED};
use crate::common::{TempDir, at, date_command, manifest, write_file};

const TAKEN: u64 = 1_700_000_000;

fn rows(backend: &TestBackend) -> Vec<String> {
    let buffer = backend.buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// A key script: each step waits for its screen, then sends its keys.
/// Gives up after a minute, so a stuck sync fails instead of hanging.
fn script(steps: Vec<(Screen, Vec<Key>)>) -> impl FnMut(&App) -> io::Result<Option<Key>> {
    let mut queue: Vec<(Screen, Key)> = steps
        .into_iter()
        .flat_map(|(screen, keys)| keys.into_iter().map(move |k| (screen, k)))
        .rev()
        .collect();
    let deadline = Instant::now() + Duration::from_secs(60);
    move |app| {
        assert!(
            Instant::now() < deadline,
            "stuck on {:?}: {app:?}",
            app.screen()
        );
        match queue.last() {
            Some((screen, _)) if *screen == app.screen() => Ok(queue.pop().map(|(_, k)| k)),
            _ => {
                std::thread::sleep(Duration::from_millis(5));
                Ok(None)
            }
        }
    }
}

fn chars(text: &str) -> Vec<Key> {
    text.chars().map(Key::Char).collect()
}

/// Every key the model knows comes from its crossterm key, and Ctrl-C is a
/// key in raw mode, not a signal.
#[test]
fn crossterm_keys_map_to_model_keys() {
    let k = |code, mods| key(KeyEvent::new(code, mods));
    let none = KeyModifiers::NONE;
    assert_eq!(
        k(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Some(Key::CtrlC)
    );
    assert_eq!(k(KeyCode::Char('c'), none), Some(Key::Char('c')));
    assert_eq!(
        k(KeyCode::Char('Q'), KeyModifiers::SHIFT),
        Some(Key::Char('Q'))
    );
    assert_eq!(k(KeyCode::Char('x'), KeyModifiers::CONTROL), None);
    assert_eq!(k(KeyCode::Char('x'), KeyModifiers::ALT), None);
    for (code, want) in [
        (KeyCode::Up, Key::Up),
        (KeyCode::Down, Key::Down),
        (KeyCode::PageUp, Key::PageUp),
        (KeyCode::PageDown, Key::PageDown),
        (KeyCode::Enter, Key::Enter),
        (KeyCode::Esc, Key::Esc),
        (KeyCode::Backspace, Key::Backspace),
    ] {
        assert_eq!(k(code, none), Some(want));
    }
    assert_eq!(k(KeyCode::F(1), none), None);
}

/// The guard restores while a panic unwinds, as it does on return.
#[test]
fn restore_runs_on_panic() {
    let (tx, rx) = mpsc::channel();
    let result = panic::catch_unwind(move || {
        let _restore = Restore(move || tx.send(()).unwrap());
        panic!("boom");
    });
    assert!(result.is_err());
    assert!(rx.try_recv().is_ok(), "not restored");
}

/// S018: the panic hook in place while the TUI runs (ratatui's restores
/// the terminal) acts for the TUI's own thread only. A worker's panic must
/// leave the screen in raw mode for the loop that keeps drawing on it.
#[test]
fn panic_hook_acts_for_the_tui_thread_only() {
    let calls = Arc::new(AtomicUsize::new(0));
    let previous = panic::take_hook();
    let counted = calls.clone();
    // Stands in for ratatui's hook, which `run` gets from `try_init`.
    panic::set_hook(Box::new(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
    }));
    restore_on_panic_of_this_thread_only();

    let worker = std::thread::spawn(|| panic!("worker")).join();
    let after_worker = calls.load(Ordering::SeqCst);
    let tui = panic::catch_unwind(|| panic!("tui"));
    let after_tui = calls.load(Ordering::SeqCst);
    panic::set_hook(previous);

    assert!(worker.is_err() && tui.is_err());
    assert_eq!(
        (after_worker, after_tui),
        (0, 1),
        "hook calls: worker, then tui"
    );
}

/// The loop on temp dirs: one device holds card and library, so the core
/// refuses, and the screen shows the CLI's sentence; quitting exits 2.
#[test]
fn refused_sync_shows_the_cli_sentence_and_exits_2() {
    let dir = TempDir::new();
    let card = dir.0.join("CARD");
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(TAKEN));
    let drive = dir.0.join("SSD");
    fs::create_dir_all(drive.join("Photos")).unwrap();
    let before = manifest(&dir.0);
    let found = || Volumes {
        sources: vec![card.clone()],
        destinations: vec![drive.clone()],
        failures: Vec::new(),
    };
    // Wide, so the sentence with its temp paths fits on one row.
    let mut terminal = Terminal::new(TestBackend::new(500, 20)).unwrap();
    let keys = script(vec![
        (Screen::Sources, vec![Key::Enter]),
        (Screen::Drive, vec![Key::Enter]),
        (Screen::Library, vec![Key::Enter]),
        (Screen::Name, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        (Screen::Refused, vec![Key::Char('q')]),
    ]);

    let app = run_loop(&mut terminal, keys, found).unwrap();

    let sentence = format!(
        "The library {} is on the same drive as source {}.",
        drive.join("Photos").display(),
        card.display()
    );
    assert!(
        rows(terminal.backend())
            .iter()
            .any(|r| r.contains(&sentence)),
        "{:#?}",
        rows(terminal.backend())
    );
    let exit = app.exit();
    assert_eq!(exit.code, EXIT_REFUSED);
    assert!(exit.message.unwrap().starts_with(&sentence));
    assert_eq!(manifest(&dir.0), before, "something was written");
}

/// The exFAT card and drive images, or `None` outside the script.
fn mounts() -> Option<(PathBuf, PathBuf)> {
    let (Some(card), Some(drive)) = (
        std::env::var_os("PHOTOSYNC_TEST_CARD"),
        std::env::var_os("PHOTOSYNC_TEST_LIBRARY"),
    ) else {
        eprintln!(
            "SKIPPED tui_sync_on_exfat_end_to_end: PHOTOSYNC_TEST_CARD or \
             PHOTOSYNC_TEST_LIBRARY is not set (scripts/test_exfat.sh sets them)"
        );
        return None;
    };
    Some((
        fs::canonicalize(card).unwrap(),
        fs::canonicalize(drive).unwrap(),
    ))
}

/// Removes this test's photos from the shared card image, failing or not:
/// `tests/exfat_e2e.rs` counts the card's files.
struct OffTheCard(PathBuf);

impl Drop for OffTheCard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A cancelled sync, then a full one, through the TUI on real mounts: the
/// cards found by discovery, the drive picked, the library typed, a name,
/// the worker thread, the report screen and the exit code.
#[test]
fn tui_sync_on_exfat_end_to_end() {
    let Some((card, drive)) = mounts() else {
        return;
    };
    let ours = card.join("DCIM/900_TUI");
    let _cleanup = OffTheCard(ours.clone());
    write_file(&card, "DCIM/900_TUI/DSCF9001.RAF", b"tui raw", at(TAKEN));
    write_file(&card, "DCIM/900_TUI/DSCF9001.JPG", b"tui jpeg", at(TAKEN));
    write_file(
        &card,
        "DCIM/900_TUI/DSCF9002.MOV",
        b"tui video",
        at(TAKEN + 60),
    );
    let library = drive.join("TUI");
    fs::create_dir(&library).unwrap();
    let card_before = manifest(&card);
    let root = card.parent().unwrap().to_path_buf();
    let found = || discover_volumes(std::slice::from_ref(&root));
    assert_eq!(
        found().sources,
        std::slice::from_ref(&card),
        "discovery finds the card"
    );
    let at_drive = found()
        .destinations
        .iter()
        .position(|d| *d == drive)
        .unwrap();

    // Cancelled before the worker starts, so no file can slip through.
    let mut app = App::new();
    app.update(Msg::Volumes(found()));
    let mut to_library = vec![Key::Enter];
    to_library.extend(vec![Key::Down; at_drive]);
    to_library.push(Key::Enter);
    for k in &to_library {
        app.update(Msg::Key(*k));
    }
    for _ in 0.."Photos".len() {
        app.update(Msg::Key(Key::Backspace));
    }
    let mut rest = chars("TUI");
    rest.push(Key::Enter);
    rest.extend(chars("Tui"));
    rest.push(Key::Enter);
    for k in rest {
        app.update(Msg::Key(k));
    }
    let Some(Effect::Start(job)) = app.update(Msg::Key(Key::Enter)) else {
        panic!("not started: {app:?}");
    };
    assert_eq!(job.library(), library);
    assert_eq!(job.name().as_deref(), Some("Tui"));
    app.update(Msg::Key(Key::Char('c')));
    let (tx, rx) = mpsc::channel();
    worker::spawn(job, move |m| tx.send(m).unwrap());
    for msg in rx {
        app.update(msg);
    }
    assert_eq!(app.screen(), Screen::Report);
    let Some(Outcome::Report { result, .. }) = app.outcome() else {
        panic!("no report: {app:?}");
    };
    assert!(
        result.starts_with("Result: NOT COMPLETE: 0 failed, "),
        "{result}"
    );
    assert!(result.contains("Do not format the card."), "{result}");
    assert_eq!(app.exit().code, EXIT_INCOMPLETE);
    let files = |dir: &Path| {
        manifest(dir)
            .into_values()
            .filter(|(_, b)| b.is_some())
            .count()
    };
    assert_eq!(files(&library), 0, "copied after the cancel");
    let folder = library.join(format!("{} - Tui", date_command(TAKEN)));

    // The full sync, every step by key through the real loop.
    let mut terminal = Terminal::new(TestBackend::new(500, 60)).unwrap();
    let mut library_keys = vec![Key::Backspace; "Photos".len()];
    library_keys.extend(chars("TUI"));
    library_keys.push(Key::Enter);
    let mut drive_keys = vec![Key::Down; at_drive];
    drive_keys.push(Key::Enter);
    let mut name_keys = chars("Tui");
    name_keys.push(Key::Enter);
    let keys = script(vec![
        (Screen::Sources, vec![Key::Char(' '), Key::Enter]),
        (Screen::Drive, drive_keys),
        (Screen::Library, library_keys),
        (Screen::Name, name_keys),
        (Screen::Confirm, vec![Key::Enter]),
        // S020: the next card in the same session, here the same card
        // again: no drive, library or name to give.
        (Screen::Report, vec![Key::Char('n')]),
        (Screen::Sources, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        (Screen::Report, vec![Key::Char('q')]),
    ]);

    let app = run_loop(&mut terminal, keys, found).unwrap();

    for (rel, bytes) in [
        ("RAW/DSCF9001.RAF", &b"tui raw"[..]),
        ("JPEG/DSCF9001.JPG", b"tui jpeg"),
        ("VIDEO/DSCF9002.MOV", b"tui video"),
    ] {
        assert_eq!(fs::read(folder.join(rel)).unwrap(), bytes, "{rel}");
    }
    assert_eq!(manifest(&card), card_before, "INV-1: the card changed");
    let screen = rows(terminal.backend());
    assert!(
        screen
            .iter()
            .any(|r| r.contains("Result: all ") && r.contains("on the drive")),
        "{screen:#?}"
    );
    assert!(
        screen
            .iter()
            .any(|r| r.contains(&format!("Source {}", card.display()))),
        "{screen:#?}"
    );
    let exit = app.exit();
    assert_eq!(exit.code, EXIT_OK, "{exit:?}");
    // What is printed after the screen closes is the session's report:
    // both syncs, where the photos went, and the result of all of them.
    let report = exit.report.unwrap();
    assert!(
        report.starts_with(&format!(
            "Report\nLibrary {}\nFolder  {}\n\nSync 1 of 2: {}\n",
            library.display(),
            folder.display(),
            card.display()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!("\nSync 2 of 2: {}\n", card.display())),
        "{report}"
    );
    assert!(report.contains("\nAll syncs\n"), "{report}");
    // The card again is all already on the drive, not twins of itself.
    let second = &report[report.find("Sync 2 of 2").unwrap()..report.find("All syncs").unwrap()];
    assert!(second.contains(" (0 copied, "), "{second}");
    assert!(
        report
            .trim_end()
            .lines()
            .last()
            .unwrap()
            .starts_with("Result: all ")
            && report.trim_end().ends_with("over 2 syncs."),
        "{report}"
    );
    assert!(exit.message.is_none(), "{:?}", exit.message);
}
