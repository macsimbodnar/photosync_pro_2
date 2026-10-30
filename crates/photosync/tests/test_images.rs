//! The images the tests mount for themselves must never stay attached, and
//! a hang must end: the gate runs on the user's own machine, and macOS's
//! disk-image service has hung on a test image before. These check the
//! helper in `tests/common/images.rs` itself. Not on a test image's temp dir:
//! `scripts/test_exfat.sh` skips them.

#![cfg(target_os = "macos")]

mod common;

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use common::TempDir;
use common::images::{CARD, Images, attached_under, limited, watchdog_command};

/// Waits for the images under `work` to be gone, and the folder with them.
fn gone_within(work: &Path, limit: Duration) -> bool {
    let log = TempDir::new();
    let start = Instant::now();
    while start.elapsed() < limit {
        let attached = attached_under(work, &log.0).unwrap();
        if attached.is_empty() && !work.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

#[test]
fn a_panic_still_detaches_the_images() {
    let work: Mutex<Option<PathBuf>> = Mutex::new(None);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let images = Images::mount(&[CARD]).unwrap();
        *work.lock().unwrap() = Some(images.work().to_path_buf());
        let log = TempDir::new();
        assert!(!attached_under(images.work(), &log.0).unwrap().is_empty());
        panic!("a test fails with the images attached");
    }));
    assert!(result.is_err(), "the panic did not happen");
    let work = work.lock().unwrap().clone().expect("never mounted");
    assert!(
        gone_within(&work, Duration::from_secs(5)),
        "{} stays",
        work.display()
    );
}

/// Run by the next test, killed by it: no panic, no `Drop`.
#[test]
#[ignore = "a helper of a_killed_test_binary_leaves_no_image_attached"]
fn holds_the_images_until_killed() {
    let tell = std::env::var_os("PHOTOSYNC_TEST_TELL").expect("only the next test runs this");
    let images = Images::mount(&[]).unwrap();
    // Renamed into place: the next test reads it the moment the name exists.
    let tell = PathBuf::from(tell);
    let written = tell.with_extension("tmp");
    fs::write(&written, images.work().as_os_str().as_encoded_bytes()).unwrap();
    fs::rename(&written, &tell).unwrap();
    std::thread::sleep(Duration::from_secs(120));
    panic!("nobody killed this test");
}

/// SIGKILLed on drop, so a failed assert does not leave it holding images
/// until its sleep ends.
struct Holder(Child);

impl Drop for Holder {
    fn drop(&mut self) {
        let _ = self.0.kill(); // SIGKILL: nothing in it runs
        let _ = self.0.wait();
    }
}

#[test]
fn a_killed_test_binary_leaves_no_image_attached() {
    let dir = TempDir::new();
    let tell = dir.0.join("work");
    let mut holder = Holder(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "holds_the_images_until_killed"])
            .env("PHOTOSYNC_TEST_TELL", &tell)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let start = Instant::now();
    while !tell.exists() {
        assert!(start.elapsed() < Duration::from_secs(120), "never mounted");
        assert!(
            holder.0.try_wait().unwrap().is_none(),
            "the holder ended early"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let work = PathBuf::from(fs::read_to_string(&tell).unwrap());
    assert!(work.exists());

    drop(holder);

    assert!(
        gone_within(&work, Duration::from_secs(60)),
        "{} stays",
        work.display()
    );
}

#[test]
fn a_command_that_hangs_is_killed_and_named() {
    let dir = TempDir::new();
    let start = Instant::now();
    let err = limited(
        Command::new("sleep").arg("100"),
        Duration::from_secs(1),
        "the sleeper",
        &dir.0,
    )
    .unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(30), "it waited");
    assert!(err.starts_with("the sleeper did not end in 1s"), "{err}");
}

/// An hdiutil that never ends, with a helper of its own that would hold a
/// pipe open. Each run's helper is written to FAKE_PIDS.
const HUNG_HDIUTIL: &str = r#"#!/bin/sh
echo "$*" >>"$FAKE_CALLS"
sleep 100 &
echo $! >>"$FAKE_PIDS"
wait
"#;

#[test]
fn a_hung_hdiutil_ends_the_watchdog_and_it_says_so() {
    let dir = TempDir::new();
    let bin = dir.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let fake = bin.join("hdiutil");
    fs::write(&fake, HUNG_HDIUTIL).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let work = dir.0.join("work");
    fs::create_dir(&work).unwrap();
    let (calls, pids) = (dir.0.join("calls"), dir.0.join("pids"));

    // `limited` gives it no stdin: to the watchdog, the test binary just died.
    limited(
        watchdog_command(&work, &[work.join("mnt")], Duration::from_secs(1))
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("FAKE_CALLS", &calls)
            .env("FAKE_PIDS", &pids),
        Duration::from_secs(30),
        "the watchdog",
        &dir.0,
    )
    .unwrap();

    let calls = fs::read_to_string(&calls).unwrap();
    for call in ["detach", "-force", "info"] {
        assert!(calls.contains(call), "no {call} in:\n{calls}");
    }
    assert!(work.exists(), "nothing shows the images are gone: it stays");
    let log = fs::read_to_string(work.join("watchdog.log")).unwrap();
    for said in [
        "hdiutil detach",
        "hdiutil info",
        "did not end in 1s",
        "left",
    ] {
        assert!(log.contains(said), "no \"{said}\" in:\n{log}");
    }

    let start = Instant::now();
    for pid in fs::read_to_string(&pids).unwrap().lines() {
        let pid: libc::pid_t = pid.parse().unwrap();
        // SAFETY: kill takes two plain ints, and signal 0 sends nothing.
        while unsafe { libc::kill(pid, 0) } == 0 {
            assert!(start.elapsed() < Duration::from_secs(5), "{pid} lives on");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// An hdiutil that ends well the moment its first helper dies, as a real one
/// might. A slow `pgrep` makes the watchdog's kill take its time, so this holds
/// on every run, not only when a loaded machine happens to slow it down.
const ENDS_WELL_HDIUTIL: &str = r#"#!/bin/sh
echo "$*" >>"$FAKE_CALLS"
[ "$1" = detach ] || exit 1
sleep 100 &
first=$!
sleep 100 &
echo "$first $!" >>"$FAKE_PIDS"
wait "$first"
exit 0
"#;
const SLOW_PGREP: &str = "#!/bin/sh\nsleep 0.2\nexec /usr/bin/pgrep \"$@\"\n";

#[test]
fn an_hdiutil_killed_at_its_limit_never_counts_as_ended_well() {
    let dir = TempDir::new();
    let bin = dir.0.join("bin");
    fs::create_dir(&bin).unwrap();
    for (tool, script) in [("hdiutil", ENDS_WELL_HDIUTIL), ("pgrep", SLOW_PGREP)] {
        let tool = bin.join(tool);
        fs::write(&tool, script).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let work = dir.0.join("work");
    fs::create_dir(&work).unwrap();
    let (calls, pids) = (dir.0.join("calls"), dir.0.join("pids"));

    limited(
        watchdog_command(&work, &[work.join("mnt")], Duration::from_secs(1))
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("FAKE_CALLS", &calls)
            .env("FAKE_PIDS", &pids),
        Duration::from_secs(30),
        "the watchdog",
        &dir.0,
    )
    .unwrap();

    let calls = fs::read_to_string(&calls).unwrap();
    assert!(calls.contains("-force"), "no -force in:\n{calls}");
    let log = fs::read_to_string(work.join("watchdog.log")).unwrap();
    assert!(log.contains("hdiutil detach"), "{log}");
    assert!(log.contains("did not end in 1s"), "{log}");

    // Every helper too: stopping the tool must not let one slip the kill.
    let start = Instant::now();
    let pids = fs::read_to_string(&pids).unwrap();
    for pid in pids.split_whitespace() {
        let pid: libc::pid_t = pid.parse().unwrap();
        // SAFETY: kill takes two plain ints, and signal 0 sends nothing.
        while unsafe { libc::kill(pid, 0) } == 0 {
            assert!(start.elapsed() < Duration::from_secs(5), "{pid} lives on");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Run by the next test, on a PATH it picks.
#[test]
#[ignore = "a helper of missing_tools_are_named_and_nothing_is_left"]
fn mounts_on_the_path_it_is_given() {
    Images::mount(&[CARD]).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn missing_tools_are_named_and_nothing_is_left() {
    let cases: [(&[&str], &str); 3] = [
        (&[], "hdiutil and mount"),
        (&["/usr/bin/hdiutil"], "mount"),
        (&["/sbin/mount"], "hdiutil"),
    ];
    for (have, named) in cases {
        let dir = TempDir::new();
        let bin = dir.0.join("bin");
        let tmp = dir.0.join("tmp");
        fs::create_dir(&bin).unwrap();
        fs::create_dir(&tmp).unwrap();
        // `sh` stays: without it the watchdog fails first, and says so.
        for tool in have.iter().chain(&["/bin/sh"]) {
            let tool = Path::new(tool);
            symlink(tool, bin.join(tool.file_name().unwrap())).unwrap();
        }
        let err = limited(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "--nocapture",
                    "mounts_on_the_path_it_is_given",
                ])
                .env("PATH", &bin)
                .env("TMPDIR", &tmp),
            Duration::from_secs(120),
            "the helper",
            &dir.0,
        )
        .unwrap_err();
        assert!(err.contains(&format!("need {named} on PATH")), "{err}");
        assert!(!err.contains("FAILED to detach"), "{err}");
        assert!(!err.contains("may be hung"), "{err}");
        let left: Vec<_> = fs::read_dir(&tmp)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert!(left.is_empty(), "left behind: {left:?}");
    }
}
