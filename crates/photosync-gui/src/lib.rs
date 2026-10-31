//! The native GUI, a thin shell over the model, worker and sentences the TUI
//! uses (`photosync_front`), so no rule lives here (DEC-067, DEC-068).

pub mod args;
pub mod gui;
pub mod png;
mod widgets;

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, PoisonError};

use photosync_core::{default_search_roots, discover_volumes};
use photosync_front::app::Exit;
use photosync_front::args::{EXIT_INCOMPLETE, EXIT_REFUSED};
use photosync_front::system::{self, say};

use crate::gui::{Discover, ExitSlot, Opening};

/// The whole run, for the user `euid`: what the binary returns.
pub fn run(arguments: impl Iterator<Item = OsString>, euid: u32) -> u8 {
    let options = match args::parse(arguments) {
        Ok(options) => options,
        Err(sentence) => {
            say(&sentence);
            return EXIT_REFUSED;
        }
    };
    let options = args::with_seam(options, |name| std::env::var_os(name));
    let search_roots = options.search_roots;
    let opening = opening(euid, || discovery(search_roots), &mut say);
    let slot = ExitSlot::default();
    let result = gui::program_of(opening, options.smoke_png, slot.clone()).run();
    after_the_window(result, &slot, &mut io::stdout().lock(), &mut say)
}

/// How a run finds volumes: under the roots given, or the system's places.
fn discovery(search_roots: Vec<PathBuf>) -> Result<Discover, String> {
    let roots = if search_roots.is_empty() {
        default_search_roots()
            .map_err(|e| format!("Cannot tell where volumes are mounted: {e}."))?
    } else {
        search_roots
    };
    Ok(Arc::new(move || discover_volumes(&roots)))
}

/// What a run opens on, for the user `euid`: a parameter, as no test runs as root (DEC-047).
/// A refusal goes to stderr first, so it is said even if no window can open. Root must not
/// touch a volume, so `find` is not called (DEC-069 (7)).
pub fn opening(
    euid: u32,
    find: impl FnOnce() -> Result<Discover, String>,
    say: &mut impl FnMut(&str),
) -> Opening {
    let found = if system::refuses_user(euid) {
        Err(system::ROOT_REFUSAL.to_string())
    } else {
        find()
    };
    match found {
        Ok(discover) => Opening::Look(discover),
        Err(sentence) => {
            say(&sentence);
            Opening::Refuse(sentence)
        }
    }
}

/// What the process says and returns once the window has quit, as `interactive()` in the TUI
/// does. The sync is over whatever happens to its report: a closed stdout loses the text, not
/// the code. `print_report` is private to the CLI's binary (DEC-072 (3)).
pub fn ending(exit: &Exit, out: &mut impl Write, say: &mut impl FnMut(&str)) -> u8 {
    if let Some(message) = &exit.message {
        say(message);
    }
    if let Some(report) = &exit.report
        && let Err(e) = system::write_all(out, report)
    {
        say(&format!(
            "Cannot write the report to stdout: {e}. The sync is over; its exit code is {}.",
            exit.code
        ));
    }
    exit.code
}

/// A window that ended with no quit heard: nothing says what is on the drive (DEC-069 (2)).
pub fn no_exit_recorded() -> Exit {
    Exit {
        code: EXIT_INCOMPLETE,
        report: None,
        message: Some(
            "The window closed with no report: not every file is known to be on the drive. Do not format the card."
                .to_string(),
        ),
    }
}

/// What `run` does with the window's end. An `Exit` recorded is what happened, so a failure
/// after it does not say that nothing was copied.
pub fn after_the_window(
    result: Result<(), iced::Error>,
    slot: &ExitSlot,
    out: &mut impl Write,
    say: &mut impl FnMut(&str),
) -> u8 {
    let recorded = slot.lock().unwrap_or_else(PoisonError::into_inner).take();
    match (result, recorded) {
        (Ok(()), recorded) => ending(&recorded.unwrap_or_else(no_exit_recorded), out, say),
        (Err(e), Some(exit)) => {
            say(&format!("The window stopped: {e}."));
            ending(&exit, out, say)
        }
        (Err(e), None) => {
            say(&format!(
                "The window could not open: {e}. Nothing was copied."
            ));
            EXIT_REFUSED
        }
    }
}

// The binary's unit tests use this too, with the images' helpers (DEC-067 (6)).
#[cfg(test)]
#[path = "../../photosync/tests/common/mod.rs"]
mod common;

#[cfg(test)]
mod tests;
