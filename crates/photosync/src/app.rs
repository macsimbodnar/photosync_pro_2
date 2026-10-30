//! The interactive front-end's state and rules: which screen shows, what
//! each key does there, and what the sync's messages change. No terminal
//! and no GUI here (no ratatui, no crossterm; `tests/ui_split.rs`), so the
//! TUI's thin view (`tui`) and a later GUI drive the same model.
//!
//! `App::update` takes a `Msg` and returns the `Effect` the front-end must
//! run: look for volumes, start the sync on a worker thread (`worker`), or
//! quit. The model does no I/O itself, so it is tested without a terminal.
//!
//! One core `Session` lasts the whole run, one card after another: the
//! library and name are fixed by the first start, and the session keeps
//! the folder, what earlier cards put on the drive, and the worst outcome.
//! The model only shows what the session says (INV-8, `flow`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use photosync_core::{Cancel, Event, Progress, RunEnd, Session, Verdict, Volumes};

use crate::args::{EXIT_INCOMPLETE, EXIT_OK, exit_code};
use crate::flow::{self, Shared, SyncError};
use crate::text;

/// The screens, in the order a sync goes through them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Sources,
    Drive,
    Library,
    Name,
    Confirm,
    Running,
    /// The sync gave no report: the refusal's sentence.
    Refused,
    Report,
}

/// A key, as the front-end saw it. Printable keys are `Char`; what a
/// letter means depends on the screen, so a text field can take any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    Esc,
    Backspace,
    Char(char),
    /// Ctrl-C. Raw mode turns it into a key, not a SIGINT.
    CtrlC,
}

pub enum Msg {
    Key(Key),
    /// What volume discovery found.
    Volumes(Volumes),
    /// The worker scanned a source: `text::scanned`'s line.
    Scanned(String),
    Progress(Event),
    /// The sync is over: its end is the session's last run.
    Finished,
    /// The worker is gone. After `Finished` that is normal; before it, the
    /// worker died, and nothing says what is on the drive: the model
    /// records a crash in the session.
    WorkerGone,
    /// The sync panicked, with the panic's text: it ended with no result.
    /// The model records it in the session (`Session::record_crash`).
    Crashed(String),
}

/// What the front-end must do for the model.
#[derive(Debug)]
pub enum Effect {
    /// Look for volumes again and send `Msg::Volumes`.
    Discover,
    /// Run `job` on a worker thread (`worker::spawn`) and send its messages.
    Start(Job),
    Quit,
}

/// One sync to run in the session. `cancel` is shared with the model,
/// which sets it on the cancel key; the core checks it between files and
/// chunks (DEC-021 (5)).
#[derive(Debug)]
pub struct Job {
    /// Locked by the worker while the sync runs; the model does not touch
    /// it until the sync's end.
    pub session: Shared,
    pub sources: Vec<PathBuf>,
    pub cancel: Cancel,
}

#[cfg(test)]
impl Job {
    pub fn library(&self) -> PathBuf {
        flow::lock(&self.session).library().to_path_buf()
    }

    pub fn name(&self) -> Option<String> {
        flow::lock(&self.session).name().map(String::from)
    }
}

/// A sync that is running.
#[derive(Debug)]
pub struct Running {
    /// One line per scanned source.
    pub scanned: Vec<String>,
    /// The latest planning or copying progress.
    pub progress: Option<Progress>,
    /// The cancel key was pressed; the core stops at the next check.
    pub cancelling: bool,
    cancel: Cancel,
    /// The cards of this sync, to name them if it ends with no result.
    sources: Vec<PathBuf>,
}

/// How the last sync ended, as its screen shows it.
#[derive(Debug)]
pub enum Outcome {
    Report {
        /// `text::session_report`'s lines: every sync of the session, and
        /// for one sync the CLI's report, word for word.
        lines: Vec<String>,
        /// `text::session_result`: whether the cards are safe to format.
        result: String,
        complete: bool,
    },
    Refused {
        /// `text::refusal`'s or `text::crash`'s sentence, then, after
        /// several syncs, the session's result.
        sentence: String,
    },
}

/// What the process says and returns once the front-end has quit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exit {
    pub code: u8,
    /// For stdout: the report, so it outlives the screen.
    pub report: Option<String>,
    /// For stderr.
    pub message: Option<String>,
}

const NO_CARD: &str =
    "No camera card found. Insert a card, or connect the X-T5 in \"USB card reader\" mode.";
const NO_DRIVE: &str = "No drive to sync to found. Plug in your external drive.";
/// Lines a page key scrolls the report.
const PAGE: usize = 10;

#[derive(Debug)]
pub struct App {
    screen: Screen,
    sources: Vec<PathBuf>,
    drives: Vec<PathBuf>,
    /// Discovery failures, as warnings.
    notes: Vec<String>,
    /// The picked sources, in the order shown.
    picked: Vec<PathBuf>,
    drive: Option<PathBuf>,
    library: String,
    name: String,
    /// The list row under the cursor, on the list screens.
    cursor: usize,
    /// The report's first line shown.
    scroll: usize,
    /// One line about the last key, e.g. why Enter did nothing.
    hint: Option<String>,
    running: Option<Running>,
    /// Made by the first start, with its library and name; kept to the end.
    session: Option<Shared>,
    /// The last sync's end, for its screen.
    outcome: Option<Outcome>,
    /// The exit as the session said at the last sync's end: its worst
    /// outcome (DEC-033). Kept here so that quitting never waits for the
    /// worker's lock; only a later end changes it, and the session never
    /// forgets a sync, so no later sync can hide an earlier failure.
    ended: Option<Exit>,
    /// Each sync of the session that was not complete, named, as of the
    /// last end (`text::not_complete`).
    not_complete: Option<String>,
    /// The session's folder and number of syncs, as of the last end.
    folder: Option<PathBuf>,
    syncs: usize,
    /// Ctrl-C was pressed while cancelling: quit at once (INV-7 makes it
    /// safe), as the CLI's second Ctrl-C does (DEC-028 (5)).
    quit_while_running: bool,
}

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}

impl App {
    /// Starts on the sources screen with nothing found yet; the front-end
    /// runs `Effect::Discover` first.
    pub fn new() -> App {
        App {
            screen: Screen::Sources,
            sources: Vec::new(),
            drives: Vec::new(),
            notes: Vec::new(),
            picked: Vec::new(),
            drive: None,
            library: String::new(),
            name: String::new(),
            cursor: 0,
            scroll: 0,
            hint: None,
            running: None,
            session: None,
            outcome: None,
            ended: None,
            not_complete: None,
            folder: None,
            syncs: 0,
            quit_while_running: false,
        }
    }

    /// A model already on its outcome screen, for view tests: a report with
    /// every kind of failure cannot come from a real sync.
    #[cfg(test)]
    pub fn with_outcome_for_test(outcome: Outcome) -> App {
        let screen = match outcome {
            Outcome::Report { .. } => Screen::Report,
            Outcome::Refused { .. } => Screen::Refused,
        };
        App {
            screen,
            outcome: Some(outcome),
            ..App::new()
        }
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }
    pub fn sources(&self) -> &[PathBuf] {
        &self.sources
    }
    pub fn is_picked(&self, source: &Path) -> bool {
        self.picked.iter().any(|p| p == source)
    }
    pub fn picked(&self) -> &[PathBuf] {
        &self.picked
    }
    pub fn drives(&self) -> &[PathBuf] {
        &self.drives
    }
    pub fn drive(&self) -> Option<&Path> {
        self.drive.as_deref()
    }
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
    pub fn library(&self) -> &str {
        &self.library
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn scroll(&self) -> usize {
        self.scroll
    }
    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }
    pub fn running(&self) -> Option<&Running> {
        self.running.as_ref()
    }
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }
    /// Whether a session is open: its library and name are then fixed, and
    /// the next card goes from the cards straight to Confirm.
    pub fn in_session(&self) -> bool {
        self.session.is_some()
    }
    /// The syncs the session has ended so far.
    pub fn syncs(&self) -> usize {
        self.syncs
    }
    /// The session's folder, once a sync fixed it.
    pub fn session_folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// The sentence shown when a list is empty.
    pub fn empty_list_note(&self) -> &'static str {
        match self.screen {
            Screen::Drive => NO_DRIVE,
            _ => NO_CARD,
        }
    }

    /// The sync folder's name: the session's, once a sync fixed it, or as
    /// it will look, the date still unknown.
    pub fn folder_preview(&self) -> String {
        if let Some(name) = self.session_folder().and_then(Path::file_name) {
            return name.to_string_lossy().into_owned();
        }
        match self.name_arg() {
            Some(name) => format!("<date> - {name}"),
            None => "<date>".to_string(),
        }
    }

    fn name_arg(&self) -> Option<String> {
        // An empty field means no name; anything else goes to the core as
        // typed, which refuses a bad one (`FolderError::InvalidName`).
        (!self.name.is_empty()).then(|| self.name.clone())
    }

    pub fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::Key(key) => {
                self.hint = None;
                self.key(key)
            }
            Msg::Volumes(volumes) => {
                self.volumes(volumes);
                None
            }
            Msg::Scanned(line) => {
                if let Some(run) = &mut self.running {
                    run.scanned.push(line);
                }
                None
            }
            Msg::Progress(event) => {
                if let Some(run) = &mut self.running {
                    run.progress = Some(event.progress);
                }
                None
            }
            Msg::Finished => {
                self.ended(None);
                None
            }
            Msg::WorkerGone => {
                self.ended(Some("its thread ended without a result".into()));
                None
            }
            Msg::Crashed(text) => {
                self.ended(Some(text));
                None
            }
        }
    }

    /// The running sync is over: `crashed` if it ended with no result,
    /// which goes into the session as a crash. After the end, a later
    /// `WorkerGone` changes nothing.
    fn ended(&mut self, crashed: Option<String>) {
        let Some(run) = self.running.take() else {
            return;
        };
        let Some(shared) = self.session.clone() else {
            return;
        };
        // The worker sends its end after unlocking: this never waits.
        let mut session = flow::lock(&shared);
        if let Some(what) = crashed {
            session.record_crash(&run.sources, what);
        }
        self.show(&session);
    }

    /// Takes what the session says now: the screen for its last sync, and
    /// the exit for its worst.
    fn show(&mut self, session: &Session) {
        self.scroll = 0;
        self.folder = session.folder().map(|f| f.path().to_path_buf());
        self.syncs = session.runs().len();
        self.not_complete = text::not_complete(session);
        let report = text::session_report(session);
        self.ended = Some(Exit {
            code: exit_code(session.verdict()),
            report: report.clone(),
            message: text::session_message(session),
        });
        let Some(last) = session.runs().last() else {
            return;
        };
        let several = self.syncs > 1;
        let sentence = |first: String| {
            if several {
                format!("{first}\n\n{}", text::session_result(session))
            } else {
                first
            }
        };
        (self.outcome, self.screen) = match &last.end {
            RunEnd::Report(_) => (
                Some(Outcome::Report {
                    lines: report
                        .unwrap_or_default()
                        .lines()
                        .map(String::from)
                        .collect(),
                    result: text::session_result(session),
                    complete: session.verdict() == Verdict::Complete,
                }),
                Screen::Report,
            ),
            RunEnd::Refused(e) => (
                Some(Outcome::Refused {
                    sentence: sentence(text::refusal(e)),
                }),
                Screen::Refused,
            ),
            RunEnd::Crashed(what) => (
                Some(Outcome::Refused {
                    sentence: sentence(text::crash(what)),
                }),
                Screen::Refused,
            ),
        };
    }

    fn key(&mut self, key: Key) -> Option<Effect> {
        if key == Key::CtrlC {
            return self.interrupt();
        }
        match self.screen {
            Screen::Sources => self.sources_key(key),
            Screen::Drive => self.drive_key(key),
            Screen::Library => {
                match key {
                    Key::Enter if self.library.trim().is_empty() => {
                        self.hint = Some("Type the library folder on your drive.".into());
                    }
                    Key::Enter => self.screen = Screen::Name,
                    Key::Esc => self.go_to_drive(),
                    _ => edit(&mut self.library, key),
                }
                None
            }
            Screen::Name => {
                match key {
                    Key::Enter => self.screen = Screen::Confirm,
                    Key::Esc => self.screen = Screen::Library,
                    _ => edit(&mut self.name, key),
                }
                None
            }
            Screen::Confirm => match key {
                // `r` on the drive screen can drop the only picked card.
                Key::Enter if self.picked.is_empty() => {
                    self.hint = Some(
                        "No card picked: the card is gone. Go back (Esc) to the cards and pick one."
                            .into(),
                    );
                    None
                }
                Key::Enter => self.start().map(Effect::Start),
                // In a session the drive, library and name are fixed.
                Key::Esc if self.in_session() => {
                    self.screen = Screen::Sources;
                    None
                }
                Key::Esc => {
                    self.screen = Screen::Name;
                    None
                }
                Key::Char('q') => Some(Effect::Quit),
                _ => None,
            },
            Screen::Running => {
                match key {
                    Key::Char('c') | Key::Esc => self.cancel(),
                    Key::Char('q') => {
                        self.hint =
                            Some("Cancel first (c), or press Ctrl-C twice to quit at once.".into());
                    }
                    _ => {}
                }
                None
            }
            Screen::Refused => match key {
                // Nothing was written, or a re-run is what the sentence
                // asks for: either way the user may fix and try again. The
                // outcome stays until a later sync ends: quitting from any
                // screen must still say the card is not safe (S018).
                Key::Enter | Key::Esc => {
                    self.screen = Screen::Confirm;
                    None
                }
                Key::Char('n') if self.in_session() => self.next_card(),
                Key::Char('q') => Some(Effect::Quit),
                _ => None,
            },
            Screen::Report => self.report_key(key),
        }
    }

    fn sources_key(&mut self, key: Key) -> Option<Effect> {
        match key {
            Key::Up | Key::Down => self.move_cursor(key, self.sources.len()),
            Key::Char(' ') => {
                if let Some(source) = self.sources.get(self.cursor).cloned() {
                    if self.is_picked(&source) {
                        self.picked.retain(|p| *p != source);
                    } else {
                        self.picked.push(source);
                        self.sort_picked();
                    }
                }
            }
            Key::Enter => {
                // Enter alone picks the card under the cursor: the usual
                // case is one card.
                if self.picked.is_empty() {
                    match self.sources.get(self.cursor) {
                        Some(source) => self.picked.push(source.clone()),
                        None => {
                            self.hint = Some(format!("{NO_CARD} Then press r."));
                            return None;
                        }
                    }
                }
                if self.in_session() {
                    self.screen = Screen::Confirm;
                } else {
                    self.go_to_drive();
                }
            }
            Key::Char('r') => return Some(Effect::Discover),
            // The first screen of a session's next card: there is no back.
            Key::Esc if self.in_session() => {
                self.hint = Some("Pick the next card, or press q to quit.".into());
            }
            Key::Char('q') | Key::Esc => return Some(Effect::Quit),
            _ => {}
        }
        None
    }

    fn drive_key(&mut self, key: Key) -> Option<Effect> {
        match key {
            Key::Up | Key::Down => self.move_cursor(key, self.drives.len()),
            Key::Enter => match self.drives.get(self.cursor).cloned() {
                Some(drive) => {
                    if self.drive.as_ref() != Some(&drive) {
                        self.library = drive.join("Photos").display().to_string();
                        self.drive = Some(drive);
                    }
                    self.screen = Screen::Library;
                }
                None => self.hint = Some(format!("{NO_DRIVE} Then press r.")),
            },
            Key::Esc => {
                self.screen = Screen::Sources;
                self.cursor = 0;
            }
            Key::Char('r') => return Some(Effect::Discover),
            Key::Char('q') => return Some(Effect::Quit),
            _ => {}
        }
        None
    }

    fn report_key(&mut self, key: Key) -> Option<Effect> {
        let last = match &self.outcome {
            Some(Outcome::Report { lines, .. }) => lines.len().saturating_sub(1),
            _ => 0,
        };
        match key {
            Key::Up => self.scroll = self.scroll.saturating_sub(1),
            Key::Down => self.scroll = (self.scroll + 1).min(last),
            Key::PageUp => self.scroll = self.scroll.saturating_sub(PAGE),
            Key::PageDown => self.scroll = (self.scroll + PAGE).min(last),
            Key::Char('n') => return self.next_card(),
            Key::Enter | Key::Esc | Key::Char('q') => return Some(Effect::Quit),
            _ => {}
        }
        None
    }

    /// Back to the cards for the next one, in the same session: same
    /// library, name and folder. Discovery runs again, as the card was
    /// swapped; nothing stays picked.
    fn next_card(&mut self) -> Option<Effect> {
        self.picked.clear();
        self.cursor = 0;
        self.screen = Screen::Sources;
        Some(Effect::Discover)
    }

    fn move_cursor(&mut self, key: Key, len: usize) {
        self.cursor = match key {
            Key::Up => self.cursor.saturating_sub(1),
            _ => (self.cursor + 1).min(len.saturating_sub(1)),
        };
    }

    fn go_to_drive(&mut self) {
        self.screen = Screen::Drive;
        self.cursor = self
            .drive
            .as_ref()
            .and_then(|d| self.drives.iter().position(|x| x == d))
            .unwrap_or(0);
    }

    fn sort_picked(&mut self) {
        let order = |p: &PathBuf| self.sources.iter().position(|s| s == p);
        let mut picked = std::mem::take(&mut self.picked);
        picked.sort_by_key(order);
        self.picked = picked;
    }

    fn interrupt(&mut self) -> Option<Effect> {
        match &self.running {
            Some(run) if run.cancelling => {
                self.quit_while_running = true;
                Some(Effect::Quit)
            }
            Some(_) => {
                self.cancel();
                None
            }
            None => Some(Effect::Quit),
        }
    }

    fn cancel(&mut self) {
        if let Some(run) = &mut self.running {
            run.cancel.cancel();
            run.cancelling = true;
        }
    }

    /// The job for the picked cards, in the session, which the first start
    /// makes. A name that is not one folder name is refused here, before
    /// any card is read, and no session is made: the name can be fixed.
    fn start(&mut self) -> Option<Job> {
        let session = match &self.session {
            Some(session) => session.clone(),
            None => {
                let library = PathBuf::from(self.library.trim());
                match flow::start(&library, self.name_arg().as_deref()) {
                    Ok(session) => {
                        let session = Arc::new(Mutex::new(session));
                        self.session = Some(session.clone());
                        session
                    }
                    Err(e) => {
                        self.refused_before_session(&e);
                        return None;
                    }
                }
            }
        };
        let cancel = Cancel::new();
        self.running = Some(Running {
            scanned: Vec::new(),
            progress: None,
            cancelling: false,
            cancel: cancel.clone(),
            sources: self.picked.clone(),
        });
        self.screen = Screen::Running;
        Some(Job {
            session,
            sources: self.picked.clone(),
            cancel,
        })
    }

    fn refused_before_session(&mut self, e: &SyncError) {
        let sentence = text::refusal(e);
        self.ended = Some(Exit {
            code: crate::args::EXIT_REFUSED,
            report: None,
            message: Some(sentence.clone()),
        });
        self.outcome = Some(Outcome::Refused { sentence });
        self.screen = Screen::Refused;
    }

    fn volumes(&mut self, volumes: Volumes) {
        self.notes = volumes
            .failures
            .iter()
            .map(|f| format!("warning: {f}"))
            .collect();
        self.sources = volumes.sources;
        self.drives = volumes.destinations;
        // A card or drive that is gone cannot stay picked.
        self.picked.retain(|p| self.sources.contains(p));
        if self
            .drive
            .as_ref()
            .is_some_and(|d| !self.drives.contains(d))
        {
            self.drive = None;
        }
        let len = match self.screen {
            Screen::Drive => self.drives.len(),
            _ => self.sources.len(),
        };
        self.cursor = self.cursor.min(len.saturating_sub(1));
    }

    /// The exit code and what to print once the front-end has quit: the
    /// same codes as the CLI (`args`), for the worst sync of the session,
    /// and its report so far. Never locks the session: a sync may hold it.
    pub fn exit(&self) -> Exit {
        if let Some(run) = &self.running {
            // Quit mid-copy: INV-7 keeps the drive safe, but some files
            // are not on it.
            let quit = if self.quit_while_running {
                "Quit during"
            } else {
                "Did not finish"
            };
            let mut message = format!(
                "{quit} sync {} ({}): not every file is known to be on the drive. Do not \
                 format the card. Run the same sync again: files already on the drive are \
                 skipped.",
                self.syncs + 1,
                run.sources
                    .iter()
                    .map(|s| s.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if let Some(earlier) = &self.not_complete {
                message.push_str(&format!(
                    " Not complete earlier in this session: {earlier}."
                ));
            }
            return Exit {
                code: EXIT_INCOMPLETE,
                report: self.ended.as_ref().and_then(|e| e.report.clone()),
                message: Some(message),
            };
        }
        self.ended.clone().unwrap_or(Exit {
            code: EXIT_OK,
            report: None,
            message: None,
        })
    }
}

/// A one-line text field: printable keys append, Backspace deletes.
fn edit(field: &mut String, key: Key) {
    match key {
        Key::Char(c) if !c.is_control() => field.push(c),
        Key::Backspace => {
            field.pop();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use photosync_core::{Counts, EventKind, Phase, ReportError, Run, SourceProgress};

    use super::*;
    use crate::args::EXIT_REFUSED;
    use crate::common::{TempDir, at, write_file};
    use crate::worker;

    fn found(sources: &[&str], drives: &[&str]) -> Volumes {
        Volumes {
            sources: sources.iter().map(PathBuf::from).collect(),
            destinations: drives.iter().map(PathBuf::from).collect(),
            failures: Vec::new(),
        }
    }

    fn app(sources: &[&str], drives: &[&str]) -> App {
        let mut app = App::new();
        assert!(app.update(Msg::Volumes(found(sources, drives))).is_none());
        app
    }

    fn keys(app: &mut App, keys: &[Key]) -> Vec<Effect> {
        keys.iter()
            .filter_map(|k| app.update(Msg::Key(*k)))
            .collect()
    }

    fn typed(app: &mut App, text: &str) {
        for c in text.chars() {
            assert!(app.update(Msg::Key(Key::Char(c))).is_none());
        }
    }

    fn start(app: &mut App) -> Job {
        match app.update(Msg::Key(Key::Enter)) {
            Some(Effect::Start(job)) => job,
            other => panic!("no start: {other:?}"),
        }
    }

    /// S039, the TUI: what the report screen is given, from real syncs. In
    /// a session of one sync and of several, failures and library problems
    /// of every sync come before the first duplicate line, and the session's
    /// result stays last. The library is off the images, as chmod does
    /// nothing on exFAT.
    #[test]
    fn duplicates_never_bury_a_failure_or_a_library_problem_in_the_tui() {
        use crate::common::{OnDrive, disk_of_the_binary, lock_in_library, test_drive};
        struct Quiet;
        impl flow::Watch for Quiet {}
        const TAKEN: u64 = 1_700_000_000;

        let test = "duplicates_never_bury_a_failure_or_a_library_problem_in_the_tui";
        if test_drive(test).is_none() {
            return;
        }
        let dir = TempDir::new();
        let library = OnDrive::new(&disk_of_the_binary(), "s039-app");
        let card = |name: &str| dir.0.join(name);
        let (a, b, backup, late) = (card("a"), card("b"), card("backup"), card("late"));
        for folder in ["100_FUJI", "101_FUJI"] {
            write_file(
                &a,
                &format!("DCIM/{folder}/DSCF0001.RAF"),
                b"twice",
                at(TAKEN),
            );
        }
        write_file(&a, "DCIM/100_FUJI/DSCF8000.RAF", b"from a", at(TAKEN));
        write_file(&b, "DCIM/100_FUJI/DSCF8000.RAF", b"from b", at(TAKEN));
        write_file(&a, "DCIM/100_FUJI/DSCF9000.RAF", b"new", at(TAKEN));
        let locked = lock_in_library(&library.0, "old/DSCF9000.RAF", b"old");
        write_file(&backup, "DCIM/100_FUJI/DSCF0001.RAF", b"twice", at(TAKEN));
        write_file(&late, "DCIM/100_FUJI/DSCF8000.RAF", b"from c", at(TAKEN));

        let collided = |card: &Path| {
            format!(
                "  failed      {}",
                card.join("DCIM/100_FUJI/DSCF8000.RAF").display()
            )
        };
        let lines_of = |app: &App| match app.outcome() {
            Some(Outcome::Report { lines, .. }) => lines.clone(),
            other => panic!("no report: {other:?}"),
        };
        let first_duplicate = |lines: &[String]| {
            lines
                .iter()
                .position(|l| l.starts_with("  duplicate in session  "))
                .unwrap_or_else(|| panic!("no duplicate: {lines:#?}"))
        };
        let before = |lines: &[String], part: &str| {
            let at = lines.iter().position(|l| l.starts_with(part));
            assert!(
                at.is_some_and(|i| i < first_duplicate(lines)),
                "{part}: {lines:#?}"
            );
        };

        let mut session = Session::new(&library.0, None).unwrap();
        let cancel = Cancel::new();
        session.sync(&[a.clone(), b.clone()], &cancel, &mut Quiet);
        let mut app = App::new();
        app.show(&session);
        let lines = lines_of(&app);
        before(&lines, &collided(&b));
        before(&lines, "Library entries that could not be read");
        before(&lines, &format!("  {}: ", locked.display()));

        // A later sync's failure is not buried by an earlier sync's list:
        // the backup card, all duplicates, comes before it.
        session.sync(std::slice::from_ref(&backup), &cancel, &mut Quiet);
        session.sync(std::slice::from_ref(&late), &cancel, &mut Quiet);
        app.show(&session);
        let lines = lines_of(&app);
        before(&lines, &collided(&b));
        before(&lines, &format!("  {}: ", locked.display()));
        before(&lines, &format!("Sync 3 of 3: {}", late.display()));
        before(&lines, &collided(&late));
        let heading = format!("Duplicates in session on {}", backup.display());
        assert!(lines.iter().any(|l| *l == heading), "{heading}: {lines:#?}");
        let all_syncs = lines.iter().position(|l| l == "All syncs").unwrap();
        assert!(first_duplicate(&lines) < all_syncs, "{lines:#?}");
        assert!(
            lines
                .last()
                .unwrap()
                .starts_with("Result: NOT COMPLETE: 2 of 3 syncs"),
            "{lines:#?}"
        );
        // S040: and the end says how many lines to go back for.
        assert!(
            lines[lines.len() - 3].ends_with(" problem lines above the duplicates: read them."),
            "{lines:#?}"
        );
    }

    /// Runs `job` on the real worker and gives the model every message,
    /// then the channel's end, as the TUI's loop does.
    fn run_job(app: &mut App, job: Job) {
        let (tx, rx) = mpsc::channel();
        worker::spawn(job, move |m| tx.send(m).unwrap());
        take_all(app, rx);
    }

    /// `run_job` with the worker's sync injected.
    fn run_job_with(
        app: &mut App,
        job: Job,
        sync: impl FnOnce(&mut Session, &[PathBuf], &Cancel, &mut dyn flow::Watch) + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        worker::spawn_with(job, move |m| tx.send(m).unwrap(), sync);
        take_all(app, rx);
    }

    fn take_all(app: &mut App, rx: mpsc::Receiver<Msg>) {
        for msg in rx {
            app.update(msg);
        }
        app.update(Msg::WorkerGone);
    }

    /// A sync whose copy ran but whose report does not add up. No real
    /// sync gives one and the binary may not use the core's test seam, so
    /// the injected sync records it in the job's session (DEC-035).
    fn refuse_report(session: &mut Session, sources: &[PathBuf], _: &Cancel, _: &mut dyn flow::Watch) {
        session.record_refusal(
            sources,
            SyncError::Report(ReportError::SourceTwice(PathBuf::from("/V/CARD"))),
        );
    }

    /// A card with one photo, in a temp dir: on the library's device, so a
    /// sync of it is refused (INV-10), unless the library is missing.
    fn temp_card(dir: &TempDir) -> String {
        let card = dir.0.join("CARD");
        write_file(
            &card,
            "DCIM/100_FUJI/DSCF0001.RAF",
            b"raw",
            at(1_700_000_000),
        );
        card.display().to_string()
    }

    /// Through every screen to the job, with the default library.
    #[test]
    fn pick_card_drive_library_and_name_then_start() {
        let mut app = app(&["/V/CARD", "/V/CARD2"], &["/V/SSD", "/V/USB"]);
        assert_eq!(app.screen(), Screen::Sources);

        keys(&mut app, &[Key::Down, Key::Char(' '), Key::Enter]);
        assert_eq!(app.picked(), [PathBuf::from("/V/CARD2")]);
        assert_eq!(app.screen(), Screen::Drive);

        keys(&mut app, &[Key::Down, Key::Enter]);
        assert_eq!(app.screen(), Screen::Library);
        assert_eq!(app.library(), "/V/USB/Photos");

        keys(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Name);
        typed(&mut app, "Tirana");
        assert_eq!(app.folder_preview(), "<date> - Tirana");
        keys(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Confirm);

        let job = start(&mut app);
        assert_eq!(app.screen(), Screen::Running);
        assert_eq!(job.sources, [PathBuf::from("/V/CARD2")]);
        assert_eq!(job.library(), PathBuf::from("/V/USB/Photos"));
        assert_eq!(job.name().as_deref(), Some("Tirana"));
        assert!(!job.cancel.is_cancelled());
    }

    #[test]
    fn enter_alone_picks_the_card_under_the_cursor_and_several_keep_list_order() {
        let mut one = app(&["/V/A", "/V/B"], &["/V/SSD"]);
        keys(&mut one, &[Key::Enter]);
        assert_eq!(one.picked(), [PathBuf::from("/V/A")]);

        let mut two = app(&["/V/A", "/V/B", "/V/C"], &["/V/SSD"]);
        keys(
            &mut two,
            &[
                Key::Down,
                Key::Down,
                Key::Char(' '),
                Key::Up,
                Key::Up,
                Key::Char(' '),
            ],
        );
        assert_eq!(two.picked(), [PathBuf::from("/V/A"), PathBuf::from("/V/C")]);
        keys(&mut two, &[Key::Char(' ')]);
        assert_eq!(two.picked(), [PathBuf::from("/V/C")]);
    }

    #[test]
    fn nothing_found_explains_and_goes_nowhere() {
        let mut none = app(&[], &[]);
        keys(&mut none, &[Key::Enter]);
        assert_eq!(none.screen(), Screen::Sources);
        assert!(
            none.hint().unwrap().starts_with(NO_CARD),
            "{:?}",
            none.hint()
        );
        assert!(matches!(
            keys(&mut none, &[Key::Char('r')])[..],
            [Effect::Discover]
        ));

        let mut no_drive = app(&["/V/CARD"], &[]);
        keys(&mut no_drive, &[Key::Enter, Key::Enter]);
        assert_eq!(no_drive.screen(), Screen::Drive);
        assert!(no_drive.hint().unwrap().starts_with(NO_DRIVE));
        assert_eq!(no_drive.empty_list_note(), NO_DRIVE);
    }

    #[test]
    fn library_field_edits_refuses_empty_and_follows_a_new_drive() {
        let mut app = app(&["/V/CARD"], &["/V/SSD", "/V/USB"]);
        keys(&mut app, &[Key::Enter, Key::Enter]);
        assert_eq!(app.library(), "/V/SSD/Photos");
        for _ in 0..6 {
            keys(&mut app, &[Key::Backspace]);
        }
        typed(&mut app, "Pics");
        assert_eq!(app.library(), "/V/SSD/Pics");
        // The same drive again keeps what was typed.
        keys(&mut app, &[Key::Esc, Key::Enter]);
        assert_eq!(app.library(), "/V/SSD/Pics");
        // Another drive gets its own default.
        keys(&mut app, &[Key::Esc, Key::Down, Key::Enter]);
        assert_eq!(app.library(), "/V/USB/Photos");

        while !app.library().is_empty() {
            keys(&mut app, &[Key::Backspace]);
        }
        typed(&mut app, "  ");
        keys(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Library);
        assert!(app.hint().is_some());
    }

    /// 'q' and ' ' are text in a field, not commands.
    #[test]
    fn letters_are_text_in_fields_and_commands_in_lists() {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        assert_eq!(app.screen(), Screen::Name);
        typed(&mut app, "q r c");
        assert_eq!(app.name(), "q r c");
        let mut list = self::app(&["/V/CARD"], &["/V/SSD"]);
        assert!(matches!(
            keys(&mut list, &[Key::Char('q')])[..],
            [Effect::Quit]
        ));
    }

    #[test]
    fn empty_name_means_no_name() {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter, Key::Enter]);
        assert_eq!(app.folder_preview(), "<date>");
        assert_eq!(start(&mut app).name(), None);
    }

    #[test]
    fn esc_walks_back_one_screen_at_a_time() {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter, Key::Enter]);
        let mut seen = vec![app.screen()];
        for _ in 0..4 {
            keys(&mut app, &[Key::Esc]);
            seen.push(app.screen());
        }
        assert_eq!(
            seen,
            [
                Screen::Confirm,
                Screen::Name,
                Screen::Library,
                Screen::Drive,
                Screen::Sources
            ]
        );
    }

    fn running() -> (App, Job) {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter, Key::Enter]);
        let job = start(&mut app);
        (app, job)
    }

    /// The cancel key sets the core's `Cancel` that the job carries.
    #[test]
    fn cancel_key_sets_the_jobs_cancel() {
        for key in [Key::Char('c'), Key::Esc, Key::CtrlC] {
            let (mut app, job) = running();
            assert!(keys(&mut app, &[key]).is_empty());
            assert!(job.cancel.is_cancelled(), "{key:?}");
            assert!(app.running().unwrap().cancelling);
            assert_eq!(app.screen(), Screen::Running);
        }
    }

    #[test]
    fn q_does_not_quit_a_running_sync_but_a_second_ctrl_c_does() {
        let (mut app, job) = running();
        assert!(keys(&mut app, &[Key::Char('q')]).is_empty());
        assert!(app.hint().is_some());
        assert!(!job.cancel.is_cancelled());

        assert!(keys(&mut app, &[Key::CtrlC]).is_empty());
        assert!(matches!(keys(&mut app, &[Key::CtrlC])[..], [Effect::Quit]));
        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE);
        assert!(exit.message.unwrap().contains("Do not format the card."));
    }

    #[test]
    fn ctrl_c_quits_when_nothing_runs() {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        assert!(matches!(keys(&mut app, &[Key::CtrlC])[..], [Effect::Quit]));
        assert_eq!(
            app.exit(),
            Exit {
                code: EXIT_OK,
                report: None,
                message: None
            }
        );
    }

    #[test]
    fn progress_and_scans_are_kept_while_running() {
        let (mut app, _job) = running();
        app.update(Msg::Scanned("scanned /V/CARD: 2 photos".into()));
        let progress = Progress {
            phase: Phase::Copying,
            done: Counts { files: 1, bytes: 5 },
            total: Counts {
                files: 2,
                bytes: 10,
            },
            sources: vec![SourceProgress {
                root: PathBuf::from("/V/CARD"),
                done: Counts { files: 1, bytes: 5 },
                total: Counts {
                    files: 2,
                    bytes: 10,
                },
            }],
            elapsed: Duration::from_secs(1),
            eta: Some(Duration::from_secs(1)),
        };
        app.update(Msg::Progress(Event {
            kind: EventKind::Bytes,
            progress: progress.clone(),
        }));
        let run = app.running().unwrap();
        assert_eq!(run.scanned, ["scanned /V/CARD: 2 photos"]);
        assert_eq!(run.progress.as_ref(), Some(&progress));
    }

    /// A refusal shows the CLI's sentence, exits 2, and lets the user go
    /// back, fix and start again with a fresh cancel.
    #[test]
    fn refusal_shows_the_cli_sentence_and_allows_a_retry() {
        let dir = TempDir::new();
        let card = temp_card(&dir);
        let mut app = app(&[&card], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter, Key::Enter]);
        let job = start(&mut app);
        keys(&mut app, &[Key::CtrlC]);
        assert!(job.cancel.is_cancelled());
        run_job(&mut app, job);
        let Some(Outcome::Refused { sentence }) = app.outcome() else {
            panic!("not refused: {app:?}");
        };
        let sentence = sentence.clone();
        assert_eq!(app.screen(), Screen::Refused);
        assert!(sentence.starts_with("The library folder /V/SSD/Photos does not exist."));
        let exit = app.exit();
        assert_eq!(
            (exit.code, exit.message.as_deref()),
            (EXIT_REFUSED, Some(&*sentence))
        );
        // The worker's end after `Finished` changes nothing.
        app.update(Msg::WorkerGone);
        assert_eq!(app.screen(), Screen::Refused);

        keys(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Confirm);
        let again = start(&mut app);
        assert!(!again.cancel.is_cancelled());
    }

    /// A sync whose report was refused after copying is not complete: it
    /// exits 1 with the refusal's sentence, and a later sync's result names
    /// it with what it left undone.
    #[test]
    fn a_report_that_does_not_add_up_exits_1() {
        let (mut app, job) = running();
        run_job_with(&mut app, job, refuse_report);
        assert_eq!(app.screen(), Screen::Refused);
        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE, "{exit:?}");
        let message = exit.message.unwrap();
        assert!(
            message.starts_with("The copy ran, but its report does not add up ("),
            "{message}"
        );
        assert!(message.contains("Do not format the card."), "{message}");

        // `/V/CARD` does not exist: the next card is refused at its scan.
        keys(&mut app, &[Key::Char('n'), Key::Enter]);
        let next = start(&mut app);
        run_job(&mut app, next);
        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE, "{exit:?}");
        let message = exit.message.unwrap();
        assert!(
            message.contains("sync 1 (/V/CARD): copied, but its report does not add up"),
            "{message}"
        );

        let run = Run {
            sources: vec![PathBuf::from("/V/CARD")],
            end: RunEnd::Refused(SyncError::Report(ReportError::SourceTwice(PathBuf::from(
                "/V/CARD",
            )))),
        };
        assert_eq!(exit_code(run.verdict()), EXIT_INCOMPLETE);
    }

    #[test]
    fn a_worker_that_dies_is_not_complete() {
        let (mut app, _job) = running();
        app.update(Msg::WorkerGone);
        assert_eq!(app.screen(), Screen::Refused);
        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE);
        assert!(exit.message.unwrap().contains("Do not format the card."));
    }

    /// S018: the outcome that says the card is not safe stays the exit
    /// until a later sync finishes. Going back from its screen and quitting
    /// elsewhere must not exit 0 or lose the sentence.
    #[test]
    fn a_failure_is_kept_after_going_back_and_quitting_elsewhere() {
        type End = Box<dyn Fn(&mut App, Job)>;
        for (what, end) in [
            (
                "worker gone",
                Box::new(|app: &mut App, _| {
                    app.update(Msg::WorkerGone);
                }) as End,
            ),
            (
                "crashed",
                Box::new(|app: &mut App, _| {
                    app.update(Msg::Crashed("boom".into()));
                }),
            ),
            (
                "report error",
                Box::new(|app: &mut App, job| run_job_with(app, job, refuse_report)),
            ),
        ] {
            for back in [Key::Enter, Key::Esc] {
                let (mut app, job) = running();
                end(&mut app, job);
                let failed = app.exit();
                assert_eq!(failed.code, EXIT_INCOMPLETE, "{what}");
                assert!(failed.message.is_some(), "{what}");

                keys(&mut app, &[back]);
                assert_eq!(app.screen(), Screen::Confirm, "{what}");
                assert_eq!(app.exit(), failed, "{what}: back with {back:?}");
                let quit = keys(
                    &mut app,
                    &[Key::Esc, Key::Esc, Key::Esc, Key::Esc, Key::Char('q')],
                );
                assert_eq!(app.screen(), Screen::Sources, "{what}");
                assert!(matches!(quit[..], [Effect::Quit]), "{what}");
                assert_eq!(app.exit(), failed, "{what}: quit from the cards");
            }
        }
    }

    /// DEC-033, rewritten by the user's ruling: across a session the worst
    /// outcome wins. A later sync refused before copying (2 alone) does not
    /// replace an earlier crash: the exit stays 1, and the message names
    /// the crashed sync, and the refused one too.
    #[test]
    fn a_later_sync_never_replaces_the_kept_failure() {
        let (mut app, _job) = running();
        app.update(Msg::WorkerGone);
        keys(&mut app, &[Key::Enter]);
        let again = start(&mut app);
        // While it runs, quitting is not complete either, and says why.
        let quit = app.exit();
        assert_eq!(quit.code, EXIT_INCOMPLETE);
        assert!(
            quit.message
                .unwrap()
                .contains("sync 1 (/V/CARD): stopped with an internal error")
        );
        // `/V/CARD` does not exist: the retry is refused at its scan.
        run_job(&mut app, again);
        assert_eq!(app.screen(), Screen::Refused);

        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE, "{exit:?}");
        let message = exit.message.unwrap();
        for part in [
            "Result: NOT COMPLETE: 2 of 2 syncs not complete: ",
            "sync 1 (/V/CARD): stopped with an internal error (its thread ended without a result)",
            "sync 2 (/V/CARD): refused, nothing copied",
            "Do not format the cards",
        ] {
            assert!(message.contains(part), "{part}: {message}");
        }
        let report = exit.report.unwrap();
        assert!(
            report.contains("Sync 2 of 2: /V/CARD\n  Cannot read /V/CARD"),
            "{report}"
        );
    }

    /// `r` on the drive screen can drop the only picked card: Confirm then
    /// has nothing to copy from and must not start a sync of no source.
    #[test]
    fn confirm_refuses_to_start_with_no_card_picked() {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Drive);
        assert!(matches!(
            keys(&mut app, &[Key::Char('r')])[..],
            [Effect::Discover]
        ));
        app.update(Msg::Volumes(found(&[], &["/V/SSD"])));
        assert!(app.picked().is_empty());
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        assert_eq!(app.screen(), Screen::Confirm);

        let effect = app.update(Msg::Key(Key::Enter));
        assert!(effect.is_none(), "{effect:?}");
        assert_eq!(app.screen(), Screen::Confirm);
        assert!(app.running().is_none());
        assert!(
            app.hint().is_some_and(|h| h.contains("No card")),
            "{:?}",
            app.hint()
        );
    }

    #[test]
    fn a_sync_that_panicked_is_not_complete_and_says_why() {
        let (mut app, _job) = running();
        app.update(Msg::Crashed("index out of bounds".into()));
        assert_eq!(app.screen(), Screen::Refused);
        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE);
        let message = exit.message.unwrap();
        assert!(message.contains("index out of bounds"), "{message}");
        assert!(message.contains("Do not format the card."), "{message}");
        // The channel closing after it changes nothing.
        app.update(Msg::WorkerGone);
        assert_eq!(app.exit().message.unwrap(), message);
    }

    #[test]
    fn refresh_keeps_picks_still_there_and_drops_gone_ones() {
        let mut app = app(&["/V/A", "/V/B"], &["/V/SSD"]);
        keys(&mut app, &[Key::Char(' '), Key::Down, Key::Char(' ')]);
        app.update(Msg::Volumes(found(&["/V/B"], &["/V/SSD"])));
        assert_eq!(app.picked(), [PathBuf::from("/V/B")]);
        assert_eq!(app.cursor(), 0);
    }

    /// S020: a crash, by panic or by a worker gone, is recorded in the
    /// session itself, so the session's verdict and report hold it.
    #[test]
    fn a_crash_is_recorded_in_the_session() {
        for (end, what) in [
            (
                Msg::Crashed("index out of bounds".into()),
                "index out of bounds",
            ),
            (Msg::WorkerGone, "its thread ended without a result"),
        ] {
            let (mut app, job) = running();
            app.update(end);
            let session = flow::lock(&job.session);
            assert_eq!(session.verdict(), Verdict::NotComplete);
            match session.runs() {
                [run] => {
                    assert_eq!(run.sources, [PathBuf::from("/V/CARD")]);
                    assert!(
                        matches!(&run.end, RunEnd::Crashed(w) if w == what),
                        "{run:?}"
                    );
                }
                runs => panic!("{} runs", runs.len()),
            }
        }
    }

    /// S020: after a sync, `n` goes back to the cards for the next one in
    /// the same session: the same library, name and session, with nothing
    /// picked and the cards looked for again. The fixed drive, library and
    /// name screens are skipped both ways.
    #[test]
    fn next_card_goes_on_in_the_same_session() {
        let mut app = app(&["/V/CARD", "/V/CARD2"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        typed(&mut app, "Tirana");
        keys(&mut app, &[Key::Enter]);
        let first = start(&mut app);
        let session = first.session.clone();
        run_job(&mut app, first);
        assert_eq!(app.screen(), Screen::Refused);

        assert!(matches!(
            keys(&mut app, &[Key::Char('n')])[..],
            [Effect::Discover]
        ));
        assert_eq!(app.screen(), Screen::Sources);
        assert!(app.picked().is_empty());
        assert!(
            keys(&mut app, &[Key::Esc]).is_empty(),
            "Esc quit the session"
        );
        keys(&mut app, &[Key::Down, Key::Enter]);
        assert_eq!(
            app.screen(),
            Screen::Confirm,
            "the fixed screens were not skipped"
        );
        keys(&mut app, &[Key::Esc]);
        assert_eq!(app.screen(), Screen::Sources);
        keys(&mut app, &[Key::Enter]);
        let next = start(&mut app);

        assert!(Arc::ptr_eq(&next.session, &session), "a new session");
        assert_eq!(next.sources, [PathBuf::from("/V/CARD2")]);
        assert_eq!(next.library(), PathBuf::from("/V/SSD/Photos"));
        assert_eq!(next.name().as_deref(), Some("Tirana"));
        run_job(&mut app, next);
        assert_eq!(flow::lock(&session).runs().len(), 2);
        assert_eq!(app.syncs(), 2);
    }

    /// S023: in the TUI, the result of a session of several syncs, one of
    /// which copied a file from its twin, names the card that could not be
    /// read, and the report keeps that sync's own result. Real syncs, so on
    /// the drive image.
    #[test]
    fn the_result_box_names_the_card_a_file_was_copied_from_its_twin_for() {
        use crate::common::{LoseOnCopy, OnDrive, test_drive, twin_cards};

        let Some(drive) =
            test_drive("the_result_box_names_the_card_a_file_was_copied_from_its_twin_for")
        else {
            return;
        };
        let Some((card_a, card_b)) =
            twin_cards("the_result_box_names_the_card_a_file_was_copied_from_its_twin_for")
        else {
            return;
        };
        // Given to the app as its drive, so the default library is in it.
        let ours = OnDrive::new(&drive, "S023-app");
        std::fs::create_dir(ours.0.join("Photos")).unwrap();
        let dir = TempDir::new();
        // The twins on two images: on one device they would be no twins
        // (DEC-038).
        let (on_a, on_b) = (
            OnDrive::new(&card_a, "S023-app"),
            OnDrive::new(&card_b, "S023-app"),
        );
        let (a, b, c) = (on_a.0.clone(), on_b.0.clone(), dir.0.join("c"));
        let lost = write_file(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"twin", at(1_700_000_000));
        write_file(&b, "DCIM/100_FUJI/DSCF0001.RAF", b"twin", at(1_700_000_000));
        write_file(
            &c,
            "DCIM/100_FUJI/DSCF0002.RAF",
            b"other",
            at(1_700_000_000),
        );
        let s = |p: &PathBuf| p.display().to_string();
        let (sa, sb, sc, sd) = (s(&a), s(&b), s(&c), s(&ours.0));

        let mut app = app(&[&sa, &sb, &sc], &[&sd]);
        keys(&mut app, &[Key::Char(' '), Key::Down, Key::Char(' ')]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter, Key::Enter]);
        let first = start(&mut app);
        assert_eq!(first.sources, [a.clone(), b.clone()]);
        run_job_with(&mut app, first, move |session, sources, cancel, watch| {
            session.sync(
                sources,
                cancel,
                &mut LoseOnCopy {
                    path: lost,
                    inner: watch,
                },
            );
        });
        keys(&mut app, &[Key::Char('n')]);
        app.update(Msg::Volumes(found(&[&sc], &[&sd])));
        keys(&mut app, &[Key::Enter]);
        let second = start(&mut app);
        run_job(&mut app, second);

        assert_eq!(app.syncs(), 2);
        assert_eq!(app.exit().code, EXIT_OK);
        let warning = format!(
            "1 could not be read on {sa} and was copied from another card: check that card"
        );
        match app.outcome() {
            Some(Outcome::Report {
                lines,
                result,
                complete,
            }) => {
                assert!(
                    result.ends_with(&format!(" over 2 syncs. {warning}.")),
                    "{result}"
                );
                // The sync's own result names the card too (S024). Only the
                // indented per-sync line counts: the session line ends the
                // same way and would pass on its own.
                let own = format!("{warning}.");
                assert!(
                    lines.iter().any(|l| l.starts_with("  Result: all") && l.ends_with(&own)),
                    "{lines:#?}"
                );
                assert!(*complete, "the photos are on the drive");
            }
            other => panic!("{other:?}"),
        }
    }

    /// S020: quitting during a later sync names every earlier sync that was
    /// not complete, as well as the one that did not finish.
    #[test]
    fn quitting_mid_sync_names_the_earlier_failures_too() {
        let (mut app, _job) = running();
        app.update(Msg::Crashed("boom".into()));
        keys(&mut app, &[Key::Char('n'), Key::Enter]);
        let _again = start(&mut app);
        keys(&mut app, &[Key::CtrlC, Key::CtrlC]);

        let exit = app.exit();
        assert_eq!(exit.code, EXIT_INCOMPLETE);
        let message = exit.message.unwrap();
        assert!(
            message.starts_with("Quit during sync 2 (/V/CARD): "),
            "{message}"
        );
        assert!(
            message.contains(
                "Not complete earlier in this session: sync 1 (/V/CARD): stopped with an \
                 internal error (boom)."
            ),
            "{message}"
        );
    }

    /// A name that cannot be a folder name is refused before any card is
    /// read, and opens no session: going back, the name can be fixed.
    #[test]
    fn an_invalid_name_is_refused_and_can_still_be_fixed() {
        let mut app = app(&["/V/CARD"], &["/V/SSD"]);
        keys(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        typed(&mut app, "a/b");
        keys(&mut app, &[Key::Enter]);
        assert!(app.update(Msg::Key(Key::Enter)).is_none(), "started");
        assert_eq!(app.screen(), Screen::Refused);
        assert!(!app.in_session());
        let exit = app.exit();
        assert_eq!(exit.code, EXIT_REFUSED);
        assert!(
            exit.message
                .unwrap()
                .starts_with("The name \"a/b\" cannot be a folder name.")
        );

        keys(&mut app, &[Key::Enter, Key::Esc]);
        assert_eq!(app.screen(), Screen::Name);
        for _ in 0..3 {
            keys(&mut app, &[Key::Backspace]);
        }
        typed(&mut app, "Tirana");
        keys(&mut app, &[Key::Enter]);
        assert_eq!(start(&mut app).name().as_deref(), Some("Tirana"));
    }
}
