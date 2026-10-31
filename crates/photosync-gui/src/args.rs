//! The binary's arguments, read by hand: the only two are hidden, for tests, so
//! the GUI needs no parser of its own (no new dependency).

use std::ffi::OsString;
use std::path::PathBuf;

const NO_ARGUMENTS: &str = "photosync-gui takes no arguments.";

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
        } else {
            return Err(NO_ARGUMENTS.to_string());
        }
    }
    Ok(options)
}
