//! `Gui`, its messages, and the plain functions iced runs. Each widget sends what the
//! TUI's key does, so every rule stays in `App` (DEC-068 (2)).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use iced::futures::channel::oneshot;
use iced::widget::scrollable::RelativeOffset;
use iced::widget::{column, operation, row, scrollable};
use iced::{Element, Font, Length, Size, Subscription, Task, Theme, window};
use photosync_core::Volumes;
use photosync_front::app::{App, Effect, Exit, Key, Msg, Outcome, Screen};
use photosync_front::system::say;

use crate::png;
use crate::widgets::{action, hint, list_row, red_banner, word_box, words};

/// The name the user sees (DEC-069 (1)).
pub const TITLE: &str = "phoosync pro 2";
/// The window's size, and the least it can be made (DEC-068 (7)).
pub const WINDOW: Size = Size::new(900.0, 650.0);
pub const LEAST: Size = Size::new(640.0, 480.0);
/// What is said while the newest look is out (DEC-073 (2)).
pub const LOOKING: &str = "Looking for cards and drives. A stuck card can hold this up: take it out, or press Look again.";

/// The banner's line (DEC-071 (2)).
pub const NOT_SAFE: &str = "Not every photo is known to be on the drive: do not format the card.";
/// The scrollables' fixed ids, as `Scrollable::diff` never looks at an id (DEC-073 (1)).
pub const BODY: &str = "body";
pub const BANNER: &str = "banner";
/// The most the banner's box takes: five rows (DEC-071 (3)).
const BANNER_MOST: f32 = 104.0;

/// How volumes are found: injected, so tests give literal folders (DEC-068 (5)).
pub type Discover = Arc<dyn Fn() -> Volumes + Send + Sync>;

/// `Msg` is not `Clone` and a button needs that, so what the worker and discovery
/// send stays in channels, and this only says to go and read them (DEC-068 (4)).
#[derive(Debug, Clone)]
pub enum M {
    Row(PathBuf),
    Next,
    Back,
    LookAgain,
    Quit,
    /// A look's thread has ended.
    Looked,
    CloseRequested,
    /// A frame was drawn, while `--smoke-png` waits for one.
    SmokeFrame,
    SmokeShot(window::Screenshot),
    /// A wheel to the body's end: the Emulator has no wheel (DEC-073 (1)).
    #[cfg(test)]
    ScrollEndForTest,
}

/// What `--smoke-png` still has to do.
struct Smoke {
    path: PathBuf,
    frames: u32,
}

pub struct Gui {
    pub app: App,
    discover: Discover,
    /// The number of the latest look started, and of the one the model has.
    newest_look: u64,
    answered_look: u64,
    answers_tx: Sender<(u64, Volumes)>,
    answers: Receiver<(u64, Volumes)>,
    /// Answers dropped as older than the newest look: a test waits on it.
    pub dropped_looks: u64,
    /// The model said to quit. iced's exit cannot be seen by a test.
    pub quit: bool,
    smoke: Option<Smoke>,
}

impl Gui {
    /// The state, and the task of the first look.
    pub fn boot(discover: Discover, smoke_png: Option<PathBuf>) -> (Gui, Task<M>) {
        let (answers_tx, answers) = mpsc::channel();
        let mut gui = Gui {
            // Never `App::new`: the library check stays on (DEC-050 (7)).
            app: App::checking_library(),
            discover,
            newest_look: 0,
            answered_look: 0,
            answers_tx,
            answers,
            dropped_looks: 0,
            quit: false,
            // Two: the first may be of the frame before the answer.
            smoke: smoke_png.map(|path| Smoke { path, frames: 2 }),
        };
        let task = gui.look();
        (gui, task)
    }

    /// The newest look has not answered: nothing shown can be trusted yet.
    pub fn looking(&self) -> bool {
        self.answered_look != self.newest_look
    }

    /// A stuck mount must not freeze the window, so each look has its own thread. One
    /// that never ends holds nothing the app needs, until the process ends (DEC-073 (2)).
    fn look(&mut self) -> Task<M> {
        self.newest_look += 1;
        let number = self.newest_look;
        let discover = self.discover.clone();
        let answers = self.answers_tx.clone();
        let (ended, heard) = oneshot::channel::<()>();
        thread::spawn(move || {
            let volumes = discover();
            let _ = answers.send((number, volumes));
            let _ = ended.send(());
        });
        Task::perform(heard, |_| M::Looked)
    }

    /// Only the newest look reaches the model: two can answer out of order.
    fn take_answers(&mut self) {
        while let Ok((number, volumes)) = self.answers.try_recv() {
            if number == self.newest_look {
                self.answered_look = number;
                self.app.update(Msg::Volumes(volumes));
            } else {
                self.dropped_looks += 1;
            }
        }
    }

    fn key(&mut self, key: Key) -> Task<M> {
        match self.app.update(Msg::Key(key)) {
            None => Task::none(),
            Some(Effect::Discover) => self.look(),
            Some(Effect::Quit) => {
                self.quit = true;
                iced::exit()
            }
            // Only the check screen starts a sync, and no window reaches it before S053.
            Some(Effect::Start(_)) => unreachable!("no sync can start before S053"),
        }
    }

    /// The model's keys move the cursor one row at a time, and stop at the ends. As many
    /// keys as there are rows to go, so a key that moves nothing cannot hold the window.
    /// Whether it got there.
    pub(crate) fn cursor_to(&mut self, row: usize) -> bool {
        let key = if self.app.cursor() < row {
            Key::Down
        } else {
            Key::Up
        };
        for _ in 0..self.app.cursor().abs_diff(row) {
            self.app.update(Msg::Key(key));
        }
        self.app.cursor() == row
    }

    fn pick(&mut self, path: &PathBuf) -> Task<M> {
        // A row of a list that has changed since this click was drawn is no row.
        let on_cards = self.app.screen() == Screen::Sources;
        let rows = if on_cards {
            self.app.sources()
        } else {
            self.app.drives()
        };
        let Some(row) = rows.iter().position(|p| p == path) else {
            return Task::none();
        };
        if !self.cursor_to(row) {
            return Task::none();
        }
        if on_cards {
            self.key(Key::Char(' '))
        } else {
            // Next chooses the drive, as Enter does in the TUI (DEC-068 (2)).
            Task::none()
        }
    }

    /// What makes a scrollable's offset belong to something else. The look too: its
    /// sentence has no room to scroll, and the rows after it would open wherever the old
    /// ones were left.
    fn place(&self) -> (Screen, u64, bool) {
        (self.app.screen(), self.app.outcome_id(), self.looking())
    }

    fn smoke_frame(&mut self) -> Task<M> {
        let looking = self.looking();
        let Some(smoke) = &mut self.smoke else {
            return Task::none();
        };
        if looking || smoke.frames == 0 {
            return Task::none();
        }
        smoke.frames -= 1;
        if smoke.frames > 0 {
            return Task::none();
        }
        window::latest()
            .and_then(window::screenshot)
            .map(M::SmokeShot)
    }

    /// Ends by the Quit button's own message, so one real run proves that road (DEC-072 (2)).
    fn smoke_shot(&mut self, shot: &window::Screenshot) -> Task<M> {
        let Some(smoke) = self.smoke.take() else {
            return Task::none();
        };
        if let Err(e) = png::write(&smoke.path, shot.size.width, shot.size.height, &shot.rgba) {
            say(&format!("Cannot write {}: {e}.", smoke.path.display()));
            std::process::exit(2);
        }
        update(self, M::Quit)
    }
}

pub fn update(gui: &mut Gui, message: M) -> Task<M> {
    let before = gui.place();
    let task = step(gui, message);
    if gui.place() == before {
        return task;
    }
    // Batched, not chained: a look's task can wait on a stuck card for ever.
    Task::batch([
        task,
        operation::snap_to(BODY, RelativeOffset::START),
        operation::snap_to(BANNER, RelativeOffset::START),
    ])
}

fn step(gui: &mut Gui, message: M) -> Task<M> {
    let held = gui.looking();
    let screen = gui.app.screen();
    let on_a_list = matches!(screen, Screen::Sources | Screen::Drive);
    match message {
        // Held here too: a click drawn on what was shown before can still arrive. Its key
        // would type in a field, and Esc on the cards would quit.
        M::Row(_) | M::Next | M::LookAgain | M::Quit | M::Back if !on_a_list => Task::none(),
        M::Back if screen != Screen::Drive => Task::none(),
        M::Row(_) | M::Next if held => Task::none(),
        M::Row(path) => gui.pick(&path),
        M::Next => gui.key(Key::Enter),
        M::LookAgain => gui.key(Key::Char('r')),
        M::Quit => gui.key(Key::Char('q')),
        M::Looked => {
            gui.take_answers();
            Task::none()
        }
        M::CloseRequested => gui.key(Key::CtrlC),
        M::SmokeFrame => gui.smoke_frame(),
        M::SmokeShot(shot) => gui.smoke_shot(&shot),
        M::Back => gui.key(Key::Esc),
        #[cfg(test)]
        M::ScrollEndForTest => operation::snap_to_end(BODY),
    }
}

pub fn view(gui: &Gui) -> Element<'_, M> {
    match gui.app.screen() {
        Screen::Sources => cards(gui),
        Screen::Drive => drives(gui),
        // Each comes with its own step, S053 on.
        _ => column![words(TITLE).size(24)].padding(16).into(),
    }
}

fn cards(gui: &Gui) -> Element<'_, M> {
    let app = &gui.app;
    // The next card goes straight to the check, with no drive screen in between.
    let heading = if app.in_session() {
        format!("Sync {}: pick the next card", app.syncs() + 1)
    } else {
        "1/5  Pick the cards to copy from".to_string()
    };
    let rows = app.sources().iter().enumerate().map(|(at, source)| {
        list_row(
            app.is_picked(source),
            at == app.cursor(),
            source.display().to_string(),
            M::Row(source.clone()),
        )
    });
    let next = (!gui.looking()).then_some(M::Next);
    let buttons = row![
        action("Next", next),
        action("Look again", Some(M::LookAgain)),
        action("Quit", Some(M::Quit)),
    ];
    list_screen(gui, heading, rows, buttons)
}

fn drives(gui: &Gui) -> Element<'_, M> {
    let app = &gui.app;
    let rows = app.drives().iter().enumerate().map(|(at, drive)| {
        list_row(
            app.drive() == Some(drive.as_path()),
            at == app.cursor(),
            drive.display().to_string(),
            M::Row(drive.clone()),
        )
    });
    let next = (!gui.looking()).then_some(M::Next);
    let buttons = row![
        action("Next", next),
        action("Back", Some(M::Back)),
        action("Look again", Some(M::LookAgain)),
        action("Quit", Some(M::Quit)),
    ];
    list_screen(
        gui,
        "2/5  Pick the drive to copy to".to_string(),
        rows,
        buttons,
    )
}

/// Rows and notes scroll; all else is fixed, so the banner, the hint and the buttons
/// never leave (DEC-068 (7)). Rows come first, so notes never take their room.
fn list_screen<'a>(
    gui: &'a Gui,
    heading: String,
    rows: impl Iterator<Item = Element<'a, M>>,
    buttons: iced::widget::Row<'a, M>,
) -> Element<'a, M> {
    let mut body = column![].spacing(6).width(Length::Fill);
    if gui.looking() {
        // No rows and no notes: they are of an older look (DEC-073 (2)).
        body = body.push(words(LOOKING));
    } else {
        let mut rows = rows.peekable();
        if rows.peek().is_none() {
            body = body.push(words(gui.app.empty_list_note()));
        }
        for list_row in rows {
            body = body.push(list_row);
        }
        for note in gui.app.notes() {
            body = body.push(words(note.as_str()));
        }
    }
    let mut screen = column![words(TITLE).size(24), words(heading)]
        .spacing(12)
        .padding(16);
    if let Some(reason) = banner(&gui.app.exit(), gui.app.outcome()) {
        let reason = (!reason.is_empty()).then(|| word_box(BANNER, reason, BANNER_MOST));
        screen = screen.push(red_banner(NOT_SAFE, reason));
    }
    screen = screen.push(scrollable(body).id(BODY).spacing(8).height(Length::Fill));
    if let Some(words) = gui.app.hint() {
        screen = screen.push(hint(hint_words(words)));
    }
    screen.push(buttons.spacing(10)).into()
}

/// The model's hints name the TUI's keys; here a key is a button (DEC-068 (3)). A test
/// goes over every hint the screens reach, so a changed hint fails.
fn hint_words(words: &str) -> String {
    words.replace("press r", "click Look again")
}

/// What the banner's box says, and `None` with nothing to warn of (DEC-071 (2)).
pub fn banner(exit: &Exit, outcome: Option<&Outcome>) -> Option<String> {
    if exit.code == 0 {
        return None;
    }
    // One failed sync has no message, and it is the commonest case.
    let words = exit.message.clone().or_else(|| {
        outcome.map(|outcome| match outcome {
            Outcome::Report { result, .. } => result.clone(),
            Outcome::Refused { sentence } => sentence.clone(),
        })
    });
    Some(words.unwrap_or_default())
}

pub fn subscription(gui: &Gui) -> Subscription<M> {
    let closes = window::close_requests().map(|_| M::CloseRequested);
    // Only then: a frame's message makes the next frame, which an idle app must not.
    if gui.smoke.is_some() && !gui.looking() {
        return Subscription::batch([closes, window::frames().map(|_| M::SmokeFrame)]);
    }
    closes
}

/// The program, one for the real window and for the Emulator, so the tests run what ships.
pub fn program(
    discover: Discover,
    smoke_png: Option<PathBuf>,
) -> iced::Application<impl iced::Program<State = Gui, Message = M, Theme = Theme>> {
    let boot = move || Gui::boot(discover.clone(), smoke_png.clone());
    iced::application(boot, update, view)
        .title(|_: &Gui| TITLE.to_string())
        .theme(|_: &Gui| Theme::Dark)
        .default_font(Font::with_name("Fira Sans"))
        .subscription(subscription)
        .window(window::Settings {
            min_size: Some(LEAST),
            ..window::Settings::default()
        })
        .window_size(WINDOW)
        .exit_on_close_request(false)
}
