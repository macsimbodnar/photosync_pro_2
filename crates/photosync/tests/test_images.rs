//! The images the tests mount for themselves must never stay attached, and
//! a hang must end: the gate runs on the user's own machine, and macOS's
//! disk-image service has hung on a test image before. These check the
//! helper in `tests/common/images.rs` itself. Not on a test image's temp dir:
//! `scripts/test_exfat.sh` skips them.

#![cfg(target_os = "macos")]

mod common;

use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use common::TempDir;
use common::images::{CARD, Images, attached_under, limited};

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
    fs::write(tell, images.work().as_os_str().as_encoded_bytes()).unwrap();
    std::thread::sleep(Duration::from_secs(120));
    panic!("nobody killed this test");
}

#[test]
fn a_killed_test_binary_leaves_no_image_attached() {
    let dir = TempDir::new();
    let tell = dir.0.join("work");
    let mut holder = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "holds_the_images_until_killed"])
        .env("PHOTOSYNC_TEST_TELL", &tell)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !tell.exists() {
        assert!(start.elapsed() < Duration::from_secs(120), "never mounted");
        assert!(
            holder.try_wait().unwrap().is_none(),
            "the holder ended early"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let work = PathBuf::from(fs::read_to_string(&tell).unwrap());
    assert!(work.exists());

    holder.kill().unwrap(); // SIGKILL: nothing in it runs
    holder.wait().unwrap();

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
