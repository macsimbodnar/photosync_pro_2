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
use iced_test::selector::{self, Target};
use photosync_front::app::{Effect, Exit, Key, Msg, Outcome, Screen};

use crate::args::{Options, parse};
use crate::common::TempDir;
use crate::gui::{
    self, BANNER, Discover, Gui, LEAST, LOOKING, M, NOT_SAFE, TITLE, WINDOW, banner, update, view,
};
use crate::png;
use harness::*;

const CARD_A: &str = "/Volumes/CARD_A";
const CARD_B: &str = "/Volumes/CARD_B";
const NO_CARD: &str =
    "No camera card found. Insert a card, or connect the X-T5 in \"USB card reader\" mode.";
const DRIVE_A: &str = "/Volumes/DRIVE_A";
const DRIVE_B: &str = "/Volumes/DRIVE_B";
const NO_DRIVE: &str = "No drive to sync to found. Plug in your external drive.";
const CARDS_HEADING: &str = "1/5  Pick the cards to copy from";
const DRIVE_HEADING: &str = "2/5  Pick the drive to copy to";

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
    let mut gui = booted(literal(&[CARD_A], &[DRIVE_A]));
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Library);
    assert!(says(&gui, TITLE));
    assert!(
        !says(&gui, "Next"),
        "nothing can go on to a screen not built"
    );
}

// The cards and drive screens by clicks (S052).

fn key(gui: &mut Gui, pressed: Key) -> Option<Effect> {
    gui.app.update(Msg::Key(pressed))
}

/// A path of exactly 300 bytes, with nowhere to wrap but inside a word.
fn long_path(start: &str) -> String {
    let path = format!("{start}/{}", "a".repeat(300 - start.len() - 1));
    assert_eq!(path.len(), 300);
    path
}

fn on_the_drive_screen(drives: &[&str]) -> Gui {
    let mut gui = booted(literal(&[CARD_A], drives));
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    gui
}

/// `(translation.y, height, content's height)` of a scrollable.
fn scroll_of(ui: &mut iced_test::Simulator<'_, M>, id: &'static str) -> (f32, f32, f32) {
    match ui.find(selector::id(id)).unwrap() {
        Target::Scrollable {
            bounds,
            content_bounds,
            translation,
            ..
        } => (translation.y, bounds.height, content_bounds.height),
        other => panic!("not a scrollable: {other:?}"),
    }
}

#[test]
fn two_picked_cards_show_ticked_with_their_buttons() {
    let mut gui = two_cards();
    click(&mut gui, CARD_B);
    click(&mut gui, CARD_A);
    assert_eq!(picked(&gui), [CARD_A, CARD_B]);
    let words = [
        TITLE,
        CARDS_HEADING,
        CARD_A,
        CARD_B,
        "Next",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S052", "cards_two_picked", &words);
    in_sight(&gui, LEAST, &words);
}

#[test]
fn next_with_no_card_says_why_and_the_hint_names_a_button() {
    let mut gui = booted(literal(&[], &[DRIVE_A]));
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Sources);
    let hint = format!("{NO_CARD} Then click Look again.");
    in_sight(&gui, LEAST, &[NO_CARD, &hint, "Next", "Look again", "Quit"]);
    click(&mut gui, "Look again");
    settle(&mut gui);
    assert!(
        !says(&gui, &hint),
        "a new look clears why the last click did nothing"
    );
}

#[test]
fn look_again_keeps_what_is_still_there_and_unticks_what_went() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A, CARD_B], &[], &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, CARD_A);
    click(&mut gui, CARD_B);
    click(&mut gui, "Look again");
    next_look(&looks)
        .send(volumes(&[CARD_B], &[], &[]))
        .unwrap();
    settle(&mut gui);
    assert_eq!(sources(&gui), [CARD_B]);
    assert_eq!(picked(&gui), [CARD_B]);
    assert!(!says(&gui, CARD_A));
}

#[test]
fn the_looking_state_holds_on_the_cards_picture() {
    let (discover, looks) = blocked_discovery();
    let (gui, _) = Gui::boot(discover, None);
    next_look(&looks);
    let words = [TITLE, CARDS_HEADING, LOOKING, "Next", "Look again", "Quit"];
    screen_check(&gui, "S052", "cards_looking", &words);
    in_sight(&gui, LEAST, &words);
}

#[test]
fn the_looking_state_holds_on_the_drive_screen() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A], &[DRIVE_A], &["note"]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert!(says(&gui, DRIVE_A));
    let row = clicks(&gui, DRIVE_A);

    click(&mut gui, "Look again");
    let out = next_look(&looks);
    in_sight(
        &gui,
        LEAST,
        &[DRIVE_HEADING, LOOKING, "Next", "Back", "Look again", "Quit"],
    );
    assert!(!says(&gui, DRIVE_A), "a row of the older look");
    assert!(
        !says(&gui, "warning: cannot scan note: test"),
        "a note of the older look"
    );
    assert!(!says(&gui, NO_DRIVE), "nothing has said there is no drive");
    assert!(clicks(&gui, "Next").is_empty(), "Next is held");

    // Clicks drawn before the look began can still arrive.
    for message in row {
        let _ = update(&mut gui, message);
    }
    let _ = update(&mut gui, M::Next);
    assert_eq!(gui.app.screen(), Screen::Drive);

    // Look again stays, as a stuck mount never answers.
    click(&mut gui, "Look again");
    let newer = next_look(&looks);
    out.send(volumes(&[CARD_A], &[DRIVE_B], &[])).unwrap();
    wait_until(&mut gui, |gui| gui.dropped_looks == 1);
    assert!(says(&gui, LOOKING));
    assert!(!says(&gui, DRIVE_B));

    newer.send(volumes(&[CARD_A], &[DRIVE_A], &[])).unwrap();
    settle(&mut gui);
    assert!(says(&gui, DRIVE_A));
    assert_eq!(clicks(&gui, "Next").len(), 1, "Next is let go");
}

#[test]
fn back_stays_while_looking_and_the_cards_are_looking_too() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A], &[DRIVE_A], &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    click(&mut gui, "Look again");
    let out = next_look(&looks);
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert!(says(&gui, LOOKING));
    assert!(!says(&gui, CARD_A));
    out.send(volumes(&[CARD_A], &[DRIVE_A], &[])).unwrap();
    settle(&mut gui);
}

#[test]
fn the_drive_screen_shows_its_rows_and_buttons_and_marks_the_chosen_drive() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    let unmarked = picture_at(&gui, WINDOW, "harness", "drive_unmarked");
    let unmarked = fs::read(unmarked).unwrap();

    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_A));
    let _ = key(&mut gui, Key::Esc);
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(gui.app.cursor(), 0, "the cursor stands on the chosen drive");
    let marked = picture_at(&gui, WINDOW, "harness", "drive_marked");
    assert_ne!(
        unmarked,
        fs::read(marked).unwrap(),
        "only the chosen drive changed between these two"
    );

    // The mark stays on the chosen drive while the cursor shows what Next takes.
    click(&mut gui, DRIVE_B);
    assert_eq!(gui.app.cursor(), 1);
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_A));
    let words = [
        TITLE,
        DRIVE_HEADING,
        DRIVE_A,
        DRIVE_B,
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S052", "drive", &words);
    in_sight(&gui, LEAST, &words);

    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_B));
}

#[test]
fn a_click_on_a_drive_moves_the_cursor_and_ticks_no_card() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_B);
    assert_eq!(gui.app.cursor(), 1);
    assert_eq!(picked(&gui), [CARD_A]);
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert!(gui.app.drive().is_none(), "Next chooses it");
    let _ = update(&mut gui, M::Row(PathBuf::from("/Volumes/GONE")));
    assert_eq!(gui.app.cursor(), 1, "a row that has gone is no row");
}

#[test]
fn next_with_no_drive_says_why_and_stays() {
    let mut gui = on_the_drive_screen(&[]);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    let hint = format!("{NO_DRIVE} Then click Look again.");
    let words = [
        TITLE,
        DRIVE_HEADING,
        NO_DRIVE,
        hint.as_str(),
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S052", "drive_empty", &words);
    in_sight(&gui, LEAST, &words);
}

#[test]
fn back_returns_to_the_cards_with_the_cards_still_picked() {
    let mut gui = booted(literal(&[CARD_A, CARD_B], &[DRIVE_A]));
    click(&mut gui, CARD_B);
    click(&mut gui, "Next");
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert_eq!(picked(&gui), [CARD_B]);
    assert!(says(&gui, CARDS_HEADING));
    assert!(!gui.quit);
}

#[test]
fn a_drive_picked_again_keeps_an_edited_library_and_a_new_one_sets_photos() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, "Next");
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Photos");

    let _ = key(&mut gui, Key::Char('2'));
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Photos2");
    let _ = key(&mut gui, Key::Esc);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    assert_eq!(
        gui.app.library(),
        "/Volumes/DRIVE_A/Photos2",
        "the same drive"
    );

    let _ = key(&mut gui, Key::Esc);
    click(&mut gui, DRIVE_B);
    click(&mut gui, "Next");
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_B/Photos", "a new drive");
}

#[test]
fn clicks_drawn_on_the_screen_left_do_nothing_on_a_field() {
    let mut gui = on_the_drive_screen(&[DRIVE_A]);
    click(&mut gui, "Next");
    let library = gui.app.library().to_string();
    for message in [
        M::Row(PathBuf::from(CARD_A)),
        M::Row(PathBuf::from(DRIVE_A)),
        M::LookAgain,
        M::Quit,
        M::Back,
        M::Next,
    ] {
        let _ = update(&mut gui, message);
        assert_eq!(gui.app.screen(), Screen::Library);
        assert_eq!(gui.app.library(), library);
    }
    assert!(!gui.quit);
}

#[test]
fn a_second_back_drawn_on_the_drive_screen_does_not_quit_from_the_cards() {
    let mut gui = on_the_drive_screen(&[DRIVE_A]);
    let back = clicks(&gui, "Back");
    assert_eq!(back.len(), 1);
    click(&mut gui, "Back");
    for message in back {
        let _ = update(&mut gui, message);
    }
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert!(!gui.quit, "Esc on the cards quits");
}

#[test]
fn long_paths_and_a_wide_label_show_whole() {
    let long_card = long_path("/Volumes/CARD");
    let long_drive = long_path("/Volumes/DRIVE");
    let wide = "/Volumes/写真カード";
    let mut gui = booted(literal(&[&long_card, wide], &[&long_drive, wide]));
    let words = [TITLE, long_card.as_str(), wide, "Next"];
    screen_check(&gui, "S052", "cards_long_paths", &words);
    in_sight(&gui, LEAST, &words);
    let mut ui = screen(&gui, LEAST);
    let found = ui.find(long_card.as_str()).unwrap();
    assert!(found.bounds().height > 30.0, "it wrapped: {found:?}");
    assert!(found.bounds().width <= LEAST.width, "{found:?}");
    drop(ui);

    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    let words = [TITLE, long_drive.as_str(), wide, "Next"];
    screen_check(&gui, "S052", "drive_long_paths", &words);
    in_sight(&gui, LEAST, &words);
    let mut ui = screen(&gui, LEAST);
    let found = ui.find(long_drive.as_str()).unwrap();
    assert!(found.bounds().height > 30.0, "it wrapped: {found:?}");
}

fn hundred_notes() -> Vec<String> {
    (0..100).map(|n| format!("/Volumes/BAD_{n:02}")).collect()
}

fn with_notes(sources: &'static [&'static str], destinations: &'static [&'static str]) -> Discover {
    Arc::new(move || {
        let notes = hundred_notes();
        let notes: Vec<&str> = notes.iter().map(String::as_str).collect();
        volumes(sources, destinations, &notes)
    })
}

#[test]
fn many_notes_scroll_and_leave_the_rows_in_sight() {
    let mut gui = booted(with_notes(&[CARD_A, CARD_B], &[DRIVE_A]));
    let last = "warning: cannot scan /Volumes/BAD_99: test";
    let words = [
        TITLE,
        CARDS_HEADING,
        CARD_A,
        CARD_B,
        "Next",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S052", "cards_notes", &words);
    in_sight(&gui, LEAST, &words);
    assert!(says(&gui, last), "found, though out of sight");

    click(&mut gui, "Next");
    in_sight(&gui, LEAST, &[DRIVE_HEADING, DRIVE_A, "Next", "Back"]);
    assert!(says(&gui, last));
}

#[test]
fn many_notes_leave_an_empty_lists_sentence_in_sight() {
    let gui = booted(with_notes(&[], &[]));
    in_sight(&gui, LEAST, &[NO_CARD, "Next"]);
}

#[test]
fn the_end_of_many_notes_can_be_reached() {
    play_steps(
        with_notes(&[CARD_A], &[DRIVE_A]),
        vec![
            Step::Fails(r#"click "warning: cannot scan /Volumes/BAD_99: test""#),
            Step::ScrollEnd,
            Step::Line(r#"click "warning: cannot scan /Volumes/BAD_99: test""#),
        ],
    );
}

// No key is named in any words shown (DEC-068 (3)).

fn no_key_named(gui: &Gui, what: &str) {
    let seen = all_words(gui, WINDOW);
    assert!(
        seen.iter().any(|w| w == TITLE),
        "{what}: not vacuous: {seen:?}"
    );
    for words in seen {
        // It names a button, not a key (DEC-074 (6)).
        if words == LOOKING {
            continue;
        }
        for name in ["Esc", "Space", "press", "Ctrl-C"] {
            assert!(!words.contains(name), "{what}: {name} in {words:?}");
        }
    }
}

#[test]
fn no_words_on_these_screens_name_a_key() {
    let mut gui = two_cards();
    no_key_named(&gui, "cards");
    click(&mut gui, CARD_A);
    no_key_named(&gui, "a card ticked");

    let mut gui = booted(literal(&[], &[]));
    no_key_named(&gui, "no card");
    click(&mut gui, "Next");
    assert!(gui.app.hint().is_some());
    no_key_named(&gui, "the hint of no card");

    let mut gui = on_the_drive_screen(&[DRIVE_A]);
    no_key_named(&gui, "drive");
    click(&mut gui, DRIVE_A);
    no_key_named(&gui, "drive, a row clicked");

    let mut gui = on_the_drive_screen(&[]);
    no_key_named(&gui, "no drive");
    click(&mut gui, "Next");
    assert!(gui.app.hint().is_some());
    no_key_named(&gui, "the hint of no drive");

    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    no_key_named(&gui, "looking, cards");
    next_look(&looks)
        .send(volumes(&[CARD_A], &[DRIVE_A], &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    click(&mut gui, "Look again");
    let out = next_look(&looks);
    no_key_named(&gui, "looking, drive");
    out.send(volumes(&[CARD_A], &[DRIVE_A], &[])).unwrap();
    settle(&mut gui);

    let dir = TempDir::new();
    let mut gui = refused_and_walked_back(&dir);
    no_key_named(&gui, "banner, drive");
    let _ = key(&mut gui, Key::Esc);
    no_key_named(&gui, "banner, cards");

    let (mut gui, _dir) = forty_named_syncs();
    no_key_named(&gui, "banner of 40 syncs");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);
}

// Through the real runtime.

#[test]
fn play_clicks_from_the_cards_through_the_drive_screen() {
    let gui = play(
        literal(&[CARD_A, CARD_B], &[DRIVE_A, DRIVE_B]),
        &[
            r#"click "/Volumes/CARD_A""#,
            r#"click "/Volumes/CARD_B""#,
            r#"click "Next""#,
            r#"expect "2/5  Pick the drive to copy to""#,
            r#"click "/Volumes/DRIVE_B""#,
            r#"click "Look again""#,
            r#"click "/Volumes/DRIVE_B""#,
            r#"click "Back""#,
            r#"expect "1/5  Pick the cards to copy from""#,
            r#"click "Next""#,
            r#"click "/Volumes/DRIVE_B""#,
            r#"click "Next""#,
        ],
    );
    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(picked(&gui), [CARD_A, CARD_B]);
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_B/Photos");
    assert!(says(&gui, TITLE));
    assert!(!says(&gui, "Next"), "a bare title until S053");
}

// The scroll opens at the top (DEC-073 (1)).

#[test]
fn the_next_screen_opens_at_its_top() {
    let first_note = r#"click "warning: cannot scan /Volumes/BAD_00: test""#;
    let gui = play_steps(
        with_notes(&[CARD_A], &[DRIVE_A]),
        vec![
            Step::Line(first_note),
            Step::ScrollEnd,
            // It scrolled: what passed above is out of sight.
            Step::Fails(first_note),
            Step::Line(r#"click "Next""#),
            Step::Line(r#"click "/Volumes/DRIVE_A""#),
            Step::ScrollEnd,
            Step::Fails(r#"click "/Volumes/DRIVE_A""#),
            Step::Line(r#"click "Back""#),
            Step::Line(r#"click "/Volumes/CARD_A""#),
        ],
    );
    assert_eq!(gui.app.screen(), Screen::Sources);
}

#[test]
fn a_new_look_opens_at_its_top() {
    play_steps(
        with_notes(&[CARD_A], &[DRIVE_A]),
        vec![
            Step::ScrollEnd,
            Step::Fails(r#"click "/Volumes/CARD_A""#),
            Step::Line(r#"click "Look again""#),
            Step::Line(r#"click "/Volumes/CARD_A""#),
        ],
    );
}

// The banner (DEC-071 (2), (3)).

/// A library that is not there, refused at the check and walked back to the drive
/// screen, by the model's keys, as the screens between are S053's.
fn refused_and_walked_back(dir: &TempDir) -> Gui {
    let card = dir.0.join("CARD").display().to_string();
    let drive = dir.0.join("DRIVE").display().to_string();
    fs::create_dir_all(&card).unwrap();
    fs::create_dir_all(&drive).unwrap();
    let mut gui = booted(literal(&[&card], &[&drive]));
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    for pressed in [Key::Enter, Key::Enter, Key::Enter] {
        assert!(key(&mut gui, pressed).is_none());
    }
    assert_eq!(gui.app.screen(), Screen::Refused);
    assert!(!gui.app.in_session());
    assert_eq!(gui.app.exit().code, 2);
    for pressed in [Key::Enter, Key::Esc, Key::Esc, Key::Esc] {
        assert!(key(&mut gui, pressed).is_none());
    }
    assert_eq!(gui.app.screen(), Screen::Drive);
    gui
}

#[test]
fn with_code_0_there_is_no_banner() {
    let dir = TempDir::new();
    let drive = dir.0.join("DRIVE").display().to_string();
    let mut gui = booted(literal(&[CARD_A], &[&drive]));
    assert_eq!(gui.app.exit().code, 0);
    assert!(!says(&gui, NOT_SAFE));
    click(&mut gui, "Next");
    assert!(!says(&gui, NOT_SAFE));
}

#[test]
fn the_banner_stays_in_sight_on_both_screens_after_a_refusal() {
    let dir = TempDir::new();
    let mut gui = refused_and_walked_back(&dir);
    let message = gui.app.exit().message.unwrap();
    let drive = dir.0.join("DRIVE").display().to_string();
    in_sight(&gui, LEAST, &[NOT_SAFE, &message, &drive, "Next", "Back"]);

    let _ = key(&mut gui, Key::Esc);
    assert_eq!(gui.app.screen(), Screen::Sources);
    let card = dir.0.join("CARD").display().to_string();
    let words = [NOT_SAFE, message.as_str(), card.as_str(), "Next"];
    in_sight(&gui, LEAST, &words);
    screen_check(&gui, "S052", "cards_banner", &words);
}

#[test]
fn a_long_library_in_the_refusal_leaves_the_drive_screens_buttons() {
    let dir = TempDir::new();
    let card = dir.0.join("CARD").display().to_string();
    let drive = dir.0.join("DRIVE").display().to_string();
    fs::create_dir_all(&card).unwrap();
    fs::create_dir_all(&drive).unwrap();
    let mut gui = booted(literal(&[&card], &[&drive]));
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    for c in "/not_there".repeat(100).chars() {
        assert!(key(&mut gui, Key::Char(c)).is_none());
    }
    for pressed in [Key::Enter, Key::Enter, Key::Enter] {
        assert!(key(&mut gui, pressed).is_none());
    }
    assert_eq!(gui.app.exit().code, 2);
    for pressed in [Key::Enter, Key::Esc, Key::Esc, Key::Esc] {
        assert!(key(&mut gui, pressed).is_none());
    }
    assert_eq!(gui.app.screen(), Screen::Drive);
    let message = gui.app.exit().message.unwrap();
    in_sight(&gui, LEAST, &[NOT_SAFE, &message, &drive, "Next", "Back"]);

    let mut ui = screen(&gui, LEAST);
    let (_, height, content) = scroll_of(&mut ui, BANNER);
    assert!(height <= 104.0, "{height}");
    assert!(content > height, "{content} in {height}");
}

/// A real session of 40 syncs, each ended by the crash message, left on its next card's
/// list. So the result names every one, as no sync has to run for it. No copy is made.
fn forty_named_syncs() -> (Gui, TempDir) {
    let dir = TempDir::new();
    let drive = dir.0.join("DRIVE");
    fs::create_dir_all(drive.join("Photos")).unwrap();
    let drive = drive.display().to_string();
    let mut gui = booted(literal(&[CARD_A, CARD_B], &[&drive]));
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert!(key(&mut gui, Key::Enter).is_none());
    assert!(key(&mut gui, Key::Enter).is_none());
    for sync in 0..40 {
        assert_eq!(gui.app.screen(), Screen::Confirm, "sync {sync}");
        match key(&mut gui, Key::Enter) {
            Some(Effect::Start(_job)) => {}
            other => panic!("no start: {other:?}"),
        }
        assert!(
            gui.app
                .update(Msg::Crashed(format!("test crash {sync}")))
                .is_none()
        );
        assert_eq!(gui.app.screen(), Screen::Refused);
        assert!(matches!(
            key(&mut gui, Key::Char('n')),
            Some(Effect::Discover)
        ));
        assert!(key(&mut gui, Key::Char(' ')).is_none());
        assert!(key(&mut gui, Key::Enter).is_none());
    }
    assert!(key(&mut gui, Key::Esc).is_none());
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert_eq!(gui.app.syncs(), 40);
    assert_eq!(gui.app.exit().code, 1);
    (gui, dir)
}

#[test]
fn forty_named_syncs_leave_the_rows_and_next_in_sight_and_the_wheel_reaches_the_box_end() {
    let (gui, _dir) = forty_named_syncs();
    let message = gui.app.exit().message.unwrap();
    assert!(message.contains(CARD_A), "{message}");
    let heading = "Sync 41: pick the next card";
    let words = [
        TITLE,
        heading,
        NOT_SAFE,
        message.as_str(),
        CARD_A,
        CARD_B,
        "Next",
        "Look again",
        "Quit",
    ];
    in_sight(&gui, LEAST, &words);
    screen_check(&gui, "S052", "cards_banner_long", &words);
    assert!(!says(&gui, "Back"), "no back on the cards in a session");
    picture_at(&gui, LEAST, "S052", "cards_banner_long_least");

    let mut ui = screen(&gui, LEAST);
    let (start, height, content) = scroll_of(&mut ui, BANNER);
    assert_eq!(start, 0.0, "it opens at its top");
    assert!(height <= 104.0, "{height}");
    assert!(content > height * 2.0, "{content} in {height}");

    let inside = ui
        .find(selector::id(BANNER))
        .unwrap()
        .visible_bounds()
        .unwrap()
        .center();
    ui.point_at(inside);
    let wheel = iced_test::core::Event::Mouse(iced_test::core::mouse::Event::WheelScrolled {
        delta: iced_test::core::mouse::ScrollDelta::Lines { x: 0.0, y: -1000.0 },
    });
    let _ = ui.simulate([wheel]);
    let (end, height, content) = scroll_of(&mut ui, BANNER);
    // iced rounds a translation to a whole pixel (DEC-075 (2)).
    assert!(
        end + height >= content.floor(),
        "{end} + {height} of {content}"
    );
    for sentence in [NOT_SAFE, CARD_A, "Next"] {
        assert!(
            ui.find(sentence).unwrap().visible_bounds().is_some(),
            "{sentence}"
        );
    }
}

#[test]
fn the_banner_and_a_hint_together_leave_the_empty_sentence_and_next() {
    let (mut gui, _dir) = forty_named_syncs();
    assert!(
        gui.app
            .update(Msg::Volumes(volumes(&[], &[], &[])))
            .is_none()
    );
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Sources);
    let hint = format!("{NO_CARD} Then click Look again.");
    in_sight(
        &gui,
        LEAST,
        &[NOT_SAFE, NO_CARD, &hint, "Next", "Look again", "Quit"],
    );
}

#[test]
fn banner_is_the_message_or_else_the_outcomes_own_words() {
    let exit = |code, message: Option<&str>| Exit {
        code,
        report: None,
        message: message.map(String::from),
    };
    let report = Outcome::Report {
        lines: vec![],
        result: "The result.".to_string(),
        complete: false,
    };
    let refused = Outcome::Refused {
        sentence: "The refusal.".to_string(),
    };
    assert_eq!(banner(&exit(0, None), None), None);
    assert_eq!(banner(&exit(0, Some("x")), Some(&report)), None);
    assert_eq!(
        banner(&exit(1, None), Some(&report)).as_deref(),
        Some("The result."),
        "one failed sync has no message (DEC-071 (2))"
    );
    assert_eq!(
        banner(&exit(1, Some("The message.")), Some(&report)).as_deref(),
        Some("The message.")
    );
    assert_eq!(
        banner(&exit(2, None), Some(&refused)).as_deref(),
        Some("The refusal.")
    );
    assert_eq!(banner(&exit(1, None), None).as_deref(), Some(""));
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
