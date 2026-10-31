//! The `photosync` command: parses arguments, runs the core through `flow`,
//! prints through `text`, and maps the outcome to an exit code. With no
//! command it opens the TUI (`tui`, over the model in `app`). No sync logic
//! lives here (INV-8).

mod app;
mod args;
mod flow;
mod system;
mod text;
mod tui;
mod worker;

// One instance for all unit tests: `TempDir` numbers its folders from a
// static, and two copies of it would hand out the same paths.
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod common;

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use photosync_core::{Cancel, RunEnd, Session, Volumes, default_search_roots, discover_volumes};

use args::{Cli, Command, EXIT_INCOMPLETE, EXIT_OK, EXIT_REFUSED, Search, SyncArgs, exit_code};

fn main() -> ExitCode {
    ExitCode::from(run(Cli::parse(), system::effective_uid()))
}

/// The command `cli` names, for the user `euid`. Not in `main`, so a test
/// reaches the refusal: no test runs as root (DEC-047).
fn run(cli: Cli, euid: u32) -> u8 {
    if system::refuses_user(euid) {
        system::say(system::ROOT_REFUSAL);
        return EXIT_REFUSED;
    }
    match cli.command {
        None => interactive(),
        Some(Command::Sources(search)) => list(
            &search,
            |v| v.sources,
            "No camera card found. Insert a card, or connect the X-T5 in \"USB card reader\" mode.",
        ),
        Some(Command::Drives(search)) => list(
            &search,
            |v| v.destinations,
            "No drive to sync to found. Plug in your external drive.",
        ),
        Some(Command::Sync(args)) => sync(&args),
    }
}

fn search_roots() -> Result<Vec<PathBuf>, u8> {
    default_search_roots().map_err(|e| {
        system::say(&format!("Cannot tell where volumes are mounted: {e}."));
        EXIT_REFUSED
    })
}

/// The TUI, then the report on stdout, so it outlives the screen.
fn interactive() -> u8 {
    if !(io::stdin().is_terminal() && io::stdout().is_terminal()) {
        // Without a terminal the TUI would wait for keys nobody can type.
        Cli::command()
            .error(
                ErrorKind::MissingSubcommand,
                "no command given, and the interactive view needs a terminal",
            )
            .exit();
    }
    let roots = match search_roots() {
        Ok(roots) => roots,
        Err(code) => return code,
    };
    match tui::run(roots) {
        Ok(app) => {
            let exit = app.exit();
            if let Some(message) = &exit.message {
                system::say(message);
            }
            match &exit.report {
                Some(report) => print_report(&mut io::stdout().lock(), report, exit.code),
                None => exit.code,
            }
        }
        Err(tui::Failed::Start(e)) => {
            system::say(&format!(
                "Cannot open the interactive view: {e}. Nothing was copied."
            ));
            EXIT_REFUSED
        }
        Err(tui::Failed::Running(e)) => {
            system::say(&format!(
                "The interactive view stopped: {e}. If a copy was running, not every file \
                 is known to be on the drive: do not format the card, and run the same \
                 sync again."
            ));
            EXIT_INCOMPLETE
        }
    }
}

/// The sync is over whatever happens to its report: a closed stdout loses
/// the text, not the exit code, which still says if the card is safe.
fn print_report(out: &mut impl Write, report: &str, code: u8) -> u8 {
    if let Err(e) = system::write_all(out, report) {
        system::say(&format!(
            "Cannot write the report to stdout: {e}. The sync is over; its exit code is {code}."
        ));
    }
    code
}

/// One path per line on stdout, so scripts can read it; the rest on stderr.
fn list(search: &Search, pick: fn(Volumes) -> Vec<PathBuf>, none: &str) -> u8 {
    let roots = if search.search_roots.is_empty() {
        match search_roots() {
            Ok(roots) => roots,
            Err(code) => return code,
        }
    } else {
        search.search_roots.clone()
    };
    let volumes = discover_volumes(&roots);
    for failure in &volumes.failures {
        system::say(&format!("warning: {failure}"));
    }
    let found = pick(volumes);
    if found.is_empty() {
        system::say(none);
    }
    let text: String = found.iter().map(|p| format!("{}\n", p.display())).collect();
    if let Err(e) = system::write_out(&text) {
        // The list is this command's whole output: lost, it is not done.
        system::say(&format!("Cannot write the list to stdout: {e}."));
        return EXIT_INCOMPLETE;
    }
    EXIT_OK
}

fn sync(args: &SyncArgs) -> u8 {
    let cancel = Cancel::new();
    if let Err(e) = system::cancel_on_ctrl_c(cancel.clone()) {
        system::say(&format!(
            "Cannot set up Ctrl-C to cancel: {e}. Nothing was copied."
        ));
        return EXIT_REFUSED;
    }
    let mut session = match flow::start(&args.dest, args.name.as_deref()) {
        Ok(session) => session,
        Err(e) => {
            system::say(&text::refusal(&e));
            return EXIT_REFUSED;
        }
    };
    // With no terminal on stdin nobody can swap a card: a pipe may stay
    // open, so waiting on it could last for ever (DEC-035).
    let prompt = io::stdin().is_terminal();
    sync_cards(
        &mut session,
        args.sources.clone(),
        args.one_by_one,
        &cancel,
        |which| prompt.then(|| next_cards(args, &cancel, which)).flatten(),
    )
}

/// The syncs of `session`: `sources`, then with `one_by_one` each set of
/// cards `next` gives, until it gives none; `next("first")` asks for the
/// first cards when `sources` is empty. Each sync's end is printed as it
/// comes, then the session's summary after several. The session's exit code.
fn sync_cards(
    session: &mut Session,
    mut sources: Vec<PathBuf>,
    one_by_one: bool,
    cancel: &Cancel,
    mut next: impl FnMut(&str) -> Option<Vec<PathBuf>>,
) -> u8 {
    // A Ctrl-C while the cards are looked for does not stop the look; it
    // must still stop the session before their sync, which would only be
    // a cancelled one, recorded as not complete (S022).
    let mut next = |which: &str| next(which).filter(|_| !cancel.is_cancelled());
    if sources.is_empty() {
        match next("first") {
            Some(cards) => sources = cards,
            None => {
                system::say("No card was synced.");
                return EXIT_REFUSED;
            }
        }
    }
    loop {
        session.sync(&sources, cancel, &mut text::StderrWatch::new());
        if let Some(code) = print_last_run(session) {
            // The report could not be written: say no more, keep the code.
            if !one_by_one {
                return code;
            }
        }
        // A Ctrl-C during a sync stops the session too.
        if !one_by_one || cancel.is_cancelled() {
            break;
        }
        match next("next") {
            Some(cards) => sources = cards,
            None => break,
        }
    }
    let code = exit_code(session.verdict());
    if session.runs().len() > 1 {
        return print_report(
            &mut io::stdout().lock(),
            &text::session_summary(session),
            code,
        );
    }
    code
}

/// The session's last sync, as it ended: its report on stdout, or why it
/// has none on stderr. `Some(code)` if the report could not be written.
fn print_last_run(session: &Session) -> Option<u8> {
    let code = exit_code(session.verdict());
    match session.runs().last().map(|run| &run.end) {
        Some(RunEnd::Report(report)) => {
            let text = text::report_in(report, session.library(), session.folder());
            if let Err(e) = system::write_all(&mut io::stdout().lock(), &text) {
                system::say(&format!(
                    "Cannot write the report to stdout: {e}. The sync is over; its exit code is {code}."
                ));
                return Some(code);
            }
        }
        Some(RunEnd::Refused(e)) => system::say(&text::refusal(e)),
        Some(RunEnd::Crashed(what)) => system::say(&text::crash(what)),
        None => {}
    }
    None
}

/// Asks on stderr for the next cards and reads the answer from stdin:
/// Enter alone takes every mounted card, a line names one card, `q` or the
/// end of the input (Ctrl-D, or Ctrl-C) finishes, `None`. Only asked with
/// a terminal on stdin; without one (`</dev/null`, a pipe) the session
/// ends after the given cards, so a script never hangs.
fn next_cards(args: &SyncArgs, cancel: &Cancel, which: &str) -> Option<Vec<PathBuf>> {
    loop {
        system::say(&format!(
            "Insert the {which} card, then press Enter (or type its path). q, or Ctrl-D, finishes."
        ));
        let line = match system::read_line(cancel) {
            Ok(Some(line)) => line,
            Ok(None) => return None,
            Err(e) => {
                system::say(&format!(
                    "Cannot read the answer: {e}. The session is over."
                ));
                return None;
            }
        };
        match line.to_string_lossy().trim() {
            "q" => return None,
            "" => {}
            // As typed: a path may hold spaces.
            _ => return Some(vec![PathBuf::from(line)]),
        }
        let roots = if args.search.search_roots.is_empty() {
            match search_roots() {
                Ok(roots) => roots,
                Err(_) => continue,
            }
        } else {
            args.search.search_roots.clone()
        };
        let volumes = discover_volumes(&roots);
        for failure in &volumes.failures {
            system::say(&format!("warning: {failure}"));
        }
        if volumes.sources.is_empty() {
            system::say("No camera card found.");
            continue;
        }
        let found: Vec<String> = volumes
            .sources
            .iter()
            .map(|s| s.display().to_string())
            .collect();
        system::say(&format!("found {}", found.join(", ")));
        return Some(volumes.sources);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Like a pipe whose reader is gone.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    #[test]
    fn a_report_that_cannot_be_written_keeps_the_exit_code() {
        for code in [EXIT_OK, EXIT_INCOMPLETE] {
            assert_eq!(print_report(&mut Closed, "Report\n", code), code);
        }
        let mut out = Vec::new();
        assert_eq!(print_report(&mut out, "Report\n", EXIT_OK), EXIT_OK);
        assert_eq!(out, b"Report\n");
    }

    /// F04: the tests in `src` that need real mounts, in the plain gate.
    #[cfg(target_os = "macos")]
    #[test]
    fn ignored_image_tests_pass_on_real_mounts() {
        use crate::common::images::{CARD, LIBRARY, TWIN_A, TWIN_B, ignored_tests_pass_on_images};
        ignored_tests_pass_on_images(
            &[CARD, LIBRARY, TWIN_A, TWIN_B],
            &[
                "tui::tests::tui_sync_on_exfat_end_to_end",
                "text::tests::a_session_names_the_card_a_file_was_copied_from_its_twin_for",
                "app::tests::the_result_box_names_the_card_a_file_was_copied_from_its_twin_for",
                "app::tests::a_refused_sync_then_the_same_card_complete_exits_0",
                "app::tests::a_refused_sync_of_two_cards_keeps_the_card_not_completed",
            ],
        );
    }

    /// S030 (M13): root gets the refusal whatever the command, and the same
    /// command as another user runs. No command last: past a removed guard
    /// it would open the TUI, so it is reached only if the others held.
    #[test]
    fn root_is_refused_before_any_command_runs() {
        let dir = crate::common::TempDir::new();
        let (card, lib) = (dir.0.join("card"), dir.0.join("lib"));
        std::fs::create_dir_all(&lib).unwrap();
        let parse = |args: &[&str]| Cli::parse_from(args);
        let root = dir.0.to_str().unwrap();
        let drives = ["photosync", "drives", "--search-root", root];
        let sources = ["photosync", "sources", "--search-root", root];
        assert_eq!(run(parse(&drives), 501), EXIT_OK);
        assert_eq!(run(parse(&sources), 501), EXIT_OK);

        assert_eq!(run(parse(&drives), 0), EXIT_REFUSED);
        assert_eq!(run(parse(&sources), 0), EXIT_REFUSED);
        let sync = [
            "photosync",
            "sync",
            "--source",
            card.to_str().unwrap(),
            "--dest",
            lib.to_str().unwrap(),
        ];
        assert_eq!(run(parse(&sync), 0), EXIT_REFUSED);
        assert_eq!(run(parse(&["photosync"]), 0), EXIT_REFUSED);
    }

    /// S022: a Ctrl-C while the next cards are looked for (Enter, then
    /// discovery) stops the session before their sync: none runs, none is
    /// recorded. For the first cards nothing was synced, which is not a
    /// success.
    #[test]
    fn a_cancel_while_looking_for_cards_runs_no_sync() {
        let gone = PathBuf::from("/photosync-test/no-such-card");
        for given in [vec![], vec![gone.clone()]] {
            let mut session = Session::new(Path::new("/photosync-test/lib"), None).unwrap();
            let cancel = Cancel::new();
            let mut asked = 0;
            let code = sync_cards(&mut session, given.clone(), true, &cancel, |_| {
                asked += 1;
                // The Ctrl-C lands while discovery runs; it finds a card.
                cancel.cancel();
                Some(vec![gone.clone()])
            });
            assert_eq!(asked, 1, "{given:?}");
            assert_eq!(session.runs().len(), given.len(), "{given:?}: a sync ran");
            assert_ne!(code, EXIT_OK, "{given:?}");
        }
    }
}
