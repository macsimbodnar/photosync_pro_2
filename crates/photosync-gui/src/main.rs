//! The GUI's binary. All of it is in the library, where the tests are (DEC-072 (1)).

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(photosync_gui::run(
        std::env::args_os().skip(1),
        photosync_front::system::effective_uid(),
    ))
}
