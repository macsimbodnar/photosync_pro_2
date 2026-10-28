//! The `photosync` command: parses arguments, runs the core through `flow`,
//! prints through `text`, and maps the outcome to an exit code. No sync
//! logic lives here (INV-8).

mod args;
mod flow;
mod system;
mod text;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use photosync_core::{Cancel, Volumes, default_search_roots, discover_volumes};

use args::{Cli, Command, EXIT_INCOMPLETE, EXIT_OK, EXIT_REFUSED, Search, SyncArgs};
use flow::{SyncError, SyncRequest};

fn main() -> ExitCode {
    let cli = Cli::parse();
    if system::refuses_user(system::effective_uid()) {
        eprintln!(
            "photosync does not run as root: root can write to system volumes. \
             Run it again as your normal user, without sudo."
        );
        return ExitCode::from(EXIT_REFUSED);
    }
    ExitCode::from(match cli.command {
        Command::Sources(search) => list(
            &search,
            |v| v.sources,
            "No camera card found. Insert a card, or connect the X-T5 in \"USB card reader\" mode.",
        ),
        Command::Drives(search) => list(
            &search,
            |v| v.destinations,
            "No drive to sync to found. Plug in your external drive.",
        ),
        Command::Sync(args) => sync(&args),
    })
}

/// One path per line on stdout, so scripts can read it; the rest on stderr.
fn list(search: &Search, pick: fn(Volumes) -> Vec<PathBuf>, none: &str) -> u8 {
    let roots = if search.search_roots.is_empty() {
        match default_search_roots() {
            Ok(roots) => roots,
            Err(e) => {
                eprintln!("Cannot tell where volumes are mounted: {e}.");
                return EXIT_REFUSED;
            }
        }
    } else {
        search.search_roots.clone()
    };
    let volumes = discover_volumes(&roots);
    for failure in &volumes.failures {
        eprintln!("warning: {failure}");
    }
    let found = pick(volumes);
    if found.is_empty() {
        eprintln!("{none}");
    }
    for path in found {
        println!("{}", path.display());
    }
    EXIT_OK
}

fn sync(args: &SyncArgs) -> u8 {
    let cancel = Cancel::new();
    if let Err(e) = system::cancel_on_ctrl_c(cancel.clone()) {
        eprintln!("Cannot set up Ctrl-C to cancel: {e}. Nothing was copied.");
        return EXIT_REFUSED;
    }
    let request = SyncRequest {
        sources: &args.sources,
        library: &args.dest,
        name: args.name.as_deref(),
    };
    match flow::sync(&request, &cancel, &mut text::StderrWatch::new()) {
        Ok(report) => {
            print!("{}", text::report(&report));
            if text::complete(&report.totals(), report.sources()) {
                EXIT_OK
            } else {
                EXIT_INCOMPLETE
            }
        }
        Err(e) => {
            eprintln!("{}", text::refusal(&e));
            match e {
                // Files may be on the drive, but nothing says which.
                SyncError::Report(_) => EXIT_INCOMPLETE,
                _ => EXIT_REFUSED,
            }
        }
    }
}
