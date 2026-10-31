//! The native GUI, a thin shell over the model, worker and sentences the TUI
//! uses (`photosync_front`), so no rule lives here (DEC-067, DEC-068).

pub mod args;
pub mod gui;
pub mod png;
mod widgets;

use std::ffi::OsString;
use std::sync::Arc;

use photosync_core::{default_search_roots, discover_volumes};
use photosync_front::args::{EXIT_OK, EXIT_REFUSED};
use photosync_front::system::say;

/// The whole run: what the binary returns. S056 adds root and the report.
pub fn run(arguments: impl Iterator<Item = OsString>) -> u8 {
    let options = match args::parse(arguments) {
        Ok(options) => options,
        Err(sentence) => {
            say(&sentence);
            return EXIT_REFUSED;
        }
    };
    let roots = if options.search_roots.is_empty() {
        match default_search_roots() {
            Ok(roots) => roots,
            Err(e) => {
                say(&format!("Cannot tell where volumes are mounted: {e}."));
                return EXIT_REFUSED;
            }
        }
    } else {
        options.search_roots
    };
    let discover: gui::Discover = Arc::new(move || discover_volumes(&roots));
    match gui::program(discover, options.smoke_png).run() {
        Ok(()) => EXIT_OK,
        Err(e) => {
            say(&format!("The window could not open: {e}. Nothing was copied."));
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
