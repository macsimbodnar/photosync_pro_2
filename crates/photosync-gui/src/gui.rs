//! `Gui`, its messages, and the plain functions iced runs. Each widget sends what the
//! TUI's key does, so every rule stays in `App` (DEC-068 (2)), but for the drive's mark,
//! as a window has no cursor to show what Next takes (DEC-077), and the first start's check,
//! which the window must not wait for (DEC-081 (1)), and "Before you quit" and the refusal
//! before any screen, which the model has no screen for (DEC-069 (3), (7)).

use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use iced::futures::channel::mpsc::{self as wake_channel, UnboundedReceiver, UnboundedSender};
use iced::futures::channel::oneshot;
use iced::widget::scrollable::RelativeOffset;
use iced::widget::{Column, Space, Text, column, lazy, operation, row, scrollable};
use iced::{
    Element, Event, Font, Length, Size, Subscription, Task, Theme, event, keyboard, window,
};
use photosync_core::{Phase, Session, SourceProgress, Volumes};
use photosync_front::app::{App, Effect, Exit, Job, Key, Msg, Outcome, Screen};
use photosync_front::args::EXIT_REFUSED;
use photosync_front::flow::{self, SyncError};
use photosync_front::system::say;
use photosync_front::{text, worker};

use crate::png;
use crate::widgets::{
    action, bar, field, hint, list_row, red_banner, red_box, result_box, under, word_box, words,
};

/// The font the window draws in, and the tests (DEC-065 (4)).
const DEFAULT_FONT: &str = "Fira Sans";
/// The name the user sees (DEC-069 (1)).
pub const TITLE: &str = "phoosync pro 2";
/// The window's size, and the least it can be made (DEC-068 (7)).
pub const WINDOW: Size = Size::new(900.0, 650.0);
pub const LEAST: Size = Size::new(640.0, 480.0);
/// What is said while the newest look is out (DEC-073 (2)).
pub const LOOKING: &str = "Looking for cards and drives. A stuck card can hold this up: take it out, or press Look again.";

/// Why the drive screen's Next is held (DEC-077).
pub const PICK_A_DRIVE: &str = "Click the drive to copy to.";
pub const DRIVE_GONE: &str =
    "The drive you picked is gone. Click another, or plug it in and click Look again.";

/// The words the TUI's view holds, with no key in them (DEC-068 (7)).
pub const LIBRARY_HELP: [&str; 2] = [
    "The folder on the drive that holds your photos. It must exist:",
    "phoosync pro 2 never creates it. Photos already anywhere in it are skipped.",
];
pub const NAME_HELP: &str =
    "A folder with this name from an earlier sync is reused. Empty: no name.";
pub const CARDS_ONLY_READ: &str =
    "The cards are only read. Photos already in the library are skipped.";
pub const THE_DATE_TO_COME: &str = "(the date of the earliest new photo)";
pub const THE_SESSIONS_FOLDER: &str = "(this session's folder)";

/// The copying screen's fixed sentences (DEC-069 (4), (5), DEC-071 (1)).
pub const DO_NOT_FORMAT: &str = "Do not format the card until the report says so.";
pub const CANCELLING: &str =
    "Cancelling: the files being copied are finished or dropped, then the report shows.";
pub const CLOSE_AGAIN: &str = "Close the window again to quit at once: not every file will be known to be on the drive. Do not format the card.";
/// What the copying screen's body says until the first progress.
pub const SCANNING: &str = "scanning the cards...";

/// The TUI's title for the report.
pub const REPORT_HEADING: &str = "Report";

/// What is said while the first start's check is out (DEC-081 (1)).
pub const CHECKING: &str =
    "Checking the library folder. A stuck drive can hold this up: click Back, or take it out.";

/// The banner's line (DEC-071 (2)).
pub const NOT_SAFE: &str = "Not every photo is known to be on the drive: do not format the card.";
/// The screen before a quit that would hide a session not complete (DEC-069 (3)).
pub const BEFORE_YOU_QUIT: &str = "Before you quit";
pub const QUIT_ANYWAY: &str = "Quit anyway";
/// The scrollables' fixed ids, as `Scrollable::diff` never looks at an id (DEC-073 (1)).
pub const BODY: &str = "body";
pub const BANNER: &str = "banner";
pub const RESULT: &str = "result";
pub const FIELD: &str = "field";
/// The most the banner's box takes: five rows (DEC-071 (3)).
const BANNER_MOST: f32 = 104.0;
/// What the result's words take of its 240, with `result_box`'s padding (DEC-071 (4), DEC-082 (1)).
const RESULT_MOST: f32 = 224.0;

/// How volumes are found: injected, so tests give literal folders (DEC-068 (5)).
pub type Discover = Arc<dyn Fn() -> Volumes + Send + Sync>;
/// What the first start checks with: `flow::start_found`, or what a test holds as a stuck
/// drive would be (DEC-081 (1)).
pub(crate) type Check =
    Arc<dyn Fn(&Path, Option<&str>) -> Result<Session, SyncError> + Send + Sync>;

/// What each quit leaves for `run`, as iced keeps the state and gives back only a `Result`.
pub type ExitSlot = Arc<Mutex<Option<Exit>>>;

/// What a run opens on: a look, or a sentence and Quit (DEC-069 (7)).
#[derive(Clone)]
pub enum Opening {
    Look(Discover),
    Refuse(String),
}

/// `Msg` is not `Clone` and a button needs that, so what the worker and discovery
/// send stays in channels, and this only says to go and read them (DEC-068 (4)).
#[derive(Debug, Clone)]
pub enum M {
    Row(PathBuf),
    /// All a field holds now, after a key or a paste, and the screen of that field. iced
    /// handles a tick's events on the widgets drawn before them, so a key after Enter comes
    /// from the field just left, with all that field held (DEC-081 (3)).
    Field(Screen, String),
    /// Esc was pressed, wherever it went (DEC-081 (4)).
    Escape,
    Next,
    /// Its own message, never `Next`: a click left from the name screen must start nothing.
    Start,
    Back,
    LookAgain,
    Quit,
    /// Its own message, never `Quit`: a click left from the screen before must not pass this
    /// one (DEC-069 (3)).
    QuitAnyway,
    /// Back from "Before you quit": its own, as a Back left over must not reach the model.
    NotYet,
    /// A look's thread has ended.
    Looked,
    /// The first start's check has ended (DEC-081 (1)).
    Checked,
    CloseRequested,
    /// A frame was drawn, while `--smoke-png` waits for one.
    SmokeFrame,
    SmokeShot(window::Screenshot),
    Cancel,
    /// Its own message, never `Next`: a click left from another screen must not look again.
    NextCard,
    /// The worker has sent something: go and read it (DEC-068 (4)).
    Wake,
    /// The worker's sender is gone: what is left, then its end if none came.
    Gone,
    /// A wheel to the body's end: the Emulator has no wheel (DEC-073 (1)).
    #[cfg(test)]
    ScrollEndForTest,
    /// Reaches the state the Emulator holds.
    #[cfg(test)]
    ForTest(ForTest),
}

/// Called on the copy thread before each real send, so a test can hold it (DEC-072 (6)).
#[cfg(test)]
pub(crate) type BeforeSend = Arc<dyn Fn(&Msg) + Send + Sync>;
/// Runs in place of `worker::spawn`, for an end no real sync gives.
#[cfg(test)]
pub(crate) type TestWorker = Arc<dyn Fn(Job, WorkerSend) + Send + Sync>;

#[cfg(test)]
#[derive(Clone)]
pub struct ForTest(pub(crate) Arc<dyn Fn(&mut Gui) + Send + Sync>);

#[cfg(test)]
impl std::fmt::Debug for ForTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ForTest")
    }
}

/// What the worker's thread sends with, on a copy thread, under the core's progress lock
/// (DEC-040 (3)), so nothing here may block. Clones are for a test's threads.
#[derive(Clone)]
pub(crate) struct WorkerSend {
    // Before `wake`, as fields drop in order: once the wake's stream has ended the channel
    // reads as gone, so `M::Gone` can tell the worker's end from a stale `M::Gone`.
    tx: Sender<Msg>,
    wake: UnboundedSender<()>,
    pending: Arc<AtomicBool>,
    #[cfg(test)]
    before_send: Option<BeforeSend>,
}

impl WorkerSend {
    pub(crate) fn send(&self, msg: Msg) {
        #[cfg(test)]
        if let Some(before) = &self.before_send {
            before(&msg);
        }
        let _ = self.tx.send(msg);
        // A wake is out already: its drain reads this too, so no flood of redraws.
        if !self.pending.swap(true, Ordering::AcqRel) {
            let _ = self.wake.unbounded_send(());
        }
    }
}

/// The window's end of a `WorkerSend`.
struct Hearing {
    messages: Receiver<Msg>,
    pending: Arc<AtomicBool>,
}

/// How much of a bar is full: all of it with nothing to do, as the words say 100%.
pub(crate) fn share(done: u64, total: u64) -> f32 {
    if total == 0 {
        return 1.0;
    }
    (done as f64 / total as f64).min(1.0) as f32
}

/// A card's row under its bar: the TUI's, as its words are.
pub(crate) fn card_line(card: &SourceProgress) -> String {
    let name = card.root.file_name().map_or_else(
        || card.root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    format!(
        "{name}  {}/{} files, {} of {}",
        card.done.files,
        card.total.files,
        text::size(card.done.bytes),
        text::size(card.total.bytes)
    )
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
    /// The model said to quit. iced's exit cannot be seen by a test. From here on `update`
    /// does nothing.
    pub quit: bool,
    pub(crate) exit_slot: ExitSlot,
    /// A run that must not go on: all it shows is this sentence and Quit (DEC-069 (7)).
    refused_to_run: Option<String>,
    /// "Before you quit" stands over the model's screen (DEC-069 (3)).
    before_quit: bool,
    smoke: Option<Smoke>,
    /// The drive picked on the drive screen: what `marked_drive` shows and Next takes. The
    /// model hears only on Next, as its library goes with the drive, and clicks there and
    /// back are no change of drive.
    picked_drive: Option<PathBuf>,
    /// A picked drive a look did not find, until the next click. Nothing stands in for it.
    lost_drive: Option<PathBuf>,
    /// The running sync's messages, or the last one's until its sender has gone.
    hearing: Option<Hearing>,
    pub(crate) check: Check,
    /// The number of the latest check started, and the one the screen waits for, if any.
    newest_check: u64,
    waited_check: Option<u64>,
    checks_tx: Sender<(u64, Result<Session, SyncError>)>,
    checks: Receiver<(u64, Result<Session, SyncError>)>,
    /// Answers dropped as out of date: a test sees it.
    #[cfg(test)]
    pub(crate) dropped_checks: u64,
    #[cfg(test)]
    pub(crate) before_send: Option<BeforeSend>,
    #[cfg(test)]
    pub(crate) test_worker: Option<TestWorker>,
    /// How many answers the checks' threads have sent, so a test knows one is waiting.
    #[cfg(test)]
    pub(crate) checks_answered: Arc<AtomicU64>,
    /// How many `M::Wake`s came, to count against the messages.
    #[cfg(test)]
    pub(crate) wakes: u64,
    /// How many syncs began: a test sees that a click started none.
    #[cfg(test)]
    pub(crate) syncs_started: u64,
}

impl Gui {
    /// The state, and the task of the first look.
    pub fn boot(discover: Discover, smoke_png: Option<PathBuf>) -> (Gui, Task<M>) {
        Gui::open(Opening::Look(discover), smoke_png, ExitSlot::default())
    }

    /// `boot`, on what the run opens on, leaving each quit's `Exit` in `exit_slot`.
    pub fn open(
        opening: Opening,
        smoke_png: Option<PathBuf>,
        exit_slot: ExitSlot,
    ) -> (Gui, Task<M>) {
        let (discover, refused_to_run): (Discover, _) = match opening {
            Opening::Look(discover) => (discover, None),
            // Never called, as nothing here looks.
            Opening::Refuse(sentence) => (Arc::new(Volumes::default), Some(sentence)),
        };
        let (answers_tx, answers) = mpsc::channel();
        let (checks_tx, checks) = mpsc::channel();
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
            exit_slot,
            refused_to_run,
            before_quit: false,
            // Two: the first may be of the frame before the answer.
            smoke: smoke_png.map(|path| Smoke { path, frames: 2 }),
            picked_drive: None,
            lost_drive: None,
            hearing: None,
            check: Arc::new(flow::start_found),
            newest_check: 0,
            waited_check: None,
            checks_tx,
            checks,
            #[cfg(test)]
            dropped_checks: 0,
            #[cfg(test)]
            before_send: None,
            #[cfg(test)]
            test_worker: None,
            #[cfg(test)]
            checks_answered: Arc::new(AtomicU64::new(0)),
            #[cfg(test)]
            wakes: 0,
            #[cfg(test)]
            syncs_started: 0,
        };
        let task = if gui.refused_to_run.is_some() {
            Task::none()
        } else {
            gui.look()
        };
        (gui, task)
    }

    /// The one marked row, and what Next takes. With nothing picked, the only drive listed
    /// has no choice to see, unless it is left after a picked one went (DEC-077).
    pub fn marked_drive(&self) -> Option<&Path> {
        if let Some(picked) = &self.picked_drive {
            return Some(picked);
        }
        match self.app.drives() {
            [only] if self.lost_drive.is_none() => Some(only),
            _ => None,
        }
    }

    /// Why the drive screen's Next is held. An empty list has its own sentence, and its
    /// Next says why as the cards' does, so that is no hold, a drive gone or not.
    fn next_held_because(&self) -> Option<&'static str> {
        let drives = self.app.drives();
        let held = self.app.screen() == Screen::Drive
            && !self.looking()
            && !drives.is_empty()
            && self.marked_drive().is_none();
        if !held {
            return None;
        }
        match &self.lost_drive {
            Some(lost) if !drives.contains(lost) => Some(DRIVE_GONE),
            _ => Some(PICK_A_DRIVE),
        }
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
                if self
                    .picked_drive
                    .as_ref()
                    .is_some_and(|picked| !self.app.drives().contains(picked))
                {
                    self.lost_drive = self.picked_drive.take();
                }
            } else {
                self.dropped_looks += 1;
            }
        }
    }

    /// The first start's check is out: Start is held, and all else stays live.
    pub fn checking(&self) -> bool {
        self.waited_check.is_some()
    }

    /// The first start's check, on its own thread, as a stuck drive must not hold the window.
    /// One that never ends holds nothing the app needs, as a look does not (DEC-081 (1)).
    fn ask_check(&mut self, library: PathBuf, name: Option<String>) -> Task<M> {
        self.newest_check += 1;
        let number = self.newest_check;
        self.waited_check = Some(number);
        let check = self.check.clone();
        let answers = self.checks_tx.clone();
        let (ended, heard) = oneshot::channel::<()>();
        #[cfg(test)]
        let answered = self.checks_answered.clone();
        thread::spawn(move || {
            let answer = check(&library, name.as_deref());
            let _ = answers.send((number, answer));
            #[cfg(test)]
            answered.fetch_add(1, Ordering::SeqCst);
            let _ = ended.send(());
        });
        Task::perform(heard, |_| M::Checked)
    }

    /// Only the answer the screen waits for reaches the model: one from before a Back is of a
    /// library the screen may no longer name, and must neither start nor refuse.
    fn take_check_answers(&mut self) -> Task<M> {
        let mut task = Task::none();
        while let Ok((number, answer)) = self.checks.try_recv() {
            if self.waited_check != Some(number) {
                #[cfg(test)]
                {
                    self.dropped_checks += 1;
                }
                continue;
            }
            self.waited_check = None;
            if let Some(Effect::Start(job)) = self.app.first_start_answered(answer) {
                task = self.run(job);
            }
        }
        task
    }

    /// Start copying: by the check's thread, if the model would run one on this.
    fn start(&mut self) -> Task<M> {
        match self.app.first_start() {
            Some((library, name)) => self.ask_check(library, name),
            None => self.key(Key::Enter),
        }
    }

    /// A new channel for a sync, with the wake's stream for `Task::run` (DEC-068 (4)).
    pub(crate) fn wire(&mut self) -> (WorkerSend, UnboundedReceiver<()>) {
        let (tx, messages) = mpsc::channel();
        let (wake, woken) = wake_channel::unbounded();
        let pending = Arc::new(AtomicBool::new(false));
        self.hearing = Some(Hearing {
            messages,
            pending: pending.clone(),
        });
        let send = WorkerSend {
            tx,
            wake,
            pending,
            #[cfg(test)]
            before_send: self.before_send.clone(),
        };
        (send, woken)
    }

    /// The sync, on the worker's thread. When that ends the sender drops, so the stream ends
    /// and `M::Gone` says so, panic or not.
    fn run(&mut self, job: Job) -> Task<M> {
        let (send, woken) = self.wire();
        #[cfg(test)]
        {
            self.syncs_started += 1;
        }
        #[cfg(test)]
        if let Some(test_worker) = &self.test_worker {
            test_worker(job, send);
            return Task::run(woken, |()| M::Wake).chain(Task::done(M::Gone));
        }
        worker::spawn(job, move |msg| send.send(msg));
        Task::run(woken, |()| M::Wake).chain(Task::done(M::Gone))
    }

    /// All the worker has sent. A snapshot is whole, so only the last one drained shows.
    /// `Msg::Finished` takes the session's lock, which the worker let go before sending it.
    /// The channel gone is `Msg::WorkerGone`, which does nothing after an end. A late `M::Gone`
    /// reads the next sync's channel, which is not gone, so it cannot end that sync.
    fn hear(&mut self) {
        let Some(hearing) = &self.hearing else {
            return;
        };
        // First: a message sent after the last read must wake again.
        hearing.pending.store(false, Ordering::Release);
        loop {
            match hearing.messages.try_recv() {
                Ok(msg) => {
                    self.app.update(msg);
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.app.update(Msg::WorkerGone);
                    self.hearing = None;
                    return;
                }
            }
        }
    }

    /// A quit, unless it would hide a session not complete (DEC-069 (3)).
    fn quit_asked(&mut self) -> Task<M> {
        if warns_first(self.app.screen(), self.app.exit().code) {
            self.before_quit = true;
            return Task::none();
        }
        self.quit_now()
    }

    /// iced gives `run` nothing back from the state, so the `Exit` goes where it can read it.
    fn quit_now(&mut self) -> Task<M> {
        self.quit = true;
        let exit = if self.refused_to_run.is_some() {
            // Its sentence went to stderr before the window opened.
            Exit {
                code: EXIT_REFUSED,
                report: None,
                message: None,
            }
        } else {
            self.app.exit()
        };
        *self
            .exit_slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(exit);
        iced::exit()
    }

    fn key(&mut self, key: Key) -> Task<M> {
        match self.app.update(Msg::Key(key)) {
            None => Task::none(),
            Some(Effect::Discover) => self.look(),
            Some(Effect::Quit) => self.quit_asked(),
            Some(Effect::Start(job)) => self.run(job),
        }
    }

    /// A field gives all it holds; the model hears it as keys, so `edit` still decides what
    /// a field can hold (DEC-068 (2)).
    fn retype(&mut self, new: &str) {
        let old = match self.app.screen() {
            Screen::Library => self.app.library(),
            _ => self.app.name(),
        };
        for _ in 0..old.chars().count() {
            self.app.update(Msg::Key(Key::Backspace));
        }
        for c in new.chars() {
            self.app.update(Msg::Key(Key::Char(c)));
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
            self.picked_drive = Some(path.clone());
            self.lost_drive = None;
            Task::none()
        }
    }

    /// Next on the drive screen takes the marked row. Anything else is unseen: the second
    /// click of a double on the cards' Next lands here, drawn before this list was.
    fn take_marked_drive(&mut self) -> Task<M> {
        let Some(marked) = self.marked_drive().map(Path::to_path_buf) else {
            if self.app.drives().is_empty() {
                return self.key(Key::Enter);
            }
            return Task::none();
        };
        let row = self.app.drives().iter().position(|d| *d == marked);
        if !row.is_some_and(|row| self.cursor_to(row)) {
            return Task::none();
        }
        self.picked_drive = Some(marked);
        self.key(Key::Enter)
    }

    /// "Before you quit" is up.
    pub fn before_quit(&self) -> bool {
        self.before_quit
    }

    /// What makes a scrollable's offset belong to something else. The look too: its
    /// sentence has no room to scroll, and the rows after it would open wherever the old
    /// ones were left.
    fn place(&self) -> (Screen, u64, bool, bool) {
        (
            self.app.screen(),
            self.app.outcome_id(),
            self.looking(),
            self.before_quit,
        )
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
    // iced_winit updates every message of a batch before any exit action, so one can come after
    // a quit: the check's answer would start a copy with the window going away.
    if gui.quit {
        return Task::none();
    }
    let before = gui.place();
    let task = step(gui, message);
    // The check was asked for by the screen left. "Before you quit" stands over the model's
    // check: the press is taken back there too, or an answer behind it would start a copy.
    if gui.app.screen() != Screen::Confirm || gui.before_quit {
        gui.waited_check = None;
    }
    if gui.place() == before {
        return task;
    }
    // Batched, not chained: a look's task can wait on a stuck card for ever.
    Task::batch([
        task,
        operation::snap_to(BODY, RelativeOffset::START),
        operation::snap_to(BANNER, RelativeOffset::START),
        operation::snap_to(RESULT, RelativeOffset::START),
        // Typing and Enter need it, and the focus also puts the cursor at the end. A screen
        // with no field has no such id, so this does nothing there.
        operation::focus(FIELD),
    ])
}

fn step(gui: &mut Gui, message: M) -> Task<M> {
    if gui.refused_to_run.is_some() {
        return match message {
            M::Quit | M::CloseRequested => gui.quit_now(),
            M::SmokeFrame => gui.smoke_frame(),
            M::SmokeShot(shot) => gui.smoke_shot(&shot),
            _ => Task::none(),
        };
    }
    if gui.before_quit {
        match message {
            M::QuitAnyway | M::CloseRequested => return gui.quit_now(),
            M::NotYet => {
                gui.before_quit = false;
                return Task::none();
            }
            // Answers to what was asked before: each goes on behind this screen.
            M::Looked | M::Checked | M::Wake | M::Gone | M::SmokeFrame | M::SmokeShot(_) => {}
            #[cfg(test)]
            M::ScrollEndForTest | M::ForTest(_) => {}
            // A click drawn on the screen before: behind this one it would start or type.
            _ => return Task::none(),
        }
    }
    let held = gui.looking();
    let screen = gui.app.screen();
    let on_a_list = matches!(screen, Screen::Sources | Screen::Drive);
    let on_a_field = matches!(screen, Screen::Library | Screen::Name);
    match message {
        // Held here too: a click drawn on what was shown before can still arrive. Its key
        // would type in a field, Esc on the cards would quit, and Enter on the check would
        // start, so only the screen's own buttons get through (DEC-075 (1), DEC-079 (1)).
        M::Row(_) | M::LookAgain if !on_a_list => Task::none(),
        M::Next if !on_a_list && !on_a_field => Task::none(),
        M::Field(..) | M::Escape if !on_a_field => Task::none(),
        M::Field(of, _) if of != screen => Task::none(),
        M::Start if screen != Screen::Confirm || gui.checking() => Task::none(),
        M::Quit
            if !on_a_list
                && !matches!(screen, Screen::Confirm | Screen::Refused | Screen::Report) =>
        {
            Task::none()
        }
        M::NextCard
            if screen != Screen::Report && !(screen == Screen::Refused && gui.app.in_session()) =>
        {
            Task::none()
        }
        M::Back if matches!(screen, Screen::Sources | Screen::Running | Screen::Report) => {
            Task::none()
        }
        // Its key would type in a field.
        M::Cancel if screen != Screen::Running => Task::none(),
        M::Row(_) | M::Next if held => Task::none(),
        M::Row(path) => gui.pick(&path),
        M::Field(_, value) => {
            gui.retype(&value);
            Task::none()
        }
        M::Next if screen == Screen::Drive => gui.take_marked_drive(),
        M::Next => gui.key(Key::Enter),
        M::Start => gui.start(),
        M::LookAgain => gui.key(Key::Char('r')),
        M::Quit => gui.key(Key::Char('q')),
        M::QuitAnyway | M::NotYet => Task::none(),
        M::Looked => {
            gui.take_answers();
            Task::none()
        }
        M::Checked => gui.take_check_answers(),
        M::CloseRequested => gui.key(Key::CtrlC),
        M::SmokeFrame => gui.smoke_frame(),
        M::SmokeShot(shot) => gui.smoke_shot(&shot),
        M::Back | M::Escape => gui.key(Key::Esc),
        M::Cancel => gui.key(Key::Char('c')),
        M::NextCard => gui.key(Key::Char('n')),
        M::Wake => {
            #[cfg(test)]
            {
                gui.wakes += 1;
            }
            gui.hear();
            Task::none()
        }
        M::Gone => {
            gui.hear();
            Task::none()
        }
        #[cfg(test)]
        M::ScrollEndForTest => operation::snap_to_end(BODY),
        #[cfg(test)]
        M::ForTest(reach) => {
            (reach.0)(gui);
            Task::none()
        }
    }
}

pub fn view(gui: &Gui) -> Element<'_, M> {
    if let Some(sentence) = &gui.refused_to_run {
        return refused_to_run_screen(sentence);
    }
    if gui.before_quit {
        return before_you_quit_screen(&gui.app.exit(), gui.app.outcome());
    }
    match gui.app.screen() {
        Screen::Sources => cards(gui),
        Screen::Drive => drives(gui),
        Screen::Library => library(gui),
        Screen::Name => name(gui),
        Screen::Confirm => check(gui),
        Screen::Refused => refused(gui),
        Screen::Running => copying(gui),
        Screen::Report => match gui.app.outcome() {
            Some(Outcome::Report {
                lines,
                result,
                complete,
            }) => report_screen(gui.app.outcome_id(), lines, result, *complete),
            // The model sets the two together.
            _ => column![words(TITLE).size(24)].padding(16).into(),
        },
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
    let marked = gui.marked_drive();
    let rows = app.drives().iter().map(|drive| {
        // One mark: the tick and the bar are never on two rows.
        let here = marked == Some(drive.as_path());
        list_row(
            here,
            here,
            drive.display().to_string(),
            M::Row(drive.clone()),
        )
    });
    let can_go = !gui.looking() && (marked.is_some() || app.drives().is_empty());
    let next = can_go.then_some(M::Next);
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

fn library(gui: &Gui) -> Element<'_, M> {
    let body = column![
        field(
            FIELD,
            "Library folder",
            gui.app.library(),
            |value| M::Field(Screen::Library, value),
            M::Next
        ),
        words(LIBRARY_HELP[0]),
        words(LIBRARY_HELP[1]),
    ];
    field_screen(gui, "3/5  Library folder on the drive", body)
}

fn name(gui: &Gui) -> Element<'_, M> {
    let app = &gui.app;
    let goes_to = format!(
        "New photos go to {}/{}/RAW, JPEG, ...",
        app.library().trim(),
        app.folder_preview()
    );
    let body = column![
        field(
            FIELD,
            "No name",
            app.name(),
            |value| M::Field(Screen::Name, value),
            M::Next
        ),
        words(goes_to),
        words(NAME_HELP),
    ];
    field_screen(gui, "4/5  Name this sync", body)
}

/// No Quit: its key would type, and a close of the window is there (DEC-068 (2)). The field
/// is in the body: fixed, it could not leave room under a five-row banner at the least size.
fn field_screen<'a>(gui: &'a Gui, heading: &'a str, body: Column<'a, M>) -> Element<'a, M> {
    let buttons = row![action("Next", Some(M::Next)), action("Back", Some(M::Back)),];
    fixed_around(gui, heading.to_string(), body.spacing(8), buttons)
}

fn check(gui: &Gui) -> Element<'_, M> {
    let app = &gui.app;
    let in_session = app.in_session();
    let heading = if in_session {
        format!("Sync {}: check and start", app.syncs() + 1)
    } else {
        "5/5  Check and start".to_string()
    };
    let cards = app.picked().iter().fold(column![], |list, card| {
        list.push(words(card.display().to_string()))
    });
    let fixed = if app.session_folder().is_some() {
        THE_SESSIONS_FOLDER
    } else {
        THE_DATE_TO_COME
    };
    let mut body = column![
        under("Copy from:", cards.spacing(2)),
        under("Into the library:", column![words(app.library().trim())]),
        under(
            "New photos go to the folder:",
            column![words(format!("{}   {fixed}", app.folder_preview()))],
        ),
    ]
    .spacing(14);
    if in_session {
        body = body.push(words(format!(
            "Sync {} of this session: the library and name stay as they are. Click Quit to change them.",
            app.syncs() + 1
        )));
    }
    body = body.push(words(CARDS_ONLY_READ));
    // Start copying stands apart from where Next and Back, to fix it were: the second click
    // of a double must not start a copy past this screen, or quit (DEC-079 (2)).
    let buttons = row![
        action("Back", Some(M::Back)),
        Space::new().width(Length::Fill),
        action("Quit", Some(M::Quit)),
        action("Start copying", (!gui.checking()).then_some(M::Start)),
    ];
    fixed_around(gui, heading, body, buttons)
}

#[cfg(test)]
thread_local! {
    /// How many times the report's lines were made, on this thread (DEC-068 (6)).
    pub(crate) static REPORT_MADE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The report, from what it is given, so a test can give what no sync on temp folders does
/// (DEC-067 (5)). `outcome_id` is what the lines are kept by, as a real report can be 50000 of
/// them (DEC-068 (6)). The result and the buttons are fixed, the report the body. The buttons
/// stand mid-row: the second click of a double on Start copying, at the right, or on Cancel, at
/// the left, must not take a report away before it is read (DEC-079 (2), DEC-082 (2)).
pub fn report_screen<'a>(
    outcome_id: u64,
    lines: &'a [String],
    result: &str,
    complete: bool,
) -> Element<'a, M> {
    let over = column![
        words(TITLE).size(24),
        words(REPORT_HEADING),
        result_box(RESULT, result.to_string(), RESULT_MOST, complete),
    ]
    .spacing(12);
    let body = column![lazy(outcome_id, move |_| report_lines(lines))];
    let buttons = row![
        Space::new().width(Length::Fill),
        action("Next card", Some(M::NextCard)),
        action("Quit", Some(M::Quit)),
        Space::new().width(Length::Fill),
    ]
    .spacing(10);
    around_the_body(over, body, column![buttons])
}

/// One text a line, or the layout of all of them comes again on each scroll (DEC-068 (6)), and
/// a fixed-width font, or the tables do not line up. They own their words, as `lazy` keeps them.
fn report_lines(lines: &[String]) -> Column<'static, M> {
    #[cfg(test)]
    REPORT_MADE.with(|made| made.set(made.get() + 1));
    Column::with_children(lines.iter().map(|line| report_line(line).into())).width(Length::Fill)
}

/// The fixed font's fallback gives a kanji a whole row and draws nothing, so `写真` would be lost
/// from a card's name with nothing wrong in its bounds. A table is all ASCII, so nothing that
/// must line up leaves the fixed font (DEC-082 (3)).
fn report_line(line: &str) -> Text<'static> {
    let font = if line.is_ascii() {
        Font::MONOSPACE
    } else {
        Font::with_name(DEFAULT_FONT)
    };
    words(line.to_string()).font(font)
}

/// The refusal's sentence is the body, so however long it is the buttons stay. It says
/// what it says by itself, so no banner (DEC-071 (2), (4)). Next card stands apart from the check's
/// Start copying and the cards' Quit, so the second click of a double lands on neither way out
/// of this sentence (DEC-079 (2), DEC-082 (2)).
fn refused(gui: &Gui) -> Element<'_, M> {
    let sentence = match gui.app.outcome() {
        Some(Outcome::Refused { sentence }) => sentence.as_str(),
        _ => "",
    };
    let mut buttons = row![
        action("Back, to fix it", Some(M::Back)),
        action("Quit", Some(M::Quit)),
    ];
    if gui.app.in_session() {
        buttons = buttons
            .push(Space::new().width(Length::Fill))
            .push(action("Next card", Some(M::NextCard)))
            .push(Space::new().width(Length::Fill));
    }
    fixed_around(
        gui,
        "Stopped, no report".to_string(),
        column![red_box(sentence)],
        buttons,
    )
}

/// The warnings, the overall bar and Cancel are fixed; the words, the cards' bars and the scan
/// lines scroll, as ten cards' bars cannot fit (DEC-071 (1)). No banner, as this says it itself,
/// and no Quit (DEC-068 (2)). Cancel stands left, where the check's Start copying is not, so the
/// second click of a double does not cancel what it started (DEC-079 (2)).
fn copying(gui: &Gui) -> Element<'_, M> {
    let run = gui.app.running();
    let progress = run.and_then(|run| run.progress.as_ref());
    let cancelling = run.is_some_and(|run| run.cancelling);
    let copying_now = progress.is_some_and(|p| p.phase == Phase::Copying);
    let heading = if copying_now {
        "Copying"
    } else {
        "Getting ready to copy"
    };
    let mut over = column![words(TITLE).size(24), words(heading), hint(DO_NOT_FORMAT)].spacing(12);
    if cancelling {
        over = over.push(hint(CANCELLING)).push(hint(CLOSE_AGAIN));
    }
    let mut body = column![].spacing(6);
    match progress {
        None => body = body.push(words(SCANNING)),
        Some(p) => {
            over = over.push(bar(share(p.done.bytes, p.total.bytes)));
            body = body.push(words(text::progress_line(p)));
            if p.sources.len() > 1 {
                for card in &p.sources {
                    body = body.push(words(card_line(card)));
                    body = body.push(bar(share(card.done.bytes, card.total.bytes)));
                }
            }
        }
    }
    for line in run.iter().flat_map(|run| &run.scanned) {
        body = body.push(words(line.as_str()));
    }
    let cancel = (!cancelling).then_some(M::Cancel);
    let under = column![row![action("Cancel", cancel)]];
    around_the_body(over, body, under)
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
    fixed_around(gui, heading, body, buttons)
}

/// What every screen but the copying one keeps out of the scroll.
fn fixed_around<'a>(
    gui: &'a Gui,
    heading: String,
    body: Column<'a, M>,
    buttons: iced::widget::Row<'a, M>,
) -> Element<'a, M> {
    let mut over = column![words(TITLE).size(24), words(heading)].spacing(12);
    // The refused screen says it itself (DEC-071 (2)).
    let warns = gui.app.screen() != Screen::Refused;
    if let Some(reason) = banner(&gui.app.exit(), gui.app.outcome()).filter(|_| warns) {
        let reason = (!reason.is_empty()).then(|| word_box(BANNER, reason, BANNER_MOST));
        over = over.push(red_banner(NOT_SAFE, reason));
    }
    let mut under = column![].spacing(12);
    if let Some(why) = gui.next_held_because() {
        under = under.push(hint(why));
    }
    if gui.checking() {
        under = under.push(hint(CHECKING));
    }
    if let Some(words) = gui.app.hint() {
        under = under.push(hint(hint_words(words)));
    }
    around_the_body(over, body, under.push(buttons.spacing(10)))
}

/// The one body, with what stays in sight over and under it. Taking all the height left in a
/// column is what lets `fixed_in_sight` trust what it sees, so no screen places it another way,
/// and none bounds a height elsewhere (`tests/gui_split.rs`, DEC-080).
fn around_the_body<'a>(
    over: Column<'a, M>,
    body: Column<'a, M>,
    under: Column<'a, M>,
) -> Element<'a, M> {
    let body = scrollable(body.width(Length::Fill))
        .id(BODY)
        .spacing(8)
        .height(Length::Fill);
    column![over, body, under].spacing(12).padding(16).into()
}

/// The model's hints name the TUI's keys; here a key is a button (DEC-068 (3)). A test
/// goes over every hint the screens reach, so a changed hint fails.
fn hint_words(words: &str) -> String {
    words
        .replace("press r", "click Look again")
        .replace(" (Esc)", "")
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

/// Whether a quit first says why: not with a session complete, and not from a screen that says
/// it itself (DEC-069 (3)).
pub fn warns_first(screen: Screen, code: u8) -> bool {
    code != 0 && !matches!(screen, Screen::Report | Screen::Refused | Screen::Running)
}

/// "Before you quit", from what it is given, as only images reach a real report. The line stays
/// while the reason scrolls. Quit anyway stands at the far right, past every screen's Quit: the
/// second click of a double on one lands here, and there is no second chance after it
/// (DEC-079 (2)).
pub fn before_you_quit_screen<'a>(exit: &Exit, outcome: Option<&Outcome>) -> Element<'a, M> {
    let over = column![
        words(TITLE).size(24),
        words(BEFORE_YOU_QUIT),
        red_banner(NOT_SAFE, None),
    ]
    .spacing(12);
    let mut body = column![];
    if let Some(reason) = banner(exit, outcome).filter(|reason| !reason.is_empty()) {
        body = body.push(red_box(reason));
    }
    let buttons = row![
        action("Back", Some(M::NotYet)),
        Space::new().width(Length::Fill),
        action(QUIT_ANYWAY, Some(M::QuitAnyway)),
    ]
    .spacing(10);
    around_the_body(over, body, column![buttons])
}

/// All a run refused before it began shows: the sentence, and Quit (DEC-069 (7)).
fn refused_to_run_screen(sentence: &str) -> Element<'_, M> {
    let buttons = row![action("Quit", Some(M::Quit))];
    around_the_body(column![], column![red_box(sentence)], column![buttons])
}

/// What a press of Esc says. Not `keyboard::listen`, which hears only what no widget took: a
/// focused field takes Esc, and only loses its focus. A held key repeats, and must not walk
/// back past the screen it was pressed on (DEC-081 (4)).
fn escape_of(event: Event, _status: event::Status, _window: window::Id) -> Option<M> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            repeat: false,
            ..
        }) => Some(M::Escape),
        _ => None,
    }
}

pub fn subscription(gui: &Gui) -> Subscription<M> {
    let closes = window::close_requests().map(|_| M::CloseRequested);
    let escapes = event::listen_with(escape_of);
    // Only then: a frame's message makes the next frame, which an idle app must not.
    if gui.smoke.is_some() && !gui.looking() {
        return Subscription::batch([closes, escapes, window::frames().map(|_| M::SmokeFrame)]);
    }
    Subscription::batch([closes, escapes])
}

/// The program, one for the real window and for the Emulator, so the tests run what ships.
pub fn program(
    discover: Discover,
    smoke_png: Option<PathBuf>,
) -> iced::Application<impl iced::Program<State = Gui, Message = M, Theme = Theme>> {
    program_of(Opening::Look(discover), smoke_png, ExitSlot::default())
}

/// `program`, on what the run opens on, leaving each quit's `Exit` in `exit_slot`.
pub fn program_of(
    opening: Opening,
    smoke_png: Option<PathBuf>,
    exit_slot: ExitSlot,
) -> iced::Application<impl iced::Program<State = Gui, Message = M, Theme = Theme>> {
    let boot = move || Gui::open(opening.clone(), smoke_png.clone(), exit_slot.clone());
    iced::application(boot, update, view)
        .title(|_: &Gui| TITLE.to_string())
        .theme(|_: &Gui| Theme::Dark)
        .default_font(Font::with_name(DEFAULT_FONT))
        .subscription(subscription)
        .window(window::Settings {
            min_size: Some(LEAST),
            ..window::Settings::default()
        })
        .window_size(WINDOW)
        .exit_on_close_request(false)
}
