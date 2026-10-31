//! The binary's arguments, read by hand: the only two are hidden, for tests, so
//! the GUI needs no parser of its own (no new dependency). The same two can come from
//! the environment, so a test can launch the app with no arguments of its own, as Finder does.

use std::ffi::OsString;
use std::path::PathBuf;

const NO_ARGUMENTS: &str = "photosync-gui takes no arguments.";
/// What macOS may pass to an app launched from Finder: an agent's memory, as `open` passes
/// none here and Finder is not measured (DEC-090 (1)). A refusal there would be no window
/// and a sentence nobody sees.
const FINDER_PREFIX: &[u8] = b"-psn_";
/// `--search-root`, for a launch with no arguments of its own (S068).
pub const SEAM_SEARCH_ROOT: &str = "PHOTOSYNC_GUI_SEAM_SEARCH_ROOT";
/// `--smoke-png`, for a launch with no arguments of its own (S068).
pub const SEAM_SMOKE_PNG: &str = "PHOTOSYNC_GUI_SEAM_SMOKE_PNG";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Where to look for volumes, in place of the system's places (DEC-068 (5)).
    pub search_roots: Vec<PathBuf>,
    /// Write a picture of the real window here, then quit (DEC-072 (2)).
    pub smoke_png: Option<PathBuf>,
}

/// `Err` holds the sentence for stderr. Refused rather than ignored: `--smoke-png`
/// spelt wrong would leave a window open with nobody to close it.
pub fn parse(arguments: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        let is = |name: &str| argument.to_str() == Some(name);
        if is("--search-root") {
            let Some(root) = arguments.next() else {
                return Err(NO_ARGUMENTS.to_string());
            };
            options.search_roots.push(PathBuf::from(root));
        } else if is("--smoke-png") {
            let Some(path) = arguments.next() else {
                return Err(NO_ARGUMENTS.to_string());
            };
            options.smoke_png = Some(PathBuf::from(path));
        } else if argument.as_encoded_bytes().starts_with(FINDER_PREFIX) {
            continue;
        } else {
            return Err(NO_ARGUMENTS.to_string());
        }
    }
    Ok(options)
}

/// Fills what the arguments left empty from the seam's variables, read by `read`.
pub fn with_seam(mut options: Options, read: impl Fn(&str) -> Option<OsString>) -> Options {
    // Set but empty is unset: an empty root says "No camera card found" with a card
    // mounted, and an empty picture path says "Cannot write" and gives 2 (DEC-090 (6)).
    let read = |name: &str| read(name).filter(|value| !value.is_empty());
    if options.search_roots.is_empty()
        && let Some(root) = read(SEAM_SEARCH_ROOT)
    {
        options.search_roots.push(PathBuf::from(root));
    }
    if options.smoke_png.is_none() {
        options.smoke_png = read(SEAM_SMOKE_PNG).map(PathBuf::from);
    }
    options
}
