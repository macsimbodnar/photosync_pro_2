//! All the GUI's unit tests, in one test binary (DEC-072 (1)).

mod harness;
#[path = "../../tests/support/one_picture.rs"]
mod one_picture;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use iced::widget::{Space, button, column, row, scrollable, text_input};
use iced::{Color, Element, Length, Size, Theme, window};
use iced_test::Simulator;
use iced_test::core::Settings;
use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::channel::mpsc::{self as emulator_channel, TryRecvError};
use iced_test::selector::{self, Target};
use photosync_front::app::{Effect, Exit, Key, Msg, Outcome, Screen};
use photosync_front::{flow, text};

use crate::args::{Options, parse};
use crate::common::TempDir;
use crate::gui::{
    self, BANNER, BODY, CARDS_ONLY_READ, DRIVE_GONE, Discover, Gui, LEAST, LIBRARY_HELP, LOOKING,
    M, NAME_HELP, NOT_SAFE, PICK_A_DRIVE, THE_DATE_TO_COME, TITLE, WINDOW, banner, update, view,
};
use crate::png;
use crate::widgets::{row_under_cursor, words};
use harness::*;
use one_picture::same_picture;

const CARD_A: &str = "/Volumes/CARD_A";
const CARD_B: &str = "/Volumes/CARD_B";
const NO_CARD: &str =
    "No camera card found. Insert a card, or connect the X-T5 in \"USB card reader\" mode.";
const DRIVE_A: &str = "/Volumes/DRIVE_A";
const DRIVE_B: &str = "/Volumes/DRIVE_B";
const NO_DRIVE: &str = "No drive to sync to found. Plug in your external drive.";
const CARDS_HEADING: &str = "1/5  Pick the cards to copy from";
const DRIVE_HEADING: &str = "2/5  Pick the drive to copy to";
const CARDS_BUTTONS: [&str; 3] = ["Next", "Look again", "Quit"];
const DRIVE_BUTTONS: [&str; 4] = ["Next", "Back", "Look again", "Quit"];

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
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
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
    fixed_in_sight(&gui, LEAST, &[TITLE]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
}

#[test]
fn an_empty_list_says_so_and_next_stays() {
    let mut gui = booted(literal(&[], &[]));
    let words = [TITLE, NO_CARD, "Next", "Look again", "Quit"];
    screen_check(&gui, "S051", "cards_empty", &words);
    in_sight(&gui, LEAST, &words);
    fixed_in_sight(&gui, LEAST, &[TITLE]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
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
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
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
    let folders = Folders::new();
    fs::create_dir_all(&folders.library).unwrap();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");
    assert_eq!(gui.app.screen(), Screen::Running);
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
    fixed_in_sight(&gui, LEAST, &[TITLE]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
}

#[test]
fn next_with_no_card_says_why_and_the_hint_names_a_button() {
    let mut gui = booted(literal(&[], &[DRIVE_A]));
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Sources);
    let hint = format!("{NO_CARD} Then click Look again.");
    in_sight(&gui, LEAST, &[NO_CARD, &hint, "Next", "Look again", "Quit"]);
    fixed_in_sight(&gui, LEAST, &[&hint]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
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
    fixed_in_sight(&gui, LEAST, &[TITLE, CARDS_HEADING]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
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
    fixed_in_sight(&gui, LEAST, &[DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
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

    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_A));
    let _ = key(&mut gui, Key::Esc);
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(gui.app.cursor(), 0, "the cursor stands on the chosen drive");
    let marked = picture_at(&gui, WINDOW, "harness", "drive_marked");
    assert_ne!(
        unmarked,
        fs::read(marked).unwrap(),
        "only the mark changed between these two"
    );

    // The mark moves at once. The model hears of the drive on Next, as its library goes with it.
    click(&mut gui, DRIVE_B);
    assert_eq!(gui.app.cursor(), 1);
    assert_eq!(mark_of(&gui), Some(DRIVE_B));
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
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);

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

// The drive's mark is what Next takes (S063, DEC-077).

const AA_FIRST: &str = "/Volumes/AA_FIRST";

fn mark_of(gui: &Gui) -> Option<&str> {
    gui.marked_drive().and_then(|path| path.to_str())
}

/// On the drive screen of a held discovery, and the first look answered.
fn held_on_the_drive_screen(
    drives: &[&str],
) -> (Gui, mpsc::Receiver<mpsc::Sender<photosync_core::Volumes>>) {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A], drives, &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    (gui, looks)
}

/// Look again, answered with these drives.
fn look_gives(
    gui: &mut Gui,
    looks: &mpsc::Receiver<mpsc::Sender<photosync_core::Volumes>>,
    drives: &[&str],
) {
    click(gui, "Look again");
    next_look(looks)
        .send(volumes(&[CARD_A], drives, &[]))
        .unwrap();
    settle(gui);
}

/// The picture of this state, to compare.
fn picture(gui: &Gui, name: &str) -> Vec<u8> {
    fs::read(picture_at(gui, WINDOW, "harness", name)).unwrap()
}

#[test]
fn choose_a_go_back_click_b_and_the_one_mark_moves_at_once_and_next_takes_b() {
    let mut fresh = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut fresh, DRIVE_B);
    let only_b = picture(&fresh, "s063_only_b");

    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_A);
    let only_a = picture(&gui, "s063_only_a");
    assert_ne!(only_a, only_b);
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_A));
    let _ = key(&mut gui, Key::Esc);
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(mark_of(&gui), Some(DRIVE_A));
    assert_eq!(picture(&gui, "s063_back_on_a"), only_a);

    click(&mut gui, DRIVE_B);
    assert_eq!(mark_of(&gui), Some(DRIVE_B), "at once, with no Next");
    assert_eq!(
        picture(&gui, "s063_a_then_b"),
        only_b,
        "nothing is left on A: not a tick, not a bar"
    );
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
    screen_check(&gui, "S063", "drive_marked", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);

    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_B));
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_B/Photos");
}

#[test]
fn with_no_click_next_takes_the_marked_drive_and_keeps_an_edited_library() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_B);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Char('2'));
    let _ = key(&mut gui, Key::Esc);
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_B));
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_B/Photos2");
}

#[test]
fn clicks_there_and_back_are_no_change_of_drive() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Char('2'));
    let _ = key(&mut gui, Key::Esc);
    click(&mut gui, DRIVE_B);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Photos2");
}

#[test]
fn a_new_drive_sorted_first_leaves_the_chosen_mark_and_next_takes_it() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_B);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Esc);
    look_gives(&mut gui, &looks, &[AA_FIRST, DRIVE_A, DRIVE_B]);
    assert_eq!(mark_of(&gui), Some(DRIVE_B));
    assert_eq!(gui.app.cursor(), 2);
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_B));
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_B/Photos");
}

#[test]
fn a_new_drive_sorted_first_leaves_a_clicked_mark_and_next_takes_it() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_B);
    look_gives(&mut gui, &looks, &[AA_FIRST, DRIVE_A, DRIVE_B]);
    assert_eq!(mark_of(&gui), Some(DRIVE_B));
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_B));
}

#[test]
fn a_click_then_back_and_next_keeps_the_mark() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Esc);
    click(&mut gui, DRIVE_B);
    click(&mut gui, "Back");
    click(&mut gui, "Next");
    assert_eq!(mark_of(&gui), Some(DRIVE_B), "as a radio button keeps it");
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_B));
}

#[test]
fn a_chosen_drive_gone_has_no_mark_and_next_is_held_with_a_hint() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    let none_marked = picture(&gui, "s063_none_marked");
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Esc);
    look_gives(&mut gui, &looks, &[DRIVE_B]);

    // One drive is left, and it still does not stand in for the one chosen.
    assert_eq!(mark_of(&gui), None);
    assert_eq!(gui.app.drive(), None);
    assert!(clicks(&gui, "Next").is_empty(), "Next is held");
    let _ = update(&mut gui, M::Next);
    assert_eq!(gui.app.screen(), Screen::Drive, "a click drawn before");
    let words = [
        TITLE,
        DRIVE_HEADING,
        DRIVE_B,
        DRIVE_GONE,
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S063", "drive_gone", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING, DRIVE_GONE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);

    // Plugged in again it is a row like any other, to be clicked: the hint no longer says gone.
    look_gives(&mut gui, &looks, &[DRIVE_A, DRIVE_B]);
    assert_eq!(mark_of(&gui), None);
    assert!(!says(&gui, DRIVE_GONE));
    assert!(says(&gui, PICK_A_DRIVE));
    assert_eq!(
        picture(&gui, "s063_gone_and_back"),
        none_marked,
        "as a first visit"
    );
    look_gives(&mut gui, &looks, &[DRIVE_A]);
    assert_eq!(
        mark_of(&gui),
        None,
        "the one left is the one that went, not yet picked again"
    );
    assert!(clicks(&gui, "Next").is_empty());

    click(&mut gui, DRIVE_A);
    assert!(!says(&gui, PICK_A_DRIVE));
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_A));
}

#[test]
fn a_clicked_drive_gone_before_next_has_no_mark() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_B);
    look_gives(&mut gui, &looks, &[DRIVE_A]);
    assert_eq!(mark_of(&gui), None);
    assert!(says(&gui, DRIVE_GONE));
    assert!(clicks(&gui, "Next").is_empty());
}

#[test]
fn a_chosen_drive_gone_while_on_the_cards_has_no_mark_on_return() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Esc);
    click(&mut gui, "Back");
    look_gives(&mut gui, &looks, &[DRIVE_B]);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(mark_of(&gui), None);
    assert!(says(&gui, DRIVE_GONE));
    assert!(clicks(&gui, "Next").is_empty());
}

#[test]
fn every_drive_gone_has_the_empty_lists_words_and_next_stays() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A]);
    click(&mut gui, "Next");
    let _ = key(&mut gui, Key::Esc);
    look_gives(&mut gui, &looks, &[]);
    assert_eq!(mark_of(&gui), None);
    assert!(!says(&gui, DRIVE_GONE), "there is no other to click");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Drive);
    let hint = format!("{NO_DRIVE} Then click Look again.");
    let words = [
        NO_DRIVE,
        hint.as_str(),
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S063", "drive_all_gone", &words);
    fixed_in_sight(&gui, LEAST, &[&hint]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    look_gives(&mut gui, &looks, &[DRIVE_B]);
    assert_eq!(mark_of(&gui), None, "not the one that went");
    assert!(says(&gui, DRIVE_GONE));
}

#[test]
fn a_look_holds_the_mark_and_next_until_it_answers() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_B);
    click(&mut gui, "Look again");
    let out = next_look(&looks);
    assert!(clicks(&gui, "Next").is_empty());
    let _ = update(&mut gui, M::Next);
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert!(
        !says(&gui, PICK_A_DRIVE),
        "the list is not shown to pick from"
    );
    let words = [
        TITLE,
        DRIVE_HEADING,
        LOOKING,
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S063", "drive_looking", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    out.send(volumes(&[CARD_A], &[DRIVE_A, DRIVE_B], &[]))
        .unwrap();
    settle(&mut gui);
    assert_eq!(mark_of(&gui), Some(DRIVE_B));
}

#[test]
fn a_double_click_on_the_cards_next_takes_no_drive_unseen() {
    let mut gui = booted(literal(&[CARD_A], &[DRIVE_A, DRIVE_B]));
    let next = clicks(&gui, "Next");
    click(&mut gui, "Next");
    // The second click of the double, drawn on the cards, lands here.
    for message in next {
        let _ = update(&mut gui, message);
    }
    assert_eq!(gui.app.screen(), Screen::Drive);
    assert_eq!(gui.app.drive(), None);
    assert_eq!(mark_of(&gui), None);
    assert!(clicks(&gui, "Next").is_empty(), "Next is held");
    let words = [
        TITLE,
        DRIVE_HEADING,
        DRIVE_A,
        DRIVE_B,
        PICK_A_DRIVE,
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S063", "drive_first_visit", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING, PICK_A_DRIVE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);

    click(&mut gui, DRIVE_B);
    assert_eq!(clicks(&gui, "Next").len(), 1);
    assert!(!says(&gui, PICK_A_DRIVE));
}

#[test]
fn the_only_drive_is_marked_and_next_takes_it() {
    let mut gui = on_the_drive_screen(&[DRIVE_A]);
    assert_eq!(mark_of(&gui), Some(DRIVE_A));
    assert!(!says(&gui, PICK_A_DRIVE));
    let words = [
        TITLE,
        DRIVE_HEADING,
        DRIVE_A,
        "Next",
        "Back",
        "Look again",
        "Quit",
    ];
    screen_check(&gui, "S063", "drive_one", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    click(&mut gui, "Next");
    assert_eq!(gui.app.drive().unwrap().to_str(), Some(DRIVE_A));
}

#[test]
fn a_second_drive_appearing_takes_the_offered_mark_away() {
    let (mut gui, looks) = held_on_the_drive_screen(&[DRIVE_B]);
    assert_eq!(mark_of(&gui), Some(DRIVE_B));
    look_gives(&mut gui, &looks, &[DRIVE_A, DRIVE_B]);
    assert_eq!(mark_of(&gui), None, "nobody picked it");
    assert!(clicks(&gui, "Next").is_empty());
}

#[test]
fn the_gone_hint_and_a_long_banner_leave_the_rows_and_next_at_the_least_size() {
    let dir = TempDir::new();
    let card = dir.0.join("CARD").display().to_string();
    let drive = dir.0.join("DRIVE").display().to_string();
    let other = dir.0.join("OTHER").display().to_string();
    fs::create_dir_all(&card).unwrap();
    fs::create_dir_all(&drive).unwrap();
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[&card], &[&drive], &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    for c in "/not_there".repeat(100).chars() {
        assert!(key(&mut gui, Key::Char(c)).is_none());
    }
    for pressed in [
        Key::Enter,
        Key::Enter,
        Key::Enter,
        Key::Enter,
        Key::Esc,
        Key::Esc,
        Key::Esc,
    ] {
        assert!(key(&mut gui, pressed).is_none());
    }
    assert_eq!(gui.app.screen(), Screen::Drive);
    click(&mut gui, "Look again");
    next_look(&looks)
        .send(volumes(&[&card], &[&other], &[]))
        .unwrap();
    settle(&mut gui);
    let message = gui.app.exit().message.unwrap();
    let words = [
        NOT_SAFE,
        message.as_str(),
        other.as_str(),
        DRIVE_GONE,
        "Next",
        "Back",
    ];
    screen_check(&gui, "S063", "drive_gone_banner", &words);
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE, DRIVE_GONE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
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
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
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
    click(&mut gui, DRIVE_A);
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
    for on in [Screen::Library, Screen::Name] {
        let _ = update(&mut gui, M::Field("typed".to_string()));
        for message in [
            M::Row(PathBuf::from(CARD_A)),
            M::Row(PathBuf::from(DRIVE_A)),
            M::LookAgain,
            M::Quit,
            M::Start,
        ] {
            let _ = update(&mut gui, message);
            assert_eq!(gui.app.screen(), on);
            assert_eq!(field_of(&gui), "typed");
        }
        assert!(!gui.quit);
        // Next and Back are this screen's own now, and they type nothing either.
        let _ = update(&mut gui, M::Next);
        let _ = update(&mut gui, M::Back);
        assert_eq!(gui.app.screen(), on);
        assert_eq!(field_of(&gui), "typed");
        let _ = update(&mut gui, M::Next);
    }
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
    fixed_in_sight(&gui, LEAST, &[TITLE]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
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
    fixed_in_sight(&gui, LEAST, &[TITLE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    let mut ui = screen(&gui, LEAST);
    let found = ui.find(long_drive.as_str()).unwrap();
    assert!(found.bounds().height > 30.0, "it wrapped: {found:?}");
    drop(ui);

    click(&mut gui, long_drive.as_str());
    let words = [TITLE, long_drive.as_str(), wide, "Next"];
    screen_check(&gui, "S063", "drive_long_path_marked", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
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
    screen_check_scrolling(&gui, "S052", "cards_notes", &words, &[last]);
    in_sight(&gui, LEAST, &words);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
    assert!(says(&gui, last), "found, though out of sight");

    click(&mut gui, "Next");
    in_sight(&gui, LEAST, &[DRIVE_HEADING, DRIVE_A, "Next", "Back"]);
    fixed_in_sight(&gui, LEAST, &[DRIVE_HEADING]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    assert!(says(&gui, last));
}

#[test]
fn many_notes_leave_an_empty_lists_sentence_in_sight() {
    let gui = booted(with_notes(&[], &[]));
    in_sight(&gui, LEAST, &[NO_CARD, "Next"]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
}

#[test]
fn the_end_of_many_notes_can_be_reached() {
    // A note is no button: its click sends nothing, and passes only if it is in sight.
    let gui = play_steps(
        with_notes(&[CARD_A], &[DRIVE_A]),
        vec![
            Step::Fails(r#"click "warning: cannot scan /Volumes/BAD_99: test""#),
            Step::ScrollEnd,
            Step::Line(r#"click "warning: cannot scan /Volumes/BAD_99: test""#),
        ],
    );
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert!(gui.app.picked().is_empty());
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
    no_key_named(&gui, "the check, in a session");

    let folders = Folders::new();
    let mut gui = on_the_library_screen(&folders);
    no_key_named(&gui, "library");
    let _ = update(&mut gui, M::Field(String::new()));
    click(&mut gui, "Next");
    assert!(gui.app.hint().is_some());
    no_key_named(&gui, "the hint of no library");
    let _ = update(&mut gui, M::Field(folders.library.clone()));
    click(&mut gui, "Next");
    no_key_named(&gui, "name");
    click(&mut gui, "Next");
    no_key_named(&gui, "check");
    click(&mut gui, "Start copying");
    assert_eq!(gui.app.screen(), Screen::Refused);
    no_key_named(&gui, "refused");
    click(&mut gui, REFUSED_BACK);
    no_key_named(&gui, "check, banner");
    click(&mut gui, "Back");
    no_key_named(&gui, "name, banner");
    click(&mut gui, "Back");
    no_key_named(&gui, "library, banner");
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
    assert!(says(&gui, LIBRARY_HEADING));
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
    // Next took the row with none ticked, and the last click, which landed, unticked it.
    assert!(gui.app.picked().is_empty());
    assert_eq!(gui.app.cursor(), 0);
}

#[test]
fn a_new_look_opens_at_its_top() {
    let gui = play_steps(
        with_notes(&[CARD_A], &[DRIVE_A]),
        vec![
            Step::ScrollEnd,
            Step::Fails(r#"click "/Volumes/CARD_A""#),
            Step::Line(r#"click "Look again""#),
            Step::Line(r#"click "/Volumes/CARD_A""#),
        ],
    );
    assert_eq!(picked(&gui), [CARD_A], "the click reached the row");
}

// The banner (DEC-071 (2), (3)).

/// A library that is not there, refused at the check and walked back to the drive
/// screen, by the model's keys.
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
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);

    let _ = key(&mut gui, Key::Esc);
    assert_eq!(gui.app.screen(), Screen::Sources);
    let card = dir.0.join("CARD").display().to_string();
    let words = [NOT_SAFE, message.as_str(), card.as_str(), "Next"];
    in_sight(&gui, LEAST, &words);
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
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
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &DRIVE_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);

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
    fixed_in_sight(&gui, LEAST, &[TITLE, heading, NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    assert!(!says(&gui, "Back"), "no back on the cards in a session");

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
    see_fixed_in_sight(&mut ui, LEAST, &[NOT_SAFE, "Next"]).unwrap();
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
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE, &hint]);
    fixed_in_sight(&gui, LEAST, &CARDS_BUTTONS);
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

// The checks see what they say they see (S062).

const FIXED: &str = "A fixed sentence.";
const TWO_LINES: &str = "A warning of two lines,\nits second line can be lost.";
const IN_THE_BODY: &str = "A sentence in the body.";
const UNDER_THE_BODY: &str = "Next";

fn simulator<'a>(size: Size, view: impl Into<Element<'a, M>>) -> Simulator<'a, M> {
    Simulator::with_size(Settings::default(), size, view)
}

/// A fixed part `height` high, then `sentence`, the body that takes what is left, and a
/// sentence under it, as the real screens' are.
fn a_screen(height: f32, sentence: &str) -> Element<'_, M> {
    let body = column![words(IN_THE_BODY)];
    column![
        Space::new().height(height),
        words(sentence),
        scrollable(body).id(BODY).height(Length::Fill),
        words(UNDER_THE_BODY),
    ]
    .into()
}

/// The height the words take, with room for them.
fn height_of(sentence: &str) -> f32 {
    let mut ui = simulator(Size::new(LEAST.width, 2000.0), words(sentence));
    ui.find(sentence).unwrap().bounds().height
}

/// How high the fixed part must be to fill the window exactly over `sentence`.
fn filling(sentence: &str) -> f32 {
    LEAST.height - height_of(sentence) - height_of(UNDER_THE_BODY)
}

#[test]
fn the_checks_pass_a_screen_that_fits() {
    let mut ui = simulator(LEAST, a_screen(100.0, FIXED));
    see_in_sight(&mut ui, LEAST, &[FIXED, IN_THE_BODY, UNDER_THE_BODY]).unwrap();
    screen_words(&mut ui, LEAST, &[FIXED, UNDER_THE_BODY], &[IN_THE_BODY]).unwrap();
}

#[test]
fn screen_check_fails_on_a_body_squeezed_to_no_height() {
    let mut ui = simulator(LEAST, a_screen(filling(FIXED), FIXED));
    let body = ui.find(selector::id(BODY)).unwrap();
    assert_eq!(body.bounds().height, 0.0, "the test's own screen");
    screen_words(&mut ui, LEAST, &[FIXED], &[]).unwrap_err();
    screen_words(&mut ui, LEAST, &[IN_THE_BODY], &[]).unwrap_err();
    screen_words(&mut ui, LEAST, &[], &[IN_THE_BODY]).unwrap_err();
}

#[test]
fn screen_check_fails_on_a_fixed_sentence_pushed_out() {
    let mut ui = simulator(LEAST, a_screen(LEAST.height + 100.0, FIXED));
    screen_words(&mut ui, LEAST, &[UNDER_THE_BODY], &[]).unwrap_err();
    screen_words(&mut ui, LEAST, &[FIXED], &[]).unwrap_err();
}

/// A fixed sentence with no room left, while a body of a fixed height keeps its rows, so
/// only the sentence's own check can fail.
#[test]
fn in_sight_fails_on_a_sentence_with_no_room_under_a_body_that_has_room() {
    let body = column![words(IN_THE_BODY)];
    let view = column![
        scrollable(body).id(BODY).height(100.0),
        Space::new().height(LEAST.height),
        words(UNDER_THE_BODY),
    ];
    let mut ui = simulator(LEAST, view);
    let error = see_in_sight(&mut ui, LEAST, &[UNDER_THE_BODY]).unwrap_err();
    assert!(error.contains("out of sight"), "{error}");
}

#[test]
fn a_text_said_to_be_in_the_body_must_be_there() {
    let mut ui = simulator(LEAST, a_screen(100.0, FIXED));
    screen_words(&mut ui, LEAST, &[], &[IN_THE_BODY]).unwrap();
    screen_words(&mut ui, LEAST, &[], &[FIXED]).unwrap_err();
    screen_words(&mut ui, LEAST, &[], &[UNDER_THE_BODY]).unwrap_err();
}

#[test]
fn a_text_far_down_the_body_is_in_the_body_and_out_of_sight() {
    let last = "warning: cannot scan /Volumes/BAD_99: test";
    let gui = booted(with_notes(&[CARD_A], &[DRIVE_A]));
    let mut ui = screen(&gui, LEAST);
    assert!(ui.find(last).unwrap().visible_bounds().is_none());
    screen_words(&mut ui, LEAST, &[CARD_A, "Next"], &[last]).unwrap();
    screen_words(&mut ui, LEAST, &[last], &[]).unwrap_err();
}

#[test]
fn in_sight_fails_when_the_fixed_part_is_1_to_40_too_tall() {
    let fits = filling(TWO_LINES);
    let mut passed = Vec::new();
    for over in 1..=40 {
        let over = over as f32;
        let mut ui = simulator(LEAST, a_screen(fits + over, TWO_LINES));
        if see_in_sight(&mut ui, LEAST, &[TWO_LINES]).is_ok() {
            passed.push(over);
        }
    }
    assert!(
        passed.is_empty(),
        "too tall by these, and it passed: {passed:?}"
    );
}

#[test]
fn in_sight_fails_when_the_body_keeps_no_row() {
    let mut ui = simulator(LEAST, a_screen(filling(FIXED) - 10.0, FIXED));
    let error = see_in_sight(&mut ui, LEAST, &[FIXED, UNDER_THE_BODY]).unwrap_err();
    assert!(error.contains("the body keeps 9.9"), "{error}");
}

#[test]
fn in_sight_fails_on_a_row_of_which_the_body_shows_a_sliver() {
    // Rows to the body's end, and the last only just begun.
    let rows = column![Space::new().height(100.0), words(IN_THE_BODY)];
    let view = column![words(FIXED), scrollable(rows).id(BODY).height(Length::Fill),];
    let height = height_of(FIXED) + 103.0;
    let size = Size::new(LEAST.width, height);
    let mut ui = simulator(size, view);
    let error = see_in_sight(&mut ui, size, &[IN_THE_BODY]).unwrap_err();
    assert!(error.contains("out of sight"), "{error}");
}

#[test]
fn in_sight_fails_on_a_screen_with_no_body() {
    let mut ui = simulator(LEAST, words(FIXED));
    let error = see_in_sight(&mut ui, LEAST, &[FIXED]).unwrap_err();
    assert!(error.contains("no body"), "{error}");
}

// What stays fixed and what scrolls (S064). `in_sight` passes any of these at offset 0.

const IN_THE_BOX: &str = "A sentence in a box.";
const FIELD: &str = "/Volumes/DRIVE_A/Photos";

/// A fixed sentence, a box, the body, a sentence under it, as the real screens' are.
fn a_screen_with_a_box<'a>() -> Element<'a, M> {
    let body = column![words(IN_THE_BODY)];
    column![
        words(FIXED),
        scrollable(words(IN_THE_BOX)).id(BANNER).height(60.0),
        scrollable(body).id(BODY).height(Length::Fill),
        words(UNDER_THE_BODY),
    ]
    .into()
}

#[test]
fn fixed_in_sight_passes_for_what_is_over_and_under_the_body() {
    let mut ui = simulator(LEAST, a_screen_with_a_box());
    see_fixed_in_sight(&mut ui, LEAST, &[FIXED, UNDER_THE_BODY]).unwrap();
}

#[test]
fn fixed_in_sight_fails_for_a_sentence_in_the_body() {
    let mut ui = simulator(LEAST, a_screen_with_a_box());
    see_in_sight(&mut ui, LEAST, &[IN_THE_BODY]).unwrap();
    let error = see_fixed_in_sight(&mut ui, LEAST, &[IN_THE_BODY]).unwrap_err();
    assert!(error.contains("in a scrollable"), "{error}");
}

#[test]
fn fixed_in_sight_fails_for_a_sentence_in_a_box() {
    let mut ui = simulator(LEAST, a_screen_with_a_box());
    see_in_sight(&mut ui, LEAST, &[IN_THE_BOX]).unwrap();
    let error = see_fixed_in_sight(&mut ui, LEAST, &[IN_THE_BOX]).unwrap_err();
    assert!(error.contains("in a scrollable"), "{error}");
}

#[test]
fn fixed_in_sight_fails_inside_a_scrollable_with_no_id_and_a_nested_one() {
    let view = column![
        scrollable(column![
            words(FIXED),
            scrollable(words(IN_THE_BOX)).height(40.0)
        ])
        .height(100.0),
        scrollable(words(IN_THE_BODY)).id(BODY).height(Length::Fill),
    ];
    let mut ui = simulator(LEAST, view);
    see_in_sight(&mut ui, LEAST, &[FIXED, IN_THE_BOX]).unwrap();
    for sentence in [FIXED, IN_THE_BOX] {
        let error = see_fixed_in_sight(&mut ui, LEAST, &[sentence]).unwrap_err();
        assert!(error.contains("in a scrollable"), "{error}");
    }
}

/// The whole screen could be put in a scrollable, with the body inside it.
#[test]
fn fixed_in_sight_fails_when_the_whole_screen_scrolls() {
    let body = scrollable(column![words(IN_THE_BODY)])
        .id(BODY)
        .height(200.0);
    let view = scrollable(column![words(FIXED), body, words(UNDER_THE_BODY)]);
    let mut ui = simulator(LEAST, view);
    see_in_sight(&mut ui, LEAST, &[FIXED, UNDER_THE_BODY]).unwrap();
    for sentence in [FIXED, UNDER_THE_BODY] {
        let error = see_fixed_in_sight(&mut ui, LEAST, &[sentence]).unwrap_err();
        assert!(error.contains("in a scrollable"), "{error}");
    }
}

/// The body's content runs far past what is under it, so its bounds hold that sentence.
#[test]
fn fixed_in_sight_passes_under_a_body_that_overflows_and_beside_a_scrollable() {
    let rows = column![Space::new().height(5000.0), words(IN_THE_BODY)];
    let side = scrollable(words(IN_THE_BOX)).width(100.0).height(60.0);
    let view = column![
        scrollable(rows).id(BODY).height(Length::Fill),
        row![side, words(FIXED)],
        words(UNDER_THE_BODY),
    ];
    let mut ui = simulator(LEAST, view);
    see_fixed_in_sight(&mut ui, LEAST, &[FIXED, UNDER_THE_BODY]).unwrap();
    see_fixed_in_sight(&mut ui, LEAST, &[IN_THE_BODY]).unwrap_err();
    see_fixed_in_sight(&mut ui, LEAST, &[IN_THE_BOX]).unwrap_err();
}

#[test]
fn fixed_in_sight_fails_for_a_sentence_out_of_sight_or_not_there() {
    let body = column![words(IN_THE_BODY)];
    let view = column![
        scrollable(body).id(BODY).height(100.0),
        Space::new().height(LEAST.height),
        words(UNDER_THE_BODY),
    ];
    let mut ui = simulator(LEAST, view);
    let error = see_fixed_in_sight(&mut ui, LEAST, &[UNDER_THE_BODY]).unwrap_err();
    assert!(error.contains("out of sight"), "{error}");
    let error = see_fixed_in_sight(&mut ui, LEAST, &["NOT THERE"]).unwrap_err();
    assert!(error.contains("NOT THERE"), "{error}");
}

#[test]
fn fixed_in_sight_fails_when_the_body_keeps_no_row_or_is_not_there() {
    let mut ui = simulator(LEAST, a_screen(filling(FIXED) - 10.0, FIXED));
    let error = see_fixed_in_sight(&mut ui, LEAST, &[FIXED]).unwrap_err();
    assert!(error.contains("the body keeps"), "{error}");
    let mut ui = simulator(LEAST, words(FIXED));
    let error = see_fixed_in_sight(&mut ui, LEAST, &[FIXED]).unwrap_err();
    assert!(error.contains("no body"), "{error}");
}

#[test]
fn fixed_in_sight_sees_a_fields_words_as_find_does() {
    let view = column![
        text_input("", FIELD),
        scrollable(column![text_input("", IN_THE_BODY)])
            .id(BODY)
            .height(Length::Fill),
    ];
    let mut ui = simulator(LEAST, view);
    see_fixed_in_sight(&mut ui, LEAST, &[FIELD]).unwrap();
    let error = see_fixed_in_sight(&mut ui, LEAST, &[IN_THE_BODY]).unwrap_err();
    assert!(error.contains("in a scrollable"), "{error}");
}

/// A card can be named as a button is. `find` meets the row first.
#[test]
fn fixed_in_sight_passes_words_that_are_also_a_row_far_down_the_body() {
    let rows = column![Space::new().height(5000.0), words(UNDER_THE_BODY)];
    let view = column![
        scrollable(rows).id(BODY).height(Length::Fill),
        words(UNDER_THE_BODY),
    ];
    let mut ui = simulator(LEAST, view);
    see_fixed_in_sight(&mut ui, LEAST, &[UNDER_THE_BODY]).unwrap();
}

#[test]
fn on_a_real_screen_the_banners_line_and_the_buttons_are_fixed_and_the_rest_is_not() {
    let (gui, _dir) = forty_named_syncs();
    let message = gui.app.exit().message.unwrap();
    fixed_in_sight(
        &gui,
        LEAST,
        &[TITLE, NOT_SAFE, "Next", "Look again", "Quit"],
    );
    let mut ui = screen(&gui, LEAST);
    for sentence in [message.as_str(), CARD_A] {
        see_in_sight(&mut ui, LEAST, &[sentence]).unwrap();
        let error = see_fixed_in_sight(&mut ui, LEAST, &[sentence]).unwrap_err();
        assert!(error.contains("in a scrollable"), "{error}");
    }
}

/// The check reads the tree, so a body at its end must change nothing it says, and what
/// it calls fixed must not have moved.
#[test]
fn what_is_fixed_stays_and_what_is_not_goes_when_the_body_is_scrolled() {
    let gui = booted(with_notes(&[CARD_A, CARD_B], &[DRIVE_A]));
    let fixed = [TITLE, CARDS_HEADING, "Next", "Look again", "Quit"];
    let mut ui = screen(&gui, LEAST);
    let before = fixed.map(|sentence| ui.find(sentence).unwrap().visible_bounds());
    see_fixed_in_sight(&mut ui, LEAST, &fixed).unwrap();

    let inside = ui
        .find(selector::id(BODY))
        .unwrap()
        .visible_bounds()
        .unwrap()
        .center();
    ui.point_at(inside);
    let wheel = iced_test::core::Event::Mouse(iced_test::core::mouse::Event::WheelScrolled {
        delta: iced_test::core::mouse::ScrollDelta::Lines {
            x: 0.0,
            y: -100000.0,
        },
    });
    let _ = ui.simulate([wheel]);

    let (end, _, _) = scroll_of(&mut ui, BODY);
    assert!(end > 0.0, "it scrolled");
    let after = fixed.map(|sentence| ui.find(sentence).unwrap().visible_bounds());
    assert_eq!(before, after);
    see_fixed_in_sight(&mut ui, LEAST, &fixed).unwrap();
    let error = see_in_sight(&mut ui, LEAST, &[CARD_A]).unwrap_err();
    assert!(error.contains("out of sight"), "{error}");
    let error = see_fixed_in_sight(&mut ui, LEAST, &[CARD_A]).unwrap_err();
    assert!(error.contains("out of sight"), "{error}");
}

#[test]
fn fixed_box_in_sight_passes_for_words_in_a_box_of_the_fixed_part() {
    let mut ui = simulator(LEAST, a_screen_with_a_box());
    see_fixed_box(&mut ui, LEAST, BANNER, &[IN_THE_BOX]).unwrap();
    let (gui, _dir) = forty_named_syncs();
    let message = gui.app.exit().message.unwrap();
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
}

#[test]
fn fixed_box_in_sight_fails_for_words_outside_that_box() {
    let mut ui = simulator(LEAST, a_screen_with_a_box());
    for sentence in [FIXED, IN_THE_BODY, UNDER_THE_BODY] {
        let error = see_fixed_box(&mut ui, LEAST, BANNER, &[sentence]).unwrap_err();
        assert!(error.contains("not in the box"), "{error}");
    }
}

/// So the banner's words moved into the body, box and all, fail.
#[test]
fn fixed_box_in_sight_fails_for_a_box_in_the_body_or_no_box() {
    let boxed = scrollable(words(IN_THE_BOX)).id(BANNER).height(60.0);
    let view = column![
        words(FIXED),
        scrollable(column![boxed]).id(BODY).height(Length::Fill),
    ];
    let mut ui = simulator(LEAST, view);
    let error = see_fixed_box(&mut ui, LEAST, BANNER, &[IN_THE_BOX]).unwrap_err();
    assert!(error.contains("in a scrollable"), "{error}");

    let mut ui = simulator(LEAST, a_screen(100.0, FIXED));
    let error = see_fixed_box(&mut ui, LEAST, BANNER, &[FIXED]).unwrap_err();
    assert!(error.contains("no box"), "{error}");
}

#[test]
fn fixed_box_in_sight_fails_for_words_below_the_boxs_fold() {
    let inside = column![Space::new().height(500.0), words(IN_THE_BOX)];
    let view = column![
        scrollable(inside).id(BANNER).height(60.0),
        scrollable(words(IN_THE_BODY)).id(BODY).height(Length::Fill),
    ];
    let mut ui = simulator(LEAST, view);
    let error = see_fixed_box(&mut ui, LEAST, BANNER, &[IN_THE_BOX]).unwrap_err();
    assert!(error.contains("out of sight"), "{error}");
}

// `Step::Fails` is a click on words that are there (S064).

#[test]
#[should_panic(expected = "line failed: expect")]
fn a_line_that_must_fail_cannot_pass_on_words_that_are_gone() {
    play_steps(
        literal(&[CARD_A], &[]),
        vec![Step::Fails(r#"click "/Volumes/CARD_GONE""#)],
    );
}

#[test]
#[should_panic(expected = "should have failed")]
fn a_line_that_must_fail_cannot_pass_on_words_in_sight() {
    play_steps(
        literal(&[CARD_A], &[]),
        vec![Step::Fails(r#"click "/Volumes/CARD_A""#)],
    );
}

#[test]
#[should_panic(expected = "a click on words")]
fn a_line_that_must_fail_is_a_click_on_words() {
    play_steps(
        literal(&[CARD_A], &[]),
        vec![Step::Fails(r#"expect "/Volumes/CARD_GONE""#)],
    );
}

#[test]
#[should_panic(expected = "sends nothing")]
fn click_fails_on_a_held_button() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks);
    click(&mut gui, "Next");
}

#[test]
#[should_panic(expected = "sends nothing")]
fn click_fails_on_words_that_are_no_button() {
    let mut gui = two_cards();
    click(&mut gui, TITLE);
}

#[test]
fn a_picture_of_another_renderers_name_compares_nothing_and_fails() {
    let gui = booted(literal(&[], &[]));
    let dir = TempDir::new();
    fs::write(dir.0.join("shot-other.png"), b"not looked at").unwrap();
    let snapshot = screen(&gui, LEAST).snapshot(&Theme::Dark).unwrap();
    let error = same_picture(&snapshot, &dir.0, "shot").unwrap_err();
    assert!(error.contains("compared nothing"), "{error}");
}

#[test]
fn a_second_picture_in_the_folder_fails_the_comparison() {
    let gui = booted(literal(&[], &[]));
    let dir = TempDir::new();
    let snapshot = screen(&gui, LEAST).snapshot(&Theme::Dark).unwrap();
    same_picture(&snapshot, &dir.0, "shot").unwrap_err();
    // The same picture, by the renderer's name: only the stray file can fail it now.
    assert!(snapshot.matches_image(dir.0.join("shot.png")).unwrap());
    same_picture(&snapshot, &dir.0, "shot").unwrap();
    fs::write(dir.0.join("stray.png"), b"x").unwrap();
    same_picture(&snapshot, &dir.0, "shot").unwrap_err();
}

#[test]
fn walking_the_cursor_ends_on_a_screen_whose_key_moves_nothing() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Library);
    let (walked, heard) = mpsc::channel();
    thread::spawn(move || {
        let got_there = gui.cursor_to(1);
        walked.send((got_there, gui.app.cursor())).unwrap();
    });
    let cursor = heard
        .recv_timeout(Duration::from_secs(5))
        .expect("the walk spins for ever");
    assert_eq!(cursor, (false, 0));
}

#[test]
fn a_stale_row_on_a_screen_with_no_list_ends_and_does_nothing() {
    let mut gui = on_the_drive_screen(&[DRIVE_A, DRIVE_B]);
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    let (walked, heard) = mpsc::channel();
    thread::spawn(move || {
        let _ = update(&mut gui, M::Row(PathBuf::from(DRIVE_B)));
        walked.send(gui.app.cursor()).unwrap();
    });
    let cursor = heard
        .recv_timeout(Duration::from_secs(5))
        .expect("the walk spins for ever");
    assert_eq!(cursor, 0);
}

/// The WCAG ratio of two opaque colours.
fn contrast(one: Color, other: Color) -> f32 {
    let light = |c: Color| {
        let part = |v: f32| {
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * part(c.r) + 0.7152 * part(c.g) + 0.0722 * part(c.b)
    };
    let (a, b) = (light(one), light(other));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn the_row_under_the_cursor_can_be_read() {
    for status in [button::Status::Active, button::Status::Hovered] {
        let style = row_under_cursor(&Theme::Dark, status);
        let Some(iced::Background::Color(behind)) = style.background else {
            panic!("the row must show: {style:?}");
        };
        let ratio = contrast(style.text_color, behind);
        // Over WCAG's 4.5 for plain text, as these are thin letters on a bar.
        assert!(ratio >= 5.5, "{status:?}: {ratio}");
        assert!(
            contrast(behind, Theme::Dark.palette().background) >= 1.5,
            "{status:?}: the bar cannot be told from the window"
        );
    }
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
    let started = Instant::now();
    loop {
        match heard.try_recv() {
            Ok(Event::Action(action)) => emulator.perform(&program, action),
            Ok(Event::Failed(line)) => panic!("{line}"),
            Ok(Event::Ready) => break,
            Err(TryRecvError::Closed) => panic!("the runtime stopped"),
            Err(TryRecvError::Empty) => {
                assert!(started.elapsed() < LIMIT, "nothing happened for too long");
                thread::sleep(Duration::from_millis(1));
            }
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
    same_picture(&snapshot, &dir.0, "shot").unwrap();
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

// The library, name and check screens, and the refusals before a session (S053).

const LIBRARY_HEADING: &str = "3/5  Library folder on the drive";
const NAME_HEADING: &str = "4/5  Name this sync";
const CHECK_HEADING: &str = "5/5  Check and start";
const REFUSED_HEADING: &str = "Stopped, no report";
const FIELD_BUTTONS: [&str; 2] = ["Next", "Back"];
const CHECK_BUTTONS: [&str; 3] = ["Start copying", "Back", "Quit"];
const REFUSED_BACK: &str = "Back, to fix it";
const NO_CARD_HINT: &str = "No card picked: the card is gone. Go back to the cards and pick one.";
const NO_LIBRARY_HINT: &str = "Type the library folder on your drive.";

/// Real folders: two cards and a drive, whose library is not made.
struct Folders {
    dir: TempDir,
    card: String,
    card_b: String,
    drive: String,
    library: String,
}

impl Folders {
    fn new() -> Folders {
        let dir = TempDir::new();
        let path = |name: &str| dir.0.join(name).display().to_string();
        let (card, card_b, drive) = (path("CARD"), path("CARD_B"), path("DRIVE"));
        for folder in [&card, &card_b, &drive] {
            fs::create_dir_all(folder).unwrap();
        }
        let library = dir.0.join("DRIVE/Photos").display().to_string();
        Folders {
            dir,
            card,
            card_b,
            drive,
            library,
        }
    }

    /// All there is on the drive, to see that nothing was made.
    fn on_the_drive(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.drive)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect()
    }
}

fn on_the_library_screen(folders: &Folders) -> Gui {
    let mut gui = booted(literal(&[&folders.card], &[&folders.drive]));
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Library);
    gui
}

fn to_the_check(folders: &Folders) -> Gui {
    let mut gui = on_the_library_screen(folders);
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);
    gui
}

/// What the screen's field holds in the model.
fn field_of(gui: &Gui) -> String {
    match gui.app.screen() {
        Screen::Library => gui.app.library().to_string(),
        _ => gui.app.name().to_string(),
    }
}

fn fill(gui: &mut Gui, value: &str) {
    let _ = update(gui, M::Field(value.to_string()));
}

fn goes_to(gui: &Gui) -> String {
    format!(
        "New photos go to {}/{}/RAW, JPEG, ...",
        gui.app.library().trim(),
        gui.app.folder_preview()
    )
}

fn check_folder_line(gui: &Gui) -> String {
    format!("{}   {THE_DATE_TO_COME}", gui.app.folder_preview())
}

#[test]
fn the_library_screen_shows_its_field_its_help_and_its_buttons() {
    let folders = Folders::new();
    let gui = on_the_library_screen(&folders);
    assert_eq!(gui.app.library(), folders.library);
    assert_eq!(
        LIBRARY_HELP,
        [
            "The folder on the drive that holds your photos. It must exist:",
            "phoosync pro 2 never creates it. Photos already anywhere in it are skipped.",
        ]
    );
    let words = [
        TITLE,
        LIBRARY_HEADING,
        LIBRARY_HELP[0],
        LIBRARY_HELP[1],
        folders.library.as_str(),
        "Next",
        "Back",
    ];
    screen_check(&gui, "S053", "library", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, LIBRARY_HEADING]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
    assert!(!says(&gui, "Quit"), "q would type: DEC-068 (2)");
}

#[test]
fn the_name_screen_shows_where_photos_go_and_it_changes_as_typed() {
    let folders = Folders::new();
    let mut gui = on_the_library_screen(&folders);
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Name);
    let no_name = format!("New photos go to {}/<date>/RAW, JPEG, ...", folders.library);
    assert!(says(&gui, &no_name), "{:?}", all_words(&gui, WINDOW));

    fill(&mut gui, "Tira");
    assert!(!says(&gui, &no_name));
    let part = format!(
        "New photos go to {}/<date> - Tira/RAW, JPEG, ...",
        folders.library
    );
    assert!(says(&gui, &part));

    fill(&mut gui, "Tirana");
    let whole = format!(
        "New photos go to {}/<date> - Tirana/RAW, JPEG, ...",
        folders.library
    );
    assert!(!says(&gui, &part));
    assert_eq!(whole, goes_to(&gui));
    let words = [
        TITLE,
        NAME_HEADING,
        whole.as_str(),
        NAME_HELP,
        "Tirana",
        "Next",
        "Back",
    ];
    screen_check(&gui, "S053", "name", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, NAME_HEADING]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
}

#[test]
fn what_a_field_is_given_goes_through_the_models_edit() {
    let folders = Folders::new();
    let mut gui = on_the_library_screen(&folders);
    click(&mut gui, "Next");
    // A letter a button's key would mean elsewhere is a letter here.
    fill(&mut gui, "qr nc");
    assert_eq!(gui.app.name(), "qr nc");
    assert_eq!(gui.app.screen(), Screen::Name);
    // Shorter, as a backspace or a cut leaves it.
    fill(&mut gui, "qr");
    assert_eq!(gui.app.name(), "qr");
    // Accents count as one each, and a pasted line's end is no part of a name.
    fill(&mut gui, "é写\nz");
    assert_eq!(gui.app.name(), "é写z");
    fill(&mut gui, "");
    assert_eq!(gui.app.name(), "");
    assert!(!gui.quit);
}

#[test]
fn an_empty_library_stays_and_next_says_why() {
    let folders = Folders::new();
    for empty in ["", "   "] {
        let mut gui = on_the_library_screen(&folders);
        fill(&mut gui, empty);
        click(&mut gui, "Next");
        assert_eq!(gui.app.screen(), Screen::Library);
        assert_eq!(gui.app.hint(), Some(NO_LIBRARY_HINT));
        fixed_in_sight(&gui, LEAST, &[NO_LIBRARY_HINT, "Next", "Back"]);
        // The next key is another attempt, as in the TUI.
        fill(&mut gui, &folders.library);
        assert_eq!(gui.app.hint(), None);
    }
}

#[test]
fn back_walks_the_screens_and_keeps_what_was_typed() {
    let folders = Folders::new();
    let mut gui = on_the_library_screen(&folders);
    fill(&mut gui, "/edited");
    click(&mut gui, "Next");
    fill(&mut gui, "Tirana");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Name);
    assert_eq!(gui.app.name(), "Tirana");
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(gui.app.library(), "/edited");
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Drive);
}

#[test]
fn enter_in_a_field_is_next_and_typing_needs_no_click_first() {
    // Through the real runtime, as only it runs a task that gives a field its focus.
    let gui = play(
        literal(&[CARD_A], &[DRIVE_A]),
        &[
            r#"click "Next""#,
            r#"click "Next""#,
            r#"type "2""#,
            "type enter",
            r#"type "Tirana""#,
            "type enter",
            r#"expect "Start copying""#,
        ],
    );
    assert_eq!(gui.app.screen(), Screen::Confirm);
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Photos2");
    assert_eq!(gui.app.name(), "Tirana");
    assert!(gui.kept_job.is_none(), "Enter here only moved to the check");
}

#[test]
fn back_to_a_field_types_at_its_end() {
    let gui = play(
        literal(&[CARD_A], &[DRIVE_A]),
        &[
            r#"click "Next""#,
            r#"click "Next""#,
            "type enter",
            r#"type "Tirana""#,
            r#"click "Back""#,
            r#"type "2""#,
            "type enter",
            r#"type "!""#,
        ],
    );
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Photos2");
    assert_eq!(gui.app.name(), "Tirana!");
}

#[test]
fn enter_on_an_empty_library_stays_as_next_does() {
    let gui = play(
        literal(&[CARD_A], &[DRIVE_A]),
        &[r#"click "Next""#, r#"click "Next""#, "type enter"],
    );
    assert_eq!(gui.app.screen(), Screen::Name, "the default library");

    // Every letter of it gone, by the field's own keys: the model hears each.
    let mut lines = vec![r#"click "Next""#, r#"click "Next""#];
    lines.extend(std::iter::repeat_n(
        "type backspace",
        "/Volumes/DRIVE_A/Photos".len(),
    ));
    lines.push("type enter");
    let gui = play(literal(&[CARD_A], &[DRIVE_A]), &lines);
    assert_eq!(gui.app.library(), "");
    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(gui.app.hint(), Some(NO_LIBRARY_HINT));
}

#[test]
fn the_check_lists_every_card_the_library_the_folder_and_its_buttons() {
    let folders = Folders::new();
    let mut gui = booted(literal(
        &[&folders.card, &folders.card_b],
        &[&folders.drive],
    ));
    click(&mut gui, &folders.card);
    click(&mut gui, &folders.card_b);
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    fill(&mut gui, "Tirana");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);
    let line = format!("<date> - Tirana   {THE_DATE_TO_COME}");
    assert_eq!(line, check_folder_line(&gui));
    assert_eq!(THE_DATE_TO_COME, "(the date of the earliest new photo)");
    assert_eq!(
        CARDS_ONLY_READ,
        "The cards are only read. Photos already in the library are skipped."
    );
    let words = [
        TITLE,
        CHECK_HEADING,
        folders.card.as_str(),
        folders.card_b.as_str(),
        folders.library.as_str(),
        line.as_str(),
        CARDS_ONLY_READ,
        "Start copying",
        "Back",
        "Quit",
    ];
    screen_check(&gui, "S053", "check_two_cards", &words);
    fixed_in_sight(&gui, LEAST, &[TITLE, CHECK_HEADING]);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
    assert!(!says(&gui, "Next"));

    click(&mut gui, "Quit");
    assert!(gui.quit);
    assert!(gui.kept_job.is_none());
}

#[test]
fn twenty_cards_on_the_check_scroll_and_the_buttons_stay() {
    let folders = Folders::new();
    let cards: Vec<String> = (0..20).map(|n| format!("/Volumes/CARD_{n:02}")).collect();
    let names: Vec<&str> = cards.iter().map(String::as_str).collect();
    let mut gui = booted(literal(&names, &[&folders.drive]));
    for card in &cards {
        let _ = update(&mut gui, M::Row(PathBuf::from(card)));
    }
    assert_eq!(gui.app.picked().len(), 20);
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    let body: Vec<&str> = names
        .iter()
        .copied()
        .chain([CARDS_ONLY_READ, folders.library.as_str()])
        .collect();
    screen_check_scrolling(&gui, "S053", "check_twenty_cards", &[TITLE], &body);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
}

#[test]
fn a_long_library_shows_whole_on_the_name_screen_and_the_check() {
    let folders = Folders::new();
    let mut gui = on_the_library_screen(&folders);
    let long = long_path("/Volumes/DRIVE");
    fill(&mut gui, &long);
    click(&mut gui, "Next");
    let preview = goes_to(&gui);
    in_sight(&gui, LEAST, &[&preview]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
    picture_at(&gui, LEAST, "S053", "name_long_library_least");
    click(&mut gui, "Next");
    in_sight(&gui, LEAST, &[&long]);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
    picture_at(&gui, LEAST, "S053", "check_long_library_least");
}

#[test]
fn a_library_that_is_not_there_is_refused_at_start_and_can_be_fixed() {
    let folders = Folders::new();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");

    let library = PathBuf::from(&folders.library);
    let sentence = text::refusal(&flow::start_found(&library, None).unwrap_err());
    let start = format!("The library folder {} does not exist.", folders.library);
    assert!(sentence.starts_with(&start), "{sentence}");
    assert_eq!(gui.app.screen(), Screen::Refused);
    assert!(!gui.app.in_session(), "no session made");
    assert!(gui.kept_job.is_none());
    assert_eq!(
        folders.on_the_drive(),
        Vec::<PathBuf>::new(),
        "no folder made"
    );
    assert!(!says(&gui, "Next card"));
    assert!(!says(&gui, NOT_SAFE), "it says it itself: DEC-071 (2)");
    let words = [
        TITLE,
        REFUSED_HEADING,
        sentence.as_str(),
        REFUSED_BACK,
        "Quit",
    ];
    screen_check_scrolling(&gui, "S053", "refused_no_library", &words, &[&sentence]);
    fixed_in_sight(&gui, LEAST, &[TITLE, REFUSED_HEADING, REFUSED_BACK, "Quit"]);

    click(&mut gui, REFUSED_BACK);
    assert_eq!(gui.app.screen(), Screen::Confirm);
    click(&mut gui, "Back");
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Library);

    let fixed = folders.dir.0.join("DRIVE/Fixed");
    fs::create_dir_all(&fixed).unwrap();
    fill(&mut gui, &fixed.display().to_string());
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);
    assert!(says(&gui, &fixed.display().to_string()));

    // The next Start goes on: the model's own answer, as nothing here runs it.
    match gui.app.update(Msg::Key(Key::Enter)) {
        Some(Effect::Start(job)) => {
            assert_eq!(job.sources, [PathBuf::from(&folders.card)]);
            assert_eq!(flow::lock(&job.session).library(), fixed);
        }
        other => panic!("no start: {other:?}"),
    }
    assert!(gui.app.in_session());
    assert_eq!(
        fs::read_dir(&fixed).unwrap().count(),
        0,
        "the job did not run"
    );
}

#[test]
fn start_copying_keeps_the_job_and_runs_nothing() {
    let folders = Folders::new();
    fs::create_dir_all(&folders.library).unwrap();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");
    assert_eq!(gui.app.screen(), Screen::Running);
    assert!(gui.app.in_session());
    let job = gui.kept_job.as_ref().expect("the job is kept");
    assert_eq!(job.sources, [PathBuf::from(&folders.card)]);
    assert_eq!(
        flow::lock(&job.session).library(),
        PathBuf::from(&folders.library)
    );
    assert_eq!(fs::read_dir(&folders.library).unwrap().count(), 0);
}

#[test]
fn a_bad_name_is_refused_the_same_with_no_card_read_and_can_be_fixed() {
    let long = "n".repeat(243);
    for (name, picture_name) in [
        ("a/b", "refused_bad_name"),
        (long.as_str(), "refused_long_name"),
    ] {
        let folders = Folders::new();
        fs::create_dir_all(&folders.library).unwrap();
        // This card is not there: reading it would be another refusal.
        let mut gui = booted(literal(&["/Volumes/NO_SUCH_CARD"], &[&folders.drive]));
        click(&mut gui, "Next");
        click(&mut gui, "Next");
        click(&mut gui, "Next");
        fill(&mut gui, name);
        click(&mut gui, "Next");
        click(&mut gui, "Start copying");

        let library = PathBuf::from(&folders.library);
        let sentence = text::refusal(&flow::start_found(&library, Some(name)).unwrap_err());
        assert!(sentence.starts_with("The name "), "{sentence}");
        assert_eq!(gui.app.screen(), Screen::Refused);
        assert!(!gui.app.in_session(), "no session made");
        assert!(gui.kept_job.is_none());
        assert!(!says(&gui, "Next card"));
        assert_eq!(fs::read_dir(&library).unwrap().count(), 0, "no folder made");
        let words = [
            TITLE,
            REFUSED_HEADING,
            sentence.as_str(),
            REFUSED_BACK,
            "Quit",
        ];
        screen_check_scrolling(&gui, "S053", picture_name, &words, &[&sentence]);
        fixed_in_sight(&gui, LEAST, &[REFUSED_BACK, "Quit"]);

        click(&mut gui, REFUSED_BACK);
        click(&mut gui, "Back");
        assert_eq!(gui.app.screen(), Screen::Name);
        assert_eq!(gui.app.name(), name, "still there to fix");
        fill(&mut gui, "Tirana");
        click(&mut gui, "Next");
        click(&mut gui, "Start copying");
        assert_eq!(gui.app.screen(), Screen::Running);
        assert!(gui.app.in_session());
    }
}

#[test]
fn refused_buttons_answer_and_quit_quits() {
    let folders = Folders::new();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");
    click(&mut gui, "Quit");
    assert!(gui.quit);
    assert_eq!(gui.app.exit().code, 2);
}

#[test]
fn no_sync_starts_with_the_only_card_dropped_by_look_again() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A], &[DRIVE_A], &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    click(&mut gui, "Look again");
    next_look(&looks)
        .send(volumes(&[], &[DRIVE_A], &[]))
        .unwrap();
    settle(&mut gui);
    assert!(gui.app.picked().is_empty());
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);

    click(&mut gui, "Start copying");
    assert_eq!(gui.app.screen(), Screen::Confirm);
    assert!(!gui.app.in_session());
    assert!(gui.kept_job.is_none());
    fixed_in_sight(&gui, LEAST, &[NO_CARD_HINT]);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
    no_key_named(&gui, "the check's hint");
    picture_at(&gui, WINDOW, "S053", "check_no_card");
}

#[test]
fn the_banner_holds_on_the_library_name_and_check_screens() {
    let folders = Folders::new();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");
    let message = gui.app.exit().message.unwrap();
    assert_eq!(gui.app.exit().code, 2);
    assert!(message.contains("does not exist"), "{message}");

    click(&mut gui, REFUSED_BACK);
    let line = check_folder_line(&gui);
    in_sight(&gui, LEAST, &[&folders.library, &line]);
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    let words = [
        NOT_SAFE,
        message.as_str(),
        folders.card.as_str(),
        "Start copying",
    ];
    screen_check(&gui, "S053", "check_banner", &words);

    click(&mut gui, "Back");
    fill(&mut gui, "Tirana");
    in_sight(&gui, LEAST, &["Tirana"]);
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    screen_check(
        &gui,
        "S053",
        "name_banner",
        &[NOT_SAFE, message.as_str(), "Tirana"],
    );

    click(&mut gui, "Back");
    in_sight(&gui, LEAST, &[&folders.library]);
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    let words = [NOT_SAFE, message.as_str(), folders.library.as_str(), "Next"];
    screen_check(&gui, "S053", "library_banner", &words);
}

#[test]
fn a_long_banner_leaves_each_field_and_button() {
    let folders = Folders::new();
    let mut gui = on_the_library_screen(&folders);
    let long = "/not_there".repeat(100);
    fill(&mut gui, &long);
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Start copying");
    let message = gui.app.exit().message.unwrap();
    click(&mut gui, REFUSED_BACK);
    fixed_in_sight(&gui, LEAST, &[NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    let (_, height, content) = scroll_of(&mut screen(&gui, LEAST), BANNER);
    assert!(height <= 104.0, "{height}");
    assert!(content > height, "{content} in {height}");

    click(&mut gui, "Back");
    fill(&mut gui, "Tirana");
    in_sight(&gui, LEAST, &["Tirana"]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    click(&mut gui, "Back");
    in_sight(&gui, LEAST, &[&long]);
    fixed_in_sight(&gui, LEAST, &FIELD_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    picture_at(&gui, LEAST, "S053", "library_long_banner_least");
}

#[test]
fn the_check_of_a_session_says_what_stays_with_no_key_named() {
    let (mut gui, _dir) = forty_named_syncs();
    let message = gui.app.exit().message.unwrap();
    click(&mut gui, "Next");
    assert_eq!(gui.app.screen(), Screen::Confirm);
    let stays = "Sync 41 of this session: the library and name stay as they are. Click Quit to change them.";
    // Every sync crashed before a folder was made, so the date is still to come.
    let line = check_folder_line(&gui);
    in_sight(&gui, LEAST, &[CARD_A]);
    fixed_in_sight(&gui, LEAST, &["Sync 41: check and start", NOT_SAFE]);
    fixed_in_sight(&gui, LEAST, &CHECK_BUTTONS);
    fixed_box_in_sight(&gui, LEAST, BANNER, &[&message]);
    screen_check_scrolling(&gui, "S053", "check_in_session", &[TITLE], &[stays, &line]);

    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Sources);
}

#[test]
fn stale_clicks_start_nothing_on_the_check_or_the_refusal() {
    let folders = Folders::new();
    fs::create_dir_all(&folders.library).unwrap();
    let mut gui = to_the_check(&folders);
    let library = gui.app.library().to_string();
    // Left from the name screen: its Next, and Enter in its field, send this.
    for message in [
        M::Next,
        M::Field("q r n".to_string()),
        M::Row(PathBuf::from(&folders.card)),
        M::LookAgain,
    ] {
        let _ = update(&mut gui, message);
        assert_eq!(gui.app.screen(), Screen::Confirm);
        assert!(gui.kept_job.is_none());
        assert!(!gui.app.in_session());
    }
    // Letters that mean something off a field: q quits, a space picks.
    assert!(!gui.quit);
    assert_eq!(gui.app.library(), library);
    assert_eq!(gui.app.name(), "");

    let folders = Folders::new();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");
    assert_eq!(gui.app.screen(), Screen::Refused);
    for message in [
        M::Start,
        M::Next,
        M::Field("q r n".to_string()),
        M::Row(PathBuf::from(&folders.card)),
        M::LookAgain,
    ] {
        let _ = update(&mut gui, message);
        assert_eq!(gui.app.screen(), Screen::Refused);
    }
    assert!(!gui.quit);

    // Off these screens, the last click of Start copying has nothing to start.
    let folders = Folders::new();
    fs::create_dir_all(&folders.library).unwrap();
    let mut gui = to_the_check(&folders);
    click(&mut gui, "Start copying");
    let _ = update(&mut gui, M::Start);
    let _ = update(&mut gui, M::Quit);
    let _ = update(&mut gui, M::Back);
    let _ = update(&mut gui, M::Next);
    assert_eq!(gui.app.screen(), Screen::Running);
    assert!(!gui.quit);

    // A field's stale words reach no list either: a space would pick the row.
    let folders = Folders::new();
    let mut gui = booted(literal(&[&folders.card], &[&folders.drive]));
    let _ = update(&mut gui, M::Field(" ".to_string()));
    let _ = update(&mut gui, M::Start);
    assert_eq!(gui.app.screen(), Screen::Sources);
    assert!(gui.app.picked().is_empty());
    assert_eq!(gui.app.library(), "");
}

/// The widths a button's words stand in, with a button's own padding either side.
fn across(gui: &Gui, size: Size, words: &str) -> (f32, f32) {
    let found = screen(gui, size).find(words).unwrap();
    let bounds = found.bounds();
    (bounds.x - 10.0, bounds.x + bounds.width + 10.0)
}

fn apart(one: (f32, f32), other: (f32, f32)) -> bool {
    one.1 <= other.0 || other.1 <= one.0
}

#[test]
fn the_second_click_of_a_double_lands_on_nothing_that_starts_or_quits() {
    let folders = Folders::new();
    for size in [WINDOW, LEAST] {
        let mut gui = on_the_library_screen(&folders);
        let next = across(&gui, size, "Next");
        click(&mut gui, "Next");
        assert_eq!(next, across(&gui, size, "Next"), "library and name alike");
        click(&mut gui, "Next");
        let start = across(&gui, size, "Start copying");
        let quit = across(&gui, size, "Quit");
        assert!(apart(next, start), "Next {next:?}, Start copying {start:?}");
        assert!(apart(next, quit), "Next {next:?}, Quit {quit:?}");

        click(&mut gui, "Start copying");
        let back = across(&gui, size, REFUSED_BACK);
        let refused_quit = across(&gui, size, "Quit");
        assert!(apart(start, back), "Start {start:?}, Back {back:?}");
        assert!(
            apart(start, refused_quit),
            "Start {start:?}, Quit {refused_quit:?}"
        );
        assert!(
            apart(back, quit),
            "Back, to fix it {back:?}, the check's Quit {quit:?}"
        );
    }
}

#[test]
fn the_words_here_are_the_tuis_words() {
    let tui = include_str!("../../../photosync/src/tui/view.rs");
    for words in [
        LIBRARY_HELP[0],
        LIBRARY_HELP[1],
        NAME_HELP,
        CARDS_ONLY_READ,
        THE_DATE_TO_COME,
        gui::THE_SESSIONS_FOLDER,
        "New photos go to {}/{}/RAW, JPEG, ...",
        LIBRARY_HEADING,
        NAME_HEADING,
        CHECK_HEADING,
        REFUSED_HEADING,
    ] {
        assert!(tui.contains(words), "{words}");
    }
}

// A look that misses the chosen drive resets the library (specs).

#[test]
fn a_drive_gone_and_back_and_taken_again_counts_as_changed_and_resets_an_edited_library() {
    let (discover, looks) = blocked_discovery();
    let (mut gui, _) = Gui::boot(discover, None);
    next_look(&looks)
        .send(volumes(&[CARD_A], &[DRIVE_A], &[]))
        .unwrap();
    settle(&mut gui);
    click(&mut gui, "Next");
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");
    fill(&mut gui, "/Volumes/DRIVE_A/Edited");
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Edited");
    click(&mut gui, "Back");
    assert_eq!(gui.app.screen(), Screen::Drive);

    look_gives(&mut gui, &looks, &[]);
    assert_eq!(gui.app.drive(), None, "the look missed it");
    look_gives(&mut gui, &looks, &[DRIVE_A]);
    assert_eq!(gui.marked_drive(), None, "DEC-077 (4)");
    click(&mut gui, DRIVE_A);
    click(&mut gui, "Next");

    assert_eq!(gui.app.screen(), Screen::Library);
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_A/Photos");
    in_sight(&gui, LEAST, &["/Volumes/DRIVE_A/Photos"]);
    assert!(!says(&gui, "/Volumes/DRIVE_A/Edited"));
}

// Where the marked drive goes below the fold (DEC-075 (9), DEC-077 (8)).

/// Rows shown whole with the body at its top: what a marked row needs to be seen.
fn whole_rows(gui: &Gui, size: Size, rows: &[String]) -> usize {
    let mut ui = screen(gui, size);
    rows.iter()
        .filter(|row| {
            let found = ui.find(row.as_str()).unwrap();
            let bounds = found.bounds();
            found
                .visible_bounds()
                .is_some_and(|seen| seen.height >= bounds.height - 0.5)
        })
        .count()
}

#[test]
fn a_marked_drive_goes_below_the_fold_at_the_number_the_decisions_give() {
    let rows: Vec<String> = (0..40).map(|n| format!("/Volumes/DRIVE_{n:02}")).collect();
    let names: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mut gui = booted(literal(&[CARD_A], &names));
    click(&mut gui, "Next");
    click(&mut gui, &rows[0]);
    let plain = (
        whole_rows(&gui, WINDOW, &rows),
        whole_rows(&gui, LEAST, &rows),
    );

    // One sentence with a short path: two rows of the banner's box, or three at the least.
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    click(&mut gui, "Start copying");
    assert_eq!(gui.app.screen(), Screen::Refused);
    for _ in 0..4 {
        let _ = update(&mut gui, M::Back);
    }
    assert_eq!(gui.app.screen(), Screen::Drive);
    let banner = (
        whole_rows(&gui, WINDOW, &rows),
        whole_rows(&gui, LEAST, &rows),
    );

    // Enough words to fill the box's five rows.
    let _ = update(&mut gui, M::Next);
    fill(&mut gui, &"/not_there".repeat(100));
    let _ = update(&mut gui, M::Next);
    let _ = update(&mut gui, M::Next);
    let _ = update(&mut gui, M::Start);
    assert_eq!(gui.app.screen(), Screen::Refused);
    for _ in 0..4 {
        let _ = update(&mut gui, M::Back);
    }
    assert_eq!(gui.app.screen(), Screen::Drive);
    let tallest = (
        whole_rows(&gui, WINDOW, &rows),
        whole_rows(&gui, LEAST, &rows),
    );

    // If the look changes, so do DEC-075 (9) and DEC-077 (8).
    assert_eq!((plain, banner, tallest), ((13, 9), (11, 6), (9, 4)));
    let mut ui = screen(&gui, WINDOW);
    let first = ui.find(rows[0].as_str()).unwrap().bounds().y;
    let second = ui.find(rows[1].as_str()).unwrap().bounds().y;
    assert!((second - first - 36.8).abs() < 0.01, "{}", second - first);
}

#[test]
fn a_marked_drive_below_the_fold_is_what_next_takes_and_the_library_names_it() {
    let rows: Vec<String> = (0..40).map(|n| format!("/Volumes/DRIVE_{n:02}")).collect();
    let names: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mut gui = booted(literal(&[CARD_A], &names));
    click(&mut gui, "Next");
    // The wheel took the user there: it is out of sight at the top.
    let _ = update(&mut gui, M::Row(PathBuf::from(&rows[39])));
    let seen = screen(&gui, WINDOW)
        .find(rows[39].as_str())
        .unwrap()
        .visible_bounds();
    assert!(seen.is_none(), "{seen:?}");
    assert_eq!(gui.marked_drive(), Some(PathBuf::from(&rows[39]).as_path()));
    fixed_in_sight(&gui, WINDOW, &["Next"]);
    click(&mut gui, "Next");
    assert_eq!(gui.app.library(), "/Volumes/DRIVE_39/Photos");
    click(&mut gui, "Next");
    click(&mut gui, "Next");
    assert!(says(&gui, "/Volumes/DRIVE_39/Photos"), "the check names it");
}

#[test]
fn a_refusals_sentence_can_be_read() {
    let style = iced::widget::container::danger(&Theme::Dark);
    let Some(iced::Background::Color(behind)) = style.background else {
        panic!("the red must show: {style:?}");
    };
    let ratio = contrast(style.text_color.unwrap(), behind);
    assert!(ratio >= 4.5, "{ratio}");
}
