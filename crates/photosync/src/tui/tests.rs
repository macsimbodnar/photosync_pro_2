//! The TUI's loop on a `TestBackend` with scripted keys: the real worker
//! thread and the real core, no terminal. The full sync needs exFAT images
//! (strict INV-10 needs three devices): `#[ignore]`d, and on macOS
//! `tests::ignored_image_tests_pass_on_real_mounts` in `main.rs` runs it.

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

/// The words of a report whose lines wrap: `w<n>` each, so a word seen is a
/// part of a line read, and one never seen is a part that cannot be read.
fn numbered_report(lines: usize, words_per_line: usize) -> (Vec<String>, Vec<String>) {
    let words: Vec<String> = (0..lines * words_per_line).map(|n| format!("w{n}.")).collect();
    let report = words
        .chunks(words_per_line)
        .map(|line| line.join(" "))
        .collect();
    (report, words)
}

fn report_app(lines: Vec<String>, result: String) -> App {
    App::with_outcome_for_test(Outcome::Report {
        lines,
        result,
        complete: false,
    })
}

/// Sends `key` until the screen stops changing, keeping every row shown.
/// Each draw goes through what `run_loop` draws with.
fn read_with(key: Key, app: &mut App, terminal: &mut Terminal<TestBackend>) -> Vec<String> {
    let mut seen = Vec::new();
    loop {
        draw_frame(terminal, app).unwrap();
        let screen = rows(terminal.backend());
        let same = seen.ends_with(&screen);
        if same {
            return seen;
        }
        seen.extend(screen);
        app.update(Msg::Key(key));
    }
}

fn missing(words: &[String], seen: &[String]) -> Vec<String> {
    let seen: std::collections::HashSet<&str> =
        seen.iter().flat_map(|row| row.split_whitespace()).collect();
    words
        .iter()
        .filter(|w| !seen.contains(w.as_str()))
        .cloned()
        .collect()
}

/// S037: wrapping makes more rows than lines, so scrolling by lines would stop
/// short of the end. A small terminal must not lose what a page key jumps over.
#[test]
fn every_part_of_a_wrapped_report_is_reached_by_line_and_by_page() {
    for (width, height) in [(80, 24), (40, 12), (30, 9), (80, 40)] {
        for key in [Key::Down, Key::PageDown] {
            let (lines, words) = numbered_report(30, 40);
            let mut app = report_app(lines, "Result: NOT COMPLETE.".into());
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let seen = read_with(key, &mut app, &mut terminal);
            assert_eq!(
                missing(&words, &seen),
                Vec::<String>::new(),
                "{width}x{height} with {key:?}"
            );
        }
    }
}

/// One line taller than the screen: its end is below the screen until the
/// line's own rows scroll, so scrolling from line to line would lose it.
#[test]
fn a_line_taller_than_the_screen_is_read_to_its_end() {
    let (lines, words) = numbered_report(1, 300);
    let mut app = report_app(lines, "Result: NOT COMPLETE.".into());
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    let seen = read_with(Key::Down, &mut app, &mut terminal);
    assert_eq!(missing(&words, &seen), Vec::<String>::new());
}

/// Back up, the report's start shows again.
#[test]
fn scrolling_back_up_reaches_the_start() {
    let (lines, words) = numbered_report(30, 40);
    let mut app = report_app(lines, "Result: NOT COMPLETE.".into());
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    read_with(Key::PageDown, &mut app, &mut terminal);
    let seen = read_with(Key::PageUp, &mut app, &mut terminal);
    assert_eq!(missing(&words, &seen), Vec::<String>::new());
    let top = rows(terminal.backend());
    assert!(top.iter().any(|r| r.contains("w0. ")), "{top:#?}");
}

/// A resize changes how the report wraps: the end stays in reach, and a
/// larger screen fills up rather than showing blank rows under the end.
#[test]
fn a_resize_keeps_the_end_of_the_report_in_reach() {
    let (lines, words) = numbered_report(30, 40);
    let end = words.last().unwrap().as_str();
    let mut app = report_app(lines, "Result: NOT COMPLETE.".into());
    let mut small = Terminal::new(TestBackend::new(40, 12)).unwrap();
    read_with(Key::PageDown, &mut app, &mut small);
    let mut wide = Terminal::new(TestBackend::new(120, 30)).unwrap();
    draw_frame(&mut wide, &mut app).unwrap();
    let screen = rows(wide.backend());
    assert!(screen.iter().any(|r| r.contains(end)), "{screen:#?}");
    assert!(screen[..6].iter().all(|r| !r.trim().is_empty()), "{screen:#?}");
}

/// A result too tall for a box over the report leaves no room for the report.
/// It comes first, in the part that scrolls, so all of it can be read.
#[test]
fn a_result_taller_than_the_screen_is_read_to_its_end() {
    let (lines, words) = numbered_report(5, 5);
    let (_, result_words) = numbered_report(1, 200);
    let result = result_words.join(" ");
    let mut app = report_app(lines, result);
    let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    draw_frame(&mut terminal, &mut app).unwrap();
    let top = rows(terminal.backend());
    assert!(top.iter().any(|r| r.contains("w0. ")), "{top:#?}");
    let seen = read_with(Key::Down, &mut app, &mut terminal);
    assert_eq!(missing(&result_words, &seen), Vec::<String>::new());
    assert_eq!(missing(&words, &seen), Vec::<String>::new());
}

/// A card can hold more photos than a `u16` of rows: all duplicates, say.
#[test]
fn the_end_of_a_report_over_65535_rows_is_reached() {
    let lines: Vec<String> = (0..70_000).map(|n| format!("line {n}")).collect();
    let mut app = report_app(lines, "Result: NOT COMPLETE.".into());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    draw_frame(&mut terminal, &mut app).unwrap();
    for _ in 0..7_001 {
        app.update(Msg::Key(Key::PageDown));
    }
    draw_frame(&mut terminal, &mut app).unwrap();
    let screen = rows(terminal.backend());
    assert!(screen.iter().any(|r| r.contains("line 69999")), "{screen:#?}");
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

/// S042: the footer of the refused screen offers `n` only in a session, as
/// `App::update` ignores it before one. Every key it offers acts: a script
/// waits for its screen, so a key that does nothing fails here.
#[test]
fn a_refusal_before_the_session_does_not_offer_the_next_card() {
    let dir = TempDir::new();
    let card = dir.0.join("CARD");
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(TAKEN));
    let drive = dir.0.join("SSD");
    fs::create_dir_all(&drive).unwrap();
    let found = || Volumes {
        sources: vec![card.clone()],
        destinations: vec![drive.clone()],
        failures: Vec::new(),
    };
    let mut terminal = Terminal::new(TestBackend::new(500, 20)).unwrap();
    let keys = script(vec![
        (Screen::Sources, vec![Key::Enter]),
        (Screen::Drive, vec![Key::Enter]),
        (Screen::Library, vec![Key::Enter]),
        (Screen::Name, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        // `n` does nothing here: it must not move the script off this screen.
        (Screen::Refused, vec![Key::Char('n'), Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        (Screen::Refused, vec![Key::Char('q')]),
    ]);

    let app = run_loop(&mut terminal, keys, found).unwrap();

    assert!(!app.in_session(), "the session started: {app:?}");
    let screen = rows(terminal.backend());
    let has = |part: &str| screen.iter().any(|r| r.contains(part));
    assert!(!has("n next card"), "{screen:#?}");
    assert!(has("Enter back") && has("q quit"), "{screen:#?}");
}

/// S042: in a session the refused screen still offers `n`, and Enter, `n`
/// and `q` all act. Card and library on one device: the core refuses.
#[test]
fn a_refusal_in_the_session_offers_the_next_card() {
    let dir = TempDir::new();
    let card = dir.0.join("CARD");
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(TAKEN));
    let drive = dir.0.join("SSD");
    fs::create_dir_all(drive.join("Photos")).unwrap();
    let found = || Volumes {
        sources: vec![card.clone()],
        destinations: vec![drive.clone()],
        failures: Vec::new(),
    };
    let mut terminal = Terminal::new(TestBackend::new(500, 20)).unwrap();
    let keys = script(vec![
        (Screen::Sources, vec![Key::Enter]),
        (Screen::Drive, vec![Key::Enter]),
        (Screen::Library, vec![Key::Enter]),
        (Screen::Name, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        (Screen::Refused, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        (Screen::Refused, vec![Key::Char('n')]),
        (Screen::Sources, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
        (Screen::Refused, vec![Key::Char('q')]),
    ]);

    let app = run_loop(&mut terminal, keys, found).unwrap();

    assert!(app.in_session(), "no session: {app:?}");
    assert_eq!(app.syncs(), 3);
    let screen = rows(terminal.backend());
    let has = |part: &str| screen.iter().any(|r| r.contains(part));
    assert!(
        has("Enter back") && has("n next card") && has("q quit"),
        "{screen:#?}"
    );
}

/// S032 (F07): on a fresh drive with no `<drive>/Photos`, the loop refuses
/// at the check, before a session exists or a card is read, so the library
/// can still be changed. Nothing is made on the drive. With the library
/// fixed, the session starts, and the core has its say (same drive here).
#[test]
fn a_fresh_drive_is_refused_before_the_session_starts() {
    let dir = TempDir::new();
    let card = dir.0.join("CARD");
    write_file(&card, "DCIM/100_FUJI/DSCF0001.RAF", b"raw", at(TAKEN));
    let drive = dir.0.join("SSD");
    fs::create_dir_all(drive.join("Lib")).unwrap();
    let before = manifest(&dir.0);
    let found = || Volumes {
        sources: vec![card.clone()],
        destinations: vec![drive.clone()],
        failures: Vec::new(),
    };
    let to_check = || {
        vec![
            (Screen::Sources, vec![Key::Enter]),
            (Screen::Drive, vec![Key::Enter]),
            (Screen::Library, vec![Key::Enter]),
            (Screen::Name, vec![Key::Enter]),
            (Screen::Confirm, vec![Key::Enter]),
        ]
    };
    let mut terminal = Terminal::new(TestBackend::new(500, 20)).unwrap();

    let mut steps = to_check();
    steps.push((Screen::Refused, vec![Key::Char('q')]));
    let app = run_loop(&mut terminal, script(steps), found).unwrap();

    assert!(!app.in_session(), "the session started: {app:?}");
    assert_eq!(app.syncs(), 0);
    let exit = app.exit();
    assert_eq!(exit.code, EXIT_REFUSED);
    let sentence = format!(
        "The library folder {} does not exist.",
        drive.join("Photos").display()
    );
    assert!(exit.message.unwrap().starts_with(&sentence));
    assert_eq!(manifest(&dir.0), before, "something was written");

    let mut library_keys = vec![Key::Backspace; "Photos".len()];
    library_keys.extend(chars("Lib"));
    library_keys.push(Key::Enter);
    let mut steps = to_check();
    steps.extend([
        (Screen::Refused, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Esc]),
        (Screen::Name, vec![Key::Esc]),
        (Screen::Library, library_keys),
        (Screen::Name, vec![Key::Enter]),
        (Screen::Confirm, vec![Key::Enter]),
    ]);
    let mut keys = script(steps);
    let mut done = false;
    let mut next_key = |app: &App| {
        // The last refusal is the core's, in a session: then quit.
        if app.screen() == Screen::Refused && app.in_session() {
            done = true;
        }
        if done { Ok(Some(Key::Char('q'))) } else { keys(app) }
    };
    let app = run_loop(&mut terminal, &mut next_key, found).unwrap();

    assert_eq!(app.syncs(), 1, "{app:?}");
    let exit = app.exit();
    assert!(
        exit.message
            .unwrap()
            .starts_with(&format!("The library {} is on the same drive", drive.join("Lib").display()))
    );
    assert_eq!(manifest(&dir.0), before, "something was written");
}

/// The exFAT card and drive images.
fn mounts() -> (PathBuf, PathBuf) {
    (
        crate::common::image_path(crate::common::images::CARD),
        crate::common::image_path(crate::common::images::LIBRARY),
    )
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
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn tui_sync_on_exfat_end_to_end() {
    let (card, drive) = mounts();
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
