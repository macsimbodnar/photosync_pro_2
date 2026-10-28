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

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use photosync_core::{Cancel, Volumes, default_search_roots, discover_volumes};

use args::{Cli, Command, EXIT_INCOMPLETE, EXIT_OK, EXIT_REFUSED, Search, SyncArgs};
use flow::{SyncError, SyncRequest};

fn main() -> ExitCode {
    let cli = Cli::parse();
    if system::refuses_user(system::effective_uid()) {
        system::say(
            "photosync does not run as root: root can write to system volumes. \
             Run it again as your normal user, without sudo.",
        );
        return ExitCode::from(EXIT_REFUSED);
    }
    ExitCode::from(match cli.command {
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
    })
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
    let request = SyncRequest {
        sources: &args.sources,
        library: &args.dest,
        name: args.name.as_deref(),
    };
    match flow::sync(&request, &cancel, &mut text::StderrWatch::new()) {
        Ok(report) => {
            let code = if text::complete(&report.totals(), report.sources()) {
                EXIT_OK
            } else {
                EXIT_INCOMPLETE
            };
            print_report(&mut io::stdout().lock(), &text::report(&report), code)
        }
        Err(e) => {
            system::say(&text::refusal(&e));
            match e {
                // Files may be on the drive, but nothing says which.
                SyncError::Report(_) => EXIT_INCOMPLETE,
                _ => EXIT_REFUSED,
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
