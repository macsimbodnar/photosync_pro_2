//! The real window against the headless picture (DEC-065 (4), DEC-072 (1), (2)). A window
//! shows on the user's screen, so this is ignored, in its own test binary: run by hand,
//!     cargo test -p photosync-gui --test smoke_window -- --ignored
//! or by a step that means to.

#[path = "../../photosync/tests/common/mod.rs"]
mod common;
#[path = "support/one_picture.rs"]
mod one_picture;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use common::TempDir;
use iced::Theme;
use iced_test::Simulator;
use iced_test::core::Settings;
use one_picture::same_picture;
use photosync_core::discover_volumes;
use photosync_gui::gui::{Gui, M, WINDOW, update, view};

const LIMIT: Duration = Duration::from_secs(60);

/// The binary, on `empty` so a mounted card changes nothing. Gives the picture it wrote.
fn run_the_binary(empty: &Path, picture: PathBuf) -> PathBuf {
    let mut child = Command::new(env!("CARGO_BIN_EXE_photosync-gui"))
        .arg("--search-root")
        .arg(empty)
        .arg("--smoke-png")
        .arg(&picture)
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > LIMIT {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("the window did not end in {LIMIT:?}");
        }
        sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "{status}");
    picture
}

/// A PNG's width and height, from its header.
fn size_of(png: &[u8]) -> (u32, u32) {
    let number = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    (number(16), number(20))
}

#[test]
#[ignore = "opens a real window on the user's screen: run by hand (DEC-072 (1))"]
fn the_real_window_draws_what_the_headless_picture_draws() {
    let empty = TempDir::new();
    let work = TempDir::new();
    let picture = match std::env::var_os("PHOTOSYNC_GUI_SMOKE_PNG") {
        Some(file) => PathBuf::from(file as OsString),
        None => run_the_binary(&empty.0, work.0.join("smoke.png")),
    };
    let bytes = fs::read(&picture).unwrap();

    let wanted = (WINDOW.width as u32 * 2, WINDOW.height as u32 * 2);
    assert_eq!(
        size_of(&bytes),
        wanted,
        "not compared: the headless picture is at scale 2, so this screen's scale is not 2, or the window was not its usual size"
    );

    let (mut gui, _) = Gui::boot(
        {
            let root = empty.0.clone();
            std::sync::Arc::new(move || discover_volumes(std::slice::from_ref(&root)))
        },
        None,
    );
    let started = Instant::now();
    while gui.looking() {
        let _ = update(&mut gui, M::Looked);
        assert!(started.elapsed() < LIMIT, "no answer came");
        sleep(Duration::from_millis(1));
    }

    // Its own folder, as `same_picture` wants one picture there.
    let compare = TempDir::new();
    let compared = compare.0.join("real-tiny-skia.png");
    fs::write(&compared, &bytes).unwrap();
    let snapshot = Simulator::with_size(Settings::default(), WINDOW, view(&gui))
        .snapshot(&Theme::Dark)
        .unwrap();
    if let Err(e) = same_picture(&snapshot, &compare.0, "real") {
        panic!(
            "the window against the headless picture: {e}; a mouse resting on a button changes the pixels, so move it away and run again"
        );
    }
    assert_eq!(
        fs::read(&compared).unwrap(),
        bytes,
        "it compared, and wrote nothing"
    );
}
