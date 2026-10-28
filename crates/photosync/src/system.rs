//! What the binary asks of the OS itself: who it runs as, Ctrl-C, the
//! terminal's width, and writing to stdout and stderr without panicking.

use std::io::{self, Write};
use std::sync::OnceLock;

use photosync_core::Cancel;

/// Root may write to system volumes, such as `/Volumes/Recovery`, that
/// discovery leaves out only because the user cannot write there (DEC-024
/// (3)). So the app never runs as root (S007 fast check).
pub fn refuses_user(euid: u32) -> bool {
    euid == 0
}

#[cfg(unix)]
pub fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

static CANCEL: OnceLock<Cancel> = OnceLock::new();

/// Makes the first Ctrl-C (SIGINT) set `cancel`, so the sync stops between
/// files or chunks and still reports. The handler then resets, so a second
/// Ctrl-C kills the process at once, which INV-7 makes safe too. Once per
/// process.
#[cfg(unix)]
pub fn cancel_on_ctrl_c(cancel: Cancel) -> io::Result<()> {
    CANCEL
        .set(cancel)
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "Ctrl-C handler already set"))?;
    // SAFETY: the handler only does an atomic load (`OnceLock::get` after
    // the set above) and an atomic store, both async-signal-safe. `action`
    // is fully initialised before `sigaction` reads it.
    let rc = unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_sigint as extern "C" fn(libc::c_int) as libc::sighandler_t;
        action.sa_flags = libc::SA_RESETHAND;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut())
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
extern "C" fn on_sigint(_: libc::c_int) {
    if let Some(cancel) = CANCEL.get() {
        cancel.cancel();
    }
}

/// The width in columns of the terminal stderr writes to, if it is one.
#[cfg(unix)]
pub fn stderr_width() -> Option<usize> {
    // SAFETY: TIOCGWINSZ only writes a `winsize` into `size`, which is
    // zeroed and lives across the call.
    let size = unsafe {
        let mut size: libc::winsize = std::mem::zeroed();
        (libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut size) == 0).then_some(size)
    }?;
    (size.ws_col > 0).then_some(usize::from(size.ws_col))
}

/// Writes `text` to stdout. `print!` panics when stdout is closed (e.g.
/// `| head`), which would end the process with code 101; this returns the
/// error, so the caller keeps the exit code within 0/1/2.
pub fn write_out(text: &str) -> io::Result<()> {
    write_all(&mut io::stdout().lock(), text)
}

pub fn write_all(out: &mut impl Write, text: &str) -> io::Result<()> {
    out.write_all(text.as_bytes())?;
    out.flush()
}

/// Writes one line to stderr. A closed stderr is not an error to report:
/// there is nowhere left to report it, and the exit code still says what
/// happened. `eprintln!` would panic instead.
pub fn say(line: &str) {
    let _ = writeln!(io::stderr().lock(), "{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_refused_and_other_users_are_not() {
        assert!(refuses_user(0));
        assert!(!refuses_user(1));
        assert!(!refuses_user(501));
    }

    /// The only test in this binary that installs the handler: a second
    /// SIGINT would kill the test process.
    #[cfg(unix)]
    #[test]
    fn ctrl_c_sets_the_cancel() {
        let cancel = Cancel::new();
        cancel_on_ctrl_c(cancel.clone()).unwrap();
        assert!(!cancel.is_cancelled());

        // SAFETY: raise has no preconditions; the handler is installed.
        assert_eq!(unsafe { libc::raise(libc::SIGINT) }, 0);

        assert!(cancel.is_cancelled());
        assert_eq!(
            cancel_on_ctrl_c(Cancel::new()).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
    }
}
