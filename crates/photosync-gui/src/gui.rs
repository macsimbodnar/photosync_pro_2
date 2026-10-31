//! `Gui`, its messages, and the plain functions iced runs. Each widget sends what the
//! TUI's key does, so every rule stays in `App` (DEC-068 (2)).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use iced::futures::channel::oneshot;
use iced::widget::{column, row, scrollable};
use iced::{Element, Font, Length, Size, Subscription, Task, Theme, window};
use photosync_core::Volumes;
use photosync_front::app::{App, Effect, Key, Msg, Screen};
use photosync_front::system::say;

use crate::png;
use crate::widgets::{action, list_row, words};

/// The name the user sees (DEC-069 (1)).
pub const TITLE: &str = "phoosync pro 2";
/// The window's size, and the least it can be made (DEC-068 (7)).
pub const WINDOW: Size = Size::new(900.0, 650.0);
pub const LEAST: Size = Size::new(640.0, 480.0);
/// What is said while the newest look is out (DEC-073 (2)).
pub const LOOKING: &str = "Looking for cards and drives. A stuck card can hold this up: take it out, or press Look again.";

/// How volumes are found: injected, so tests give literal folders (DEC-068 (5)).
pub type Discover = Arc<dyn Fn() -> Volumes + Send + Sync>;

/// `Msg` is not `Clone` and a button needs that, so what the worker and discovery
/// send stays in channels, and this only says to go and read them (DEC-068 (4)).
#[derive(Debug, Clone)]
pub enum M {
    Row(PathBuf),
    Next,
    LookAgain,
    Quit,
    /// A look's thread has ended.
    Looked,
    CloseRequested,
    /// A frame was drawn, while `--smoke-png` waits for one.
    SmokeFrame,
    SmokeShot(window::Screenshot),
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

    /// The model's keys move the cursor one row at a time, and stop at the ends.
    fn cursor_to(&mut self, row: usize) {
        while self.app.cursor() < row {
            self.app.update(Msg::Key(Key::Down));
        }
        while self.app.cursor() > row {
            self.app.update(Msg::Key(Key::Up));
        }
    }

    fn pick(&mut self, path: &PathBuf) -> Task<M> {
        // A row of a list that has changed since this click was drawn is no row.
        let Some(row) = self.app.sources().iter().position(|p| p == path) else {
            return Task::none();
        };
        self.cursor_to(row);
        self.key(Key::Char(' '))
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
    let held = gui.looking();
    match message {
        // Held in `update` too: a click drawn before the look began can still arrive.
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
    }
}

pub fn view(gui: &Gui) -> Element<'_, M> {
    match gui.app.screen() {
        Screen::Sources => cards(gui),
        // Each comes with its own step, S052 on.
        _ => column![words(TITLE).size(24)].padding(16).into(),
    }
}

fn cards(gui: &Gui) -> Element<'_, M> {
    let looking = gui.looking();
    let mut body = column![].spacing(6).width(Length::Fill);
    if looking {
        // No rows and no notes: they are of an older look (DEC-073 (2)).
        body = body.push(words(LOOKING));
    } else if gui.app.sources().is_empty() {
        body = body.push(words(gui.app.empty_list_note()));
    }
    if !looking {
        for (at, source) in gui.app.sources().iter().enumerate() {
            body = body.push(list_row(
                gui.app.is_picked(source),
                at == gui.app.cursor(),
                source.display().to_string(),
                M::Row(source.clone()),
            ));
        }
        for note in gui.app.notes() {
            body = body.push(words(note.as_str()));
        }
    }
    let buttons = row![
        action("Next", (!looking).then_some(M::Next)),
        action("Look again", Some(M::LookAgain)),
        action("Quit", Some(M::Quit)),
    ]
    .spacing(10);
    column![
        words(TITLE).size(24),
        words("1/5  Pick the cards to copy from"),
        scrollable(body).spacing(8).height(Length::Fill),
        buttons,
    ]
    .spacing(12)
    .padding(16)
    .into()
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
