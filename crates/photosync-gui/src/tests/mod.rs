//! All the GUI's unit tests, in one test binary (DEC-072 (1)).

mod harness;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use iced::{Theme, window};
use iced_test::Simulator;
use iced_test::core::Settings;
use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::StreamExt as _;
use iced_test::futures::futures::channel::mpsc as emulator_channel;
use iced_test::futures::futures::executor::block_on;
use photosync_front::app::Screen;

use crate::args::{Options, parse};
use crate::common::TempDir;
use crate::gui::{self, Discover, Gui, LEAST, LOOKING, M, TITLE, WINDOW, update, view};
use crate::png;
use harness::*;

const CARD_A: &str = "/Volumes/CARD_A";
const CARD_B: &str = "/Volumes/CARD_B";
const NO_CARD: &str =
    "No camera card found. Insert a card, or connect the X-T5 in \"USB card reader\" mode.";

fn two_cards() -> Gui {
    booted(literal(&[CARD_A, CARD_B], &[]))
}

fn sources(gui: &Gui) -> Vec<String> {
    gui.app
        .sources()
        .iter()
        .map(|p| p.display().to_string())
        .collect()
}

fn picked(gui: &Gui) -> Vec<String> {
    gui.app
        .picked()
        .iter()
        .map(|p| p.display().to_string())
        .collect()
}

// The looking state (DEC-073 (2)).

#[test]
fn before_the_first_answer_it_says_looking_and_next_sends_nothing() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    let first = next_look(&looks);

    assert!(gui.looking());
    in_sight(&gui, LEAST, &[LOOKING, "Next", "Look again", "Quit"]);
    assert!(!says(&gui, NO_CARD), "nothing has said there is no card");
    assert!(clicks(&gui, "Next").is_empty(), "Next is held");
    let _ = update(&mut gui, M::Looked);
    assert!(gui.looking(), "nothing has answered yet");

    first.send(volumes(&[CARD_A], &[], &[])).unwrap();
    settle(&mut gui);
    assert!(says(&gui, CARD_A));
    assert!(!says(&gui, LOOKING));
    assert_eq!(clicks(&gui, "Next").len(), 1, "Next is let go");
}

#[test]
fn an_older_look_answering_last_leaves_the_rows_of_the_newest() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    let first = next_look(&looks);
    click(&mut gui, "Look again");
    let second = next_look(&looks);

    second.send(volumes(&[CARD_B], &[], &[])).unwrap();
    settle(&mut gui);
    assert_eq!(sources(&gui), [CARD_B]);

    first.send(volumes(&[CARD_A], &[], &[])).unwrap();
    wait_until(&mut gui, |gui| gui.dropped_looks == 1);
    assert_eq!(sources(&gui), [CARD_B]);
    assert!(says(&gui, CARD_B));
    assert!(!says(&gui, CARD_A));
    assert!(!gui.looking());
}

#[test]
fn an_older_look_answering_first_shows_nothing_and_the_newest_is_still_waited_for() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    let first = next_look(&looks);
    click(&mut gui, "Look again");
    let second = next_look(&looks);

    first.send(volumes(&[CARD_A], &[], &[])).unwrap();
    wait_until(&mut gui, |gui| gui.dropped_looks == 1);
    assert!(gui.looking());
    assert!(says(&gui, LOOKING));
    assert!(!says(&gui, CARD_A));
    assert!(clicks(&gui, "Next").is_empty());

    second.send(volumes(&[CARD_B], &[], &[])).unwrap();
    settle(&mut gui);
    assert_eq!(sources(&gui), [CARD_B]);
}

#[test]
fn while_a_later_look_is_out_no_old_row_shows_and_none_can_be_taken() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A], &[], &["note"]))
        .unwrap();
    settle(&mut gui);
    assert!(says(&gui, CARD_A));
    let row = clicks(&gui, CARD_A);

    click(&mut gui, "Look again");
    let out = next_look(&looks);
    assert!(says(&gui, LOOKING));
    assert!(!says(&gui, CARD_A), "a row of the older look");
    assert!(
        !says(&gui, "warning: cannot scan note: test"),
        "a note of the older look"
    );

    // A click drawn before the look began can still arrive.
    for message in row {
        let _ = update(&mut gui, message);
    }
    let _ = update(&mut gui, M::Next);
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert!(gui.app.picked().is_empty());
    out.send(volumes(&[CARD_A], &[], &[])).unwrap();
    settle(&mut gui);
}

#[test]
fn look_again_stays_while_looking_and_starts_a_newer_look() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks);
    click(&mut gui, "Look again");
    next_look(&looks);
    click(&mut gui, "Look again");
    next_look(&looks);
    assert!(gui.looking());
}

// The cards screen (DEC-068 (2)).

#[test]
fn the_cards_screen_shows_its_rows_notes_and_buttons() {
    let notes = ["/Volumes/BAD"];
    let discover: Discover = Arc::new(move || volumes(&[CARD_A, CARD_B], &[], &notes));
    let mut gui = booted(discover);
    click(&mut gui, CARD_B);
    click(&mut gui, CARD_A);
    click(&mut gui, CARD_A);
    let words = [
        TITLE,
        "1/5  Pick the cards to copy from",
        CARD_A,
        CARD_B,
        "warning: cannot scan /Volumes/BAD: test",
        "Next",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S051", "cards", &words);
    in_sight(&gui, LEAST, &words);
    picture_at(&gui, LEAST, "S051", "cards_least");
}

#[test]
fn an_empty_list_says_so_and_next_stays() {
    let mut gui = booted(literal(&[], &[]));
    let words = [TITLE, NO_CARD, "Next", "Look again", "Quit"];
    screen_check(&gui, "S051", "cards_empty", &words);
    in_sight(&gui, LEAST, &words);
    assert_eq!(gui.app.empty_list_note(), NO_CARD);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Sources);
}

#[test]
fn a_click_on_a_rows_words_ticks_it_and_again_unticks_it() {
    let mut gui = two_cards();
    click(&mut gui, CARD_B);
    assert_eq!(picked(&gui), [CARD_B]);
    assert_eq!(gui.app.cursor(), 1);
    click(&mut gui, CARD_A);
    assert_eq!(picked(&gui), [CARD_A, CARD_B], "in list order");
    assert_eq!(gui.app.cursor(), 0);
    click(&mut gui, CARD_B);
    assert_eq!(picked(&gui), [CARD_A]);
}

#[test]
fn a_row_that_has_gone_is_no_row() {
    let mut gui = two_cards();
    let _ = update(&mut gui, M::Row(PathBuf::from("/Volumes/GONE")));
    assert!(gui.app.picked().is_empty());
    assert_eq!(gui.app.cursor(), 0);
}

#[test]
fn next_takes_the_ticked_cards_to_the_drive_screen() {
    let mut gui = two_cards();
    click(&mut gui, CARD_B);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(picked(&gui), [CARD_B]);
    in_sight(&gui, LEAST, &[TITLE]);
}

#[test]
fn next_with_none_ticked_takes_the_row_under_the_cursor() {
    let mut gui = two_cards();
    click(&mut gui, CARD_B);
    click(&mut gui, CARD_B);
    assert!(gui.app.picked().is_empty());
    click(&mut gui, "Next");
    assert_eq!(picked(&gui), [CARD_B]);
}

#[test]
fn look_again_runs_the_discovery_again() {
    static LOOKS: AtomicU32 = AtomicU32::new(0);
    let discover: Discover = Arc::new(|| {
        let look = LOOKS.fetch_add(1, Ordering::SeqCst);
        volumes(if look == 0 { &[CARD_A] } else { &[CARD_B] }, &[], &[])
    });
    let mut gui = booted(discover);
    assert_eq!(sources(&gui), [CARD_A]);
    click(&mut gui, "Look again");
    settle(&mut gui);
    assert_eq!(sources(&gui), [CARD_B]);
    assert_eq!(LOOKS.load(Ordering::SeqCst), 2);
}

#[test]
fn quit_quits() {
    let mut gui = two_cards();
    assert!(!gui.quit);
    click(&mut gui, "Quit");
    assert!(gui.quit);
}

#[test]
fn quit_works_while_looking() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks);
    click(&mut gui, "Quit");
    assert!(gui.quit);
}

#[test]
fn a_close_on_the_cards_quits() {
    let mut gui = two_cards();
    let _ = update(&mut gui, M::CloseRequested);
    assert!(gui.quit);
}

#[test]
fn the_screens_to_come_are_a_bare_title() {
    let mut gui = two_cards();
    click(&mut gui, "Next");
    assert!(says(&gui, TITLE));
    assert!(
        !says(&gui, "Next"),
        "nothing can go on to a screen not built"
    );
}

// The window's settings, the program being what the real window runs.

#[test]
fn the_window_has_its_title_theme_and_sizes() {
    use iced::Program;
    let program = gui::program(literal(&[], &[]), None);
    let settings = Program::window(&program).expect("a window");
    assert_eq!(settings.size, WINDOW);
    assert_eq!((WINDOW.width, WINDOW.height), (900.0, 650.0));
    assert_eq!(settings.min_size, Some(LEAST));
    assert_eq!((LEAST.width, LEAST.height), (640.0, 480.0));
    assert!(
        !settings.exit_on_close_request,
        "a close cancels a copy first (DEC-069 (4))"
    );
    assert_eq!(
        Program::settings(&program).default_font,
        iced::Font::with_name("Fira Sans")
    );

    let (state, _) = Program::boot(&program);
    let id = window::Id::unique();
    assert_eq!(Program::title(&program, &state, id), TITLE);
    assert_eq!(TITLE, "phoosync pro 2");
    assert_eq!(Program::theme(&program, &state, id), Some(Theme::Dark));
}

// The harness itself: a check that cannot fail proves nothing.

#[test]
fn play_boots_on_the_injected_folders_and_clicks_through_the_real_runtime() {
    let gui = play(
        literal(&[CARD_A, CARD_B], &[]),
        &[
            r#"expect "/Volumes/CARD_A""#,
            r#"expect "/Volumes/CARD_B""#,
            r#"click "/Volumes/CARD_B""#,
            r#"click "Look again""#,
            r#"expect "/Volumes/CARD_B""#,
            r#"click "Next""#,
        ],
    );
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(picked(&gui), [CARD_B]);
}

#[test]
#[should_panic(expected = "line failed")]
fn play_fails_on_words_that_are_not_there() {
    play(literal(&[CARD_A], &[]), &[r#"expect "/Volumes/NOT_THERE""#]);
}

#[test]
#[should_panic(expected = "line failed")]
fn play_cannot_click_words_that_are_not_there() {
    play(literal(&[], &[]), &[r#"click "/Volumes/CARD_A""#]);
}

#[test]
#[should_panic(expected = "NOT THERE")]
fn screen_check_fails_on_words_that_are_not_there() {
    screen_check(&two_cards(), "harness", "fails", &["NOT THERE"]);
}

#[test]
#[should_panic(expected = "out of sight")]
fn in_sight_fails_on_words_scrolled_out_of_sight() {
    let many: Vec<String> = (0..60).map(|n| format!("/Volumes/CARD_{n:02}")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    let gui = booted(literal(&many, &[]));
    in_sight(&gui, LEAST, &["/Volumes/CARD_59"]);
}

#[test]
fn screen_check_replaces_an_old_picture() {
    let first = picture_at(&two_cards(), WINDOW, "harness", "replaced");
    let first_bytes = fs::read(&first).unwrap();
    let second = picture_at(&booted(literal(&[], &[])), WINDOW, "harness", "replaced");
    assert_eq!(first, second);
    assert_ne!(first_bytes, fs::read(&second).unwrap());
}

// The arguments.

fn arguments(words: &[&str]) -> Result<Options, String> {
    parse(words.iter().map(OsString::from))
}

#[test]
fn arguments_are_the_two_hidden_flags_and_nothing_else() {
    assert_eq!(arguments(&[]), Ok(Options::default()));
    assert_eq!(
        arguments(&[
            "--search-root",
            "/a",
            "--search-root",
            "/b",
            "--smoke-png",
            "/p.png"
        ]),
        Ok(Options {
            search_roots: vec![PathBuf::from("/a"), PathBuf::from("/b")],
            smoke_png: Some(PathBuf::from("/p.png")),
        })
    );
    for refused in [
        &["--smoke"][..],
        &["x"],
        &["--search-root"],
        &["--smoke-png"],
    ] {
        assert!(arguments(refused).is_err(), "{refused:?}");
    }
}

// The PNG writer and the smoke run's end.

#[test]
fn the_checksums_are_the_known_ones() {
    assert_eq!(png::crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(png::crc32(b"IEND"), 0xAE42_6082);
    assert_eq!(png::adler32(b"Wikipedia"), 0x11E6_0398);
}

#[test]
fn a_picture_of_the_wrong_length_is_refused() {
    assert!(png::encode(2, 2, &[0; 15]).is_err());
}

#[test]
fn a_written_picture_reads_back_as_the_same_pixels_as_the_headless_one() {
    let program = gui::program(literal(&[CARD_A], &[]), None);
    let (events, mut heard) = emulator_channel::channel(1);
    let mut emulator = Emulator::new(events, &program, Mode::Zen, WINDOW);
    loop {
        match block_on(heard.next()).expect("the runtime stopped") {
            Event::Action(action) => emulator.perform(&program, action),
            Event::Failed(line) => panic!("{line}"),
            Event::Ready => break,
        }
    }
    let shot = emulator.screenshot(&program, &Theme::Dark, 2.0);
    assert!(
        shot.rgba.len() > 65535 * 2,
        "so it takes several stored blocks"
    );

    let dir = TempDir::new();
    let file = dir.0.join("shot-tiny-skia.png");
    png::write(&file, shot.size.width, shot.size.height, &shot.rgba).unwrap();
    let before = fs::read(&file).unwrap();

    let (state, _) = emulator.into_state();
    let mut ui = Simulator::with_size(Settings::default(), WINDOW, view(&state));
    let snapshot = ui.snapshot(&Theme::Dark).unwrap();
    assert!(snapshot.matches_image(dir.0.join("shot.png")).unwrap());
    assert_eq!(
        before,
        fs::read(&file).unwrap(),
        "it compared, and wrote nothing"
    );
}

#[test]
fn the_smoke_shot_writes_its_picture_and_ends_by_quits_message() {
    let dir = TempDir::new();
    let file = dir.0.join("smoke.png");
    let (mut gui, _) = Gui::boot(literal(&[], &[]), Some(file.clone()));
    settle(&mut gui);
    let shot = window::Screenshot::new(vec![9u8; 16], iced::Size::new(2u32, 2u32), 2.0);
    let _ = update(&mut gui, M::SmokeShot(shot));
    assert!(gui.quit);
    assert_eq!(
        fs::read(&file).unwrap(),
        png::encode(2, 2, &[9; 16]).unwrap()
    );
}
