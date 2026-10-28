//! Command-line arguments and exit codes. No I/O here, so the TUI shares the
//! exit codes.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Every supported file was copied or was already on the drive.
pub const EXIT_OK: u8 = 0;
/// The sync ran, but not every file is known to be on the drive: some
/// failed or were cancelled, or the scan could not read part of a card.
pub const EXIT_INCOMPLETE: u8 = 1;
/// Nothing was copied: a usage error, or the sync was refused before it
/// started (also clap's own code for a usage error).
pub const EXIT_REFUSED: u8 = 2;

const EXIT_CODES: &str = "\
Exit codes:
  0  every photo and video is on the drive (copied, or already there)
  1  some files failed or were cancelled, or part of a card could not be read:
     do not format the card; fix the cause and run the same sync again
  2  usage error, or the sync was refused before copying anything";

#[derive(Debug, Parser)]
#[command(
    name = "photosync",
    version,
    about = "Copies photos from a Fujifilm X-T5 card to an external drive.",
    long_about = "Copies photos from a Fujifilm X-T5 card (in a card reader, or the camera \
                  over USB in \"USB card reader\" mode) to an external drive. RAW, JPEG, HEIF \
                  and video go to separate folders. Photos already on the drive are skipped. \
                  The card is never written to.\n\n\
                  With no command, in a terminal, photosync opens its interactive view (TUI): \
                  pick the cards, the drive and the library folder, follow the copy, read the \
                  report.",
    after_help = EXIT_CODES
)]
pub struct Cli {
    /// None: open the TUI, if stdin and stdout are a terminal.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Lists the mounted camera cards (volumes with a DCIM folder), one per line.
    Sources(Search),
    /// Lists the mounted drives you can sync to, one per line.
    Drives(Search),
    /// Copies the photos of one or more cards into a library folder.
    #[command(after_help = EXIT_CODES)]
    Sync(SyncArgs),
}

#[derive(Debug, clap::Args)]
pub struct Search {
    /// Looks here instead of the platform's mount folders (/Volumes on
    /// macOS). For tests: hidden, as the app only ever offers mounted volumes.
    #[arg(long = "search-root", value_name = "PATH", hide = true)]
    pub search_roots: Vec<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    /// A card to copy from, e.g. /Volumes/X-T5 (see `photosync sources`).
    /// Give it several times to copy several cards in one sync.
    #[arg(long = "source", value_name = "PATH", required = true, num_args = 1..)]
    pub sources: Vec<PathBuf>,
    /// The library folder on the external drive, e.g. /Volumes/SSD/Photos.
    /// It must exist. Never on a card, nor on the same drive as a source.
    #[arg(long = "dest", value_name = "LIBRARY")]
    pub dest: PathBuf,
    /// Names the sync folder: <library>/<date> - <NAME>/RAW/... A folder
    /// with this name from an earlier sync is reused.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn arguments_are_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn exit_codes_in_help_match_the_constants() {
        for code in [EXIT_OK, EXIT_INCOMPLETE, EXIT_REFUSED] {
            assert!(
                EXIT_CODES.contains(&format!("\n  {code}  ")),
                "{code} is not documented"
            );
        }
    }

    #[test]
    fn several_sources_in_one_flag_or_repeated() {
        let one = Cli::try_parse_from(["photosync", "sync", "--source", "a", "b", "--dest", "l"]);
        let two = Cli::try_parse_from([
            "photosync",
            "sync",
            "--source",
            "a",
            "--source",
            "b",
            "--dest",
            "l",
        ]);
        for cli in [one, two] {
            let Some(Command::Sync(args)) = cli.unwrap().command else {
                panic!("not sync")
            };
            assert_eq!(args.sources, [PathBuf::from("a"), PathBuf::from("b")]);
        }
    }
}
