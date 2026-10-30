//! What the binary asks of the OS itself: who it runs as, Ctrl-C, the
//! terminal's width, and writing to stdout and stderr without panicking.

use std::ffi::OsString;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
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

/// One line from stdin, without its newline. `None` at the end of the
/// input, or once Ctrl-C has set `cancel`. Not std's `read_line`: it
/// retries after a signal, so a Ctrl-C would go on waiting for Enter. Byte
/// by byte, so nothing after the line is taken from a script's input.
#[cfg(unix)]
pub fn read_line(cancel: &Cancel) -> io::Result<Option<OsString>> {
    read_line_from(libc::STDIN_FILENO, cancel)
}

/// How long one wait for input lasts before the cancel is checked again.
#[cfg(unix)]
const CANCEL_CHECK_MS: libc::c_int = 100;

#[cfg(unix)]
fn read_line_from(fd: libc::c_int, cancel: &Cancel) -> io::Result<Option<OsString>> {
    let mut line = Vec::new();
    loop {
        // A Ctrl-C that lands between this check and the wait does not
        // interrupt the wait: so the wait is short, and the check comes
        // round again (S022).
        if cancel.is_cancelled() {
            return Ok(None);
        }
        let mut ready = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: polls the one `pollfd` given, which lives across the call.
        match unsafe { libc::poll(&mut ready, 1, CANCEL_CHECK_MS) } {
            0 => continue,
            n if n < 0 => {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            // Readable, at its end (POLLHUP) or in error: the read says which.
            _ => {}
        }
        let mut byte = 0u8;
        // SAFETY: reads at most one byte into `byte`, which lives across
        // the call.
        let n = unsafe { libc::read(fd, (&raw mut byte).cast(), 1) };
        match n {
            1 if byte == b'\n' => break,
            1 => line.push(byte),
            0 if line.is_empty() => return Ok(None),
            0 => break,
            _ => {
                let e = io::Error::last_os_error();
                if e.kind() != io::ErrorKind::Interrupted {
                    return Err(e);
                }
            }
        }
    }
    Ok(Some(OsString::from_vec(line)))
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

    /// `main` hands this to the root refusal: it must be who we really are.
    #[cfg(unix)]
    #[test]
    fn effective_uid_is_what_the_system_says() {
        let out = std::process::Command::new("id").arg("-u").output().unwrap();
        assert!(out.status.success(), "id failed: {out:?}");
        let id: u32 = String::from_utf8(out.stdout).unwrap().trim().parse().unwrap();
        assert_eq!(effective_uid(), id);
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

    /// A pipe: (read end, write end).
    #[cfg(unix)]
    fn pipe() -> (libc::c_int, libc::c_int) {
        let mut fds = [0; 2];
        // SAFETY: pipe writes two fds into `fds`, which lives across the call.
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        (fds[0], fds[1])
    }

    #[cfg(unix)]
    fn close(fd: libc::c_int) {
        // SAFETY: the fd is the test's own, closed once.
        unsafe { libc::close(fd) };
    }

    /// S022: a Ctrl-C that lands just before the wait for input begins
    /// interrupts nothing: only the cancel says it came. The wait must see
    /// it anyway, not wait for a line that may never come. Setting the
    /// cancel with no signal is that case.
    #[cfg(unix)]
    #[test]
    fn a_cancel_set_while_the_prompt_waits_is_never_missed() {
        let (read, write) = pipe();
        let cancel = Cancel::new();
        let (tx, rx) = std::sync::mpsc::channel();
        let waiting = cancel.clone();
        std::thread::spawn(move || tx.send(read_line_from(read, &waiting).unwrap()));
        std::thread::sleep(std::time::Duration::from_millis(200));
        cancel.cancel();
        let got = rx.recv_timeout(std::time::Duration::from_secs(10));
        // Ends a wait that missed the cancel, so the thread does not leak.
        close(write);
        assert_eq!(got.expect("the cancel was missed"), None);
        close(read);
    }

    /// The wait still reads what comes: a line, then the end of the input.
    #[cfg(unix)]
    #[test]
    fn the_prompt_reads_a_line_then_the_end_of_the_input() {
        let (read, write) = pipe();
        // SAFETY: writes from a live buffer of that length.
        assert_eq!(unsafe { libc::write(write, b"/V/B\n".as_ptr().cast(), 5) }, 5);
        let cancel = Cancel::new();
        assert_eq!(read_line_from(read, &cancel).unwrap(), Some(OsString::from("/V/B")));
        close(write);
        assert_eq!(read_line_from(read, &cancel).unwrap(), None);
        close(read);
    }
}
