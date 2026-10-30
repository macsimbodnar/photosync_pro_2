//! Helpers shared by the binary's integration tests.

// Each test file is its own crate and uses only some of these.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
#[path = "../../../photosync-core/tests/common/images.rs"]
pub mod images;

/// A fresh directory under the system temp dir, removed on drop.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> TempDir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("photosync-cli-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path); // left over from a killed run
        fs::create_dir_all(&path).unwrap();
        // Resolved, so paths compare equal to what the binary prints.
        TempDir(fs::canonicalize(path).unwrap())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn write_file(root: &Path, rel: &str, bytes: &[u8], modified: SystemTime) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    path
}

pub fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

/// The local `YYYY-MM-DD` that `date(1)` prints for `secs` after the epoch.
pub fn date_command(secs: u64) -> String {
    let mut cmd = Command::new("date");
    if cfg!(target_vendor = "apple") {
        cmd.args(["-r", &secs.to_string()]);
    } else {
        cmd.arg(format!("-d@{secs}"));
    }
    let out = cmd.arg("+%Y-%m-%d").output().unwrap();
    assert!(out.status.success(), "date failed: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Every path under `root` with its mtime, and full contents for files.
pub fn manifest(root: &Path) -> BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)> {
    fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let bytes = meta.is_file().then(|| fs::read(&path).unwrap());
            out.insert(path.clone(), (meta.modified().unwrap(), bytes));
            if meta.is_dir() {
                walk(&path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &mut out);
    out
}

/// A disk image's mount point, resolved. Only the tests that are
/// `#[ignore]`d for their images call this, so a test that needs mounts can
/// never pass with nothing checked: run outside the images, it fails.
pub fn image_path(var: &str) -> PathBuf {
    let Some(path) = std::env::var_os(var) else {
        panic!(
            "{var} is not set: this test needs disk images. On macOS plain `cargo test` runs it \
             on its own images; on Linux see the README."
        );
    };
    fs::canonicalize(path).unwrap()
}

/// The drive image: another device than the temp dir, so a sync of temp
/// folders into it passes INV-10.
pub fn test_drive() -> PathBuf {
    image_path(images::LIBRARY)
}

/// The two card images for twins: a device each, as a twin must be on
/// another device than the file it stands in for (DEC-038), and the temp
/// dir is one device.
pub fn twin_cards() -> (PathBuf, PathBuf) {
    (image_path(images::TWIN_A), image_path(images::TWIN_B))
}

/// A fresh folder on a test image (a library on the drive, or a card on
/// a twin image), removed on drop, failing or not: the images are shared
/// by the tests.
pub struct OnDrive(pub PathBuf);

impl OnDrive {
    pub fn new(drive: &Path, name: &str) -> OnDrive {
        let path = drive.join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path); // left over from a killed run
        fs::create_dir(&path).unwrap();
        OnDrive(path)
    }
}

impl Drop for OnDrive {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A disk outside the images, the one the test binary is on: chmod does nothing on exFAT, so a library with an unreadable
/// file must live here, and the cards on the images keep it on another device.
pub fn disk_of_the_binary() -> PathBuf {
    let binary = std::env::current_exe().unwrap();
    fs::canonicalize(binary.parent().unwrap()).unwrap()
}

/// A library file planning cannot hash: a candidate for any source file of
/// its name and size, so it is a library read problem (specs, Already-synced
/// detection).
#[cfg(unix)]
pub fn lock_in_library(library: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = write_file(library, rel, bytes, at(1_600_000_000));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    path
}

/// Removes `path` once copying starts, after planning read it, so its
/// copy has a read error: chmod does nothing on exFAT, removing does.
/// Passes every call on to `inner`.
pub struct LoseOnCopy<'w> {
    pub path: PathBuf,
    pub inner: &'w mut dyn photosync_core::Watch,
}

impl photosync_core::Watch for LoseOnCopy<'_> {
    fn scanned(&mut self, scan: &photosync_core::SourceScan) {
        self.inner.scanned(scan);
    }

    fn event(&mut self, event: photosync_core::Event) {
        if event.progress.phase == photosync_core::Phase::Copying
            && event.kind == photosync_core::EventKind::Started
        {
            fs::remove_file(&self.path).unwrap();
        }
        self.inner.event(event);
    }
}

/// What one run of the binary gave.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// The last line on stderr: a refusal's sentence comes after the scans.
    pub fn last_err_line(&self) -> &str {
        self.stderr.lines().last().unwrap_or("")
    }
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit {}\n--- stdout\n{}--- stderr\n{}",
            self.code, self.stdout, self.stderr
        )
    }
}

/// Not `env!`: the binary's own unit tests share this file, and Cargo
/// names the binary only to integration tests.
fn binary() -> &'static str {
    match option_env!("CARGO_BIN_EXE_photosync") {
        Some(path) => path,
        None => panic!("only integration tests run the binary"),
    }
}

pub fn photosync<I, S>(args: I) -> Run
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(binary()).args(args).output().unwrap();
    Run {
        // A signal death has no code; -1 fails every exit code check.
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

/// A pseudo-terminal, so the binary's stdin is a terminal and its prompt
/// asks, as for a person at the keyboard: the test types into `keys`. No
/// echo, as nobody reads it back.
#[cfg(unix)]
pub struct Pty {
    pub keys: File,
    terminal: Option<File>,
}

#[cfg(unix)]
impl Pty {
    pub fn open() -> Pty {
        use std::os::fd::FromRawFd;

        let (mut keys, mut terminal) = (0, 0);
        // SAFETY: openpty writes two fds into the ints given, which live
        // across the call; name, termios and winsize may be null.
        let rc = unsafe {
            libc::openpty(
                &mut keys,
                &mut terminal,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(rc, 0, "openpty: {}", std::io::Error::last_os_error());
        // SAFETY: both fds are new and ours; `mode` is fully written by
        // tcgetattr before it is changed and set.
        unsafe {
            // The binary must not hold the keys' side: it is only ours.
            // Not atomic with openpty: a parallel test's child may inherit
            // these for a moment. Nothing waits for the pty to close, so
            // that costs at most a delay on a failure path.
            libc::fcntl(keys, libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(terminal, libc::F_SETFD, libc::FD_CLOEXEC);
            let mut mode: libc::termios = std::mem::zeroed();
            assert_eq!(libc::tcgetattr(terminal, &mut mode), 0);
            mode.c_lflag &= !libc::ECHO;
            assert_eq!(libc::tcsetattr(terminal, libc::TCSANOW, &mode), 0);
            Pty {
                keys: File::from_raw_fd(keys),
                terminal: Some(File::from_raw_fd(terminal)),
            }
        }
    }

    /// The terminal's side, for the binary's stdin. Once.
    pub fn stdin(&mut self) -> std::process::Stdio {
        self.terminal.take().expect("stdin taken twice").into()
    }

    /// Ctrl-D at the start of a line: the end of the input.
    pub fn end_input(&mut self) {
        use std::io::Write;
        // Same EIO as above when the binary has already exited.
        let _ = self.keys.write_all(&[0x04]);
    }
}

/// `photosync(args)` on a terminal: `input` typed, then Ctrl-D, the end
/// of the input. Fails the test, and kills the binary, if it runs for over
/// a minute: a prompt that waits for input nobody gives would hang the
/// suite.
#[cfg(unix)]
pub fn photosync_tty<I, S>(args: I, input: &[u8]) -> Run
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    use std::io::Write;
    use std::process::Stdio;

    let mut pty = Pty::open();
    let child = Command::new(binary())
        .args(args)
        .stdin(pty.stdin())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    // Input past the pty's line buffer (MAX_INPUT, ~1 KiB on macOS) is
    // dropped silently, so keep a test's typed input well below that.
    assert!(input.len() < 512, "pty input too long to arrive whole");
    // A binary that exits before reading it all makes the write fail with
    // EIO; that is its business, and the Run below still shows what it did.
    let _ = pty.keys.write_all(input);
    pty.end_input();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || tx.send(child.wait_with_output().unwrap()));
    let Ok(Output {
        status,
        stdout,
        stderr,
    }) = rx.recv_timeout(Duration::from_secs(60))
    else {
        let _ = Command::new("kill").arg(pid.to_string()).status();
        panic!("photosync hangs on its input");
    };
    // Kept open to the end: a closed terminal is not the end of the input.
    drop(pty);
    Run {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}
