//! The interactive front-end's state and rules: which screen shows, what
//! each key does there, and what the sync's messages change. No terminal
//! and no GUI here (no ratatui, no crossterm; `tests/ui_split.rs`), so the
//! TUI's thin view (`tui`) and a later GUI drive the same model.
//!
//! `App::update` takes a `Msg` and returns the `Effect` the front-end must
//! run: look for volumes, start the sync on a worker thread (`worker`), or
//! quit. The model does no I/O itself, so it is tested without a terminal.
//! Every sync rule stays in the core, reached through `flow` (INV-8).

use std::path::{Path, PathBuf};

use photosync_core::{Cancel, Event, Progress, Report, Volumes};

use crate::args::{EXIT_INCOMPLETE, EXIT_OK, EXIT_REFUSED};
use crate::flow::SyncError;
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
    /// The sync is over.
    Finished(Result<Report, SyncError>),
    /// The worker is gone. After `Finished` that is normal; before it, the
    /// worker died, and nothing says what is on the drive.
    WorkerGone,
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

/// One sync to run. `cancel` is shared with the model, which sets it on the
/// cancel key; the core checks it between files and chunks (DEC-021 (5)).
#[derive(Debug)]
pub struct Job {
    pub sources: Vec<PathBuf>,
    pub library: PathBuf,
    pub name: Option<String>,
    pub cancel: Cancel,
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
}

/// How the last sync ended.
#[derive(Debug)]
pub enum Outcome {
    Report {
        /// `text::report`'s lines: the CLI's report, word for word.
        lines: Vec<String>,
        /// `text::result_line`: says whether the cards are safe to format.
        result: String,
        complete: bool,
    },
    Refused {
        /// `text::refusal`'s sentence.
        sentence: String,
        code: u8,
    },
}

impl Outcome {
    pub fn from_report(report: &Report) -> Outcome {
        let totals = report.totals();
        Outcome::Report {
            lines: text::report(report).lines().map(String::from).collect(),
            result: text::result_line(&totals, report.sources()),
            complete: text::complete(&totals, report.sources()),
        }
    }
}

/// What the process says and returns once the front-end has quit.
#[derive(Debug, PartialEq, Eq)]
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
    outcome: Option<Outcome>,
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
            outcome: None,
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

    /// The sentence shown when a list is empty.
    pub fn empty_list_note(&self) -> &'static str {
        match self.screen {
            Screen::Drive => NO_DRIVE,
            _ => NO_CARD,
        }
    }

    /// The sync folder's name as it will look, the date still unknown.
    pub fn folder_preview(&self) -> String {
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
            Msg::Finished(result) => {
                self.finished(result);
                None
            }
            Msg::WorkerGone => {
                if self.running.take().is_some() {
                    self.outcome = Some(Outcome::Refused {
                        sentence: "The copy stopped with an internal error, so nothing \
                                   says which files are on the drive. Do not format the \
                                   card. Run the same sync again: files already on the \
                                   drive are skipped."
                            .to_string(),
                        code: EXIT_INCOMPLETE,
                    });
                    self.screen = Screen::Refused;
                }
                None
            }
        }
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
                Key::Enter => Some(Effect::Start(self.start())),
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
                // asks for: either way the user may fix and try again.
                Key::Enter | Key::Esc => {
                    self.outcome = None;
                    self.screen = Screen::Confirm;
                    None
                }
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
                self.go_to_drive();
            }
            Key::Char('r') => return Some(Effect::Discover),
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
            Key::Enter | Key::Esc | Key::Char('q') => return Some(Effect::Quit),
            _ => {}
        }
        None
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

    fn start(&mut self) -> Job {
        let cancel = Cancel::new();
        self.running = Some(Running {
            scanned: Vec::new(),
            progress: None,
            cancelling: false,
            cancel: cancel.clone(),
        });
        self.outcome = None;
        self.screen = Screen::Running;
        Job {
            sources: self.picked.clone(),
            library: PathBuf::from(self.library.trim()),
            name: self.name_arg(),
            cancel,
        }
    }

    fn finished(&mut self, result: Result<Report, SyncError>) {
        self.running = None;
        self.scroll = 0;
        match result {
            Ok(report) => {
                self.outcome = Some(Outcome::from_report(&report));
                self.screen = Screen::Report;
            }
            Err(e) => {
                let code = match e {
                    // Files may be on the drive, but nothing says which.
                    SyncError::Report(_) => EXIT_INCOMPLETE,
                    _ => EXIT_REFUSED,
                };
                self.outcome = Some(Outcome::Refused {
                    sentence: text::refusal(&e),
                    code,
                });
                self.screen = Screen::Refused;
            }
        }
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
    /// same codes as the CLI (`args`).
    pub fn exit(&self) -> Exit {
        if self.running.is_some() {
            // Quit mid-copy: INV-7 keeps the drive safe, but some files
            // are not on it.
            let quit = if self.quit_while_running {
                "Quit during the sync"
            } else {
                "The sync did not finish"
            };
            return Exit {
                code: EXIT_INCOMPLETE,
                report: None,
                message: Some(format!(
                    "{quit}: not every file is known to be on the drive. Do not format \
                     the card. Run the same sync again: files already on the drive are \
                     skipped."
                )),
            };
        }
        match &self.outcome {
            None => Exit {
                code: EXIT_OK,
                report: None,
                message: None,
            },
            Some(Outcome::Report {
                lines, complete, ..
            }) => Exit {
                code: if *complete { EXIT_OK } else { EXIT_INCOMPLETE },
                report: Some(lines.join("\n") + "\n"),
                message: None,
            },
            Some(Outcome::Refused { sentence, code }) => Exit {
                code: *code,
                report: None,
                message: Some(sentence.clone()),
            },
        }
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
    use std::io;
    use std::time::Duration;

    use photosync_core::{Counts, EventKind, Phase, PlanError, ScanError, SourceProgress};

    use super::*;

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
        assert_eq!(job.library, PathBuf::from("/V/USB/Photos"));
        assert_eq!(job.name.as_deref(), Some("Tirana"));
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
        assert_eq!(start(&mut app).name, None);
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
        let (mut app, job) = running();
        keys(&mut app, &[Key::CtrlC]);
        assert!(job.cancel.is_cancelled());
        let err = SyncError::Plan(PlanError::Library(ScanError {
            path: PathBuf::from("/V/SSD/Photos"),
            source: io::Error::from(io::ErrorKind::NotFound),
        }));
        let sentence = text::refusal(&err);
        app.update(Msg::Finished(Err(err)));
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

    #[test]
    fn a_report_that_does_not_add_up_exits_1() {
        let (mut app, _job) = running();
        app.update(Msg::Finished(Err(SyncError::Report(
            photosync_core::ReportError::SourceTwice(PathBuf::from("/V/CARD")),
        ))));
        assert_eq!(app.exit().code, EXIT_INCOMPLETE);
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

    #[test]
    fn refresh_keeps_picks_still_there_and_drops_gone_ones() {
        let mut app = app(&["/V/A", "/V/B"], &["/V/SSD"]);
        keys(&mut app, &[Key::Char(' '), Key::Down, Key::Char(' ')]);
        app.update(Msg::Volumes(found(&["/V/B"], &["/V/SSD"])));
        assert_eq!(app.picked(), [PathBuf::from("/V/B")]);
        assert_eq!(app.cursor(), 0);
    }
}
