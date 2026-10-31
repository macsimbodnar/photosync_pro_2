//! How a GUI test sees a screen with no window (DEC-066). Later steps use these:
//! `screen_check` for a screen's words and picture, `in_sight` for what may never scroll,
//! `clicks` and `click` for buttons, `play` for the real program, tasks and threads
//! included, and `blocked_discovery` to hold a look.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, Instant};

use iced::{Size, Theme};
use iced_test::core::Settings;
use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::channel::mpsc::{self as emulator_channel, TryRecvError};
use iced_test::selector::Candidate;
use iced_test::{Instruction, Simulator};
use photosync_core::{ScanError, Volumes};

use crate::gui::{self, Discover, Gui, LEAST, M, WINDOW, update, view};

/// A test that waits longer than this has found a fault: never hang the gate.
const LIMIT: Duration = Duration::from_secs(120);
/// DEC-066 (5a) names it beside the least size.
const COMMON: Size = Size::new(800.0, 600.0);

/// `Settings::default()` is what makes the Simulator draw in Fira Sans.
pub(crate) fn screen(gui: &Gui, size: Size) -> Simulator<'_, M> {
    Simulator::with_size(Settings::default(), size, view(gui))
}

/// Volumes with these literal folders, and each note as a discovery failure.
pub(crate) fn volumes(sources: &[&str], destinations: &[&str], notes: &[&str]) -> Volumes {
    Volumes {
        sources: sources.iter().map(PathBuf::from).collect(),
        destinations: destinations.iter().map(PathBuf::from).collect(),
        failures: notes
            .iter()
            .map(|note| ScanError {
                path: PathBuf::from(note),
                source: std::io::Error::other("test"),
            })
            .collect(),
    }
}

/// A discovery that always gives these folders.
pub(crate) fn literal(sources: &[&str], destinations: &[&str]) -> Discover {
    let sources: Vec<String> = sources.iter().map(|s| s.to_string()).collect();
    let destinations: Vec<String> = destinations.iter().map(|s| s.to_string()).collect();
    Arc::new(move || {
        let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
        let destinations: Vec<&str> = destinations.iter().map(String::as_str).collect();
        volumes(&sources, &destinations, &[])
    })
}

/// A discovery in which each look waits. The receiver gets a sender per look, in the
/// order they started; a look ends when its sender is given its `Volumes`. Only by plain
/// `update` and the Simulator: `Mode::Zen` would wait for the look it holds.
pub(crate) fn blocked_discovery() -> (Discover, Receiver<Sender<Volumes>>) {
    let (looks, started) = mpsc::channel();
    let discover: Discover = Arc::new(move || {
        let (answer, answered) = mpsc::channel();
        looks.send(answer).unwrap();
        answered.recv().unwrap_or_default()
    });
    (discover, started)
}

/// The next look a `blocked_discovery` started.
pub(crate) fn next_look(started: &Receiver<Sender<Volumes>>) -> Sender<Volumes> {
    started.recv_timeout(LIMIT).expect("no look started")
}

/// Booted, with the first look answered. Its task is dropped: tests that use plain
/// `update` hear a look by polling, and the Emulator in `play` runs tasks.
pub(crate) fn booted(discover: Discover) -> Gui {
    let (mut gui, _) = Gui::boot(discover, None);
    settle(&mut gui);
    gui
}

/// Takes answers until the newest look has answered.
pub(crate) fn settle(gui: &mut Gui) {
    wait_until(gui, |gui| !gui.looking());
}

/// Takes answers until `done` holds, as `M::Looked` would when a look's thread ends.
pub(crate) fn wait_until(gui: &mut Gui, done: impl Fn(&Gui) -> bool) {
    let started = Instant::now();
    loop {
        let _ = update(gui, M::Looked);
        if done(gui) {
            return;
        }
        assert!(started.elapsed() < LIMIT, "no answer came");
        sleep(Duration::from_millis(1));
    }
}

/// What a click on `words` sends, at the window's size. Panics if they are not in sight.
pub(crate) fn clicks(gui: &Gui, words: &str) -> Vec<M> {
    let mut ui = screen(gui, WINDOW);
    if let Err(e) = ui.click(words) {
        panic!("no click on {words:?}: {e}");
    }
    ui.into_messages().collect()
}

/// A click on `words`, its messages fed to `update`. Their tasks are dropped.
pub(crate) fn click(gui: &mut Gui, words: &str) {
    for message in clicks(gui, words) {
        let _ = update(gui, message);
    }
}

/// `sentence` is on the screen, by its whole words.
pub(crate) fn says(gui: &Gui, sentence: &str) -> bool {
    screen(gui, WINDOW).find(sentence).is_ok()
}

/// Each sentence is in sight at `size`, with no scrolling: this is the check for a warning,
/// a button, or anything else of the fixed part (DEC-071 (1)).
pub(crate) fn in_sight(gui: &Gui, size: Size, sentences: &[&str]) {
    let mut ui = screen(gui, size);
    for sentence in sentences {
        let found = ui
            .find(*sentence)
            .unwrap_or_else(|e| panic!("{sentence:?} at {size:?}: {e}"));
        assert!(
            found.visible_bounds().is_some(),
            "{sentence:?} is out of sight at {size:?}"
        );
    }
}

/// A SCREEN CHECK (DEC-066 (5)). At 800x600 and the least size, each sentence is found by
/// its whole words, in sight, or in a `scrollable` and no wider than the window: use
/// `in_sight` too for a warning, which may not scroll. Then the picture, drawn at the
/// window's size, for the agent to read: `target/gui_screens/<step>_<screen>.png`.
pub(crate) fn screen_check(gui: &Gui, step: &str, name: &str, sentences: &[&str]) {
    for size in [COMMON, LEAST] {
        let mut ui = screen(gui, size);
        for sentence in sentences {
            let found = ui
                .find(*sentence)
                .unwrap_or_else(|e| panic!("{step}_{name}: {sentence:?} at {size:?}: {e}"));
            assert!(
                found.visible_bounds().is_some() || found.bounds().width <= size.width,
                "{step}_{name}: {sentence:?} is wider than {size:?}"
            );
        }
    }
    picture_at(gui, WINDOW, step, name);
}

/// The picture alone, at a size of the test's choosing. Returns where it is.
pub(crate) fn picture_at(gui: &Gui, size: Size, step: &str, name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/gui_screens");
    fs::create_dir_all(&dir).unwrap();
    let wanted = dir.join(format!("{step}_{name}.png"));
    // `matches_image` writes only if nothing is there, and compares otherwise, and
    // it adds the renderer's name to the file's.
    let written = dir.join(format!("{step}_{name}-tiny-skia.png"));
    for old in [&wanted, &written] {
        match fs::remove_file(old) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("{}: {e}", old.display()),
        }
    }
    let snapshot = screen(gui, size).snapshot(&Theme::Dark).unwrap();
    assert!(snapshot.matches_image(&wanted).unwrap());
    fs::rename(&written, &wanted).unwrap();
    wanted
}

/// Every text on the screen, in or out of sight.
pub(crate) fn all_words(gui: &Gui, size: Size) -> Vec<String> {
    let mut seen = Vec::new();
    // Selects nothing, so the walk goes over the whole tree.
    let _ = screen(gui, size).find(|candidate: Candidate<'_>| {
        if let Candidate::Text { content, .. } = candidate {
            seen.push(content.to_string());
        }
        None::<()>
    });
    seen
}

/// One thing `play_steps` does.
pub(crate) enum Step {
    /// An Emulator line that must pass.
    Line(&'static str),
    /// One that must fail: a click on words out of sight.
    Fails(&'static str),
    /// The body to its end.
    ScrollEnd,
}

/// Runs the real program with no window, tasks and threads included, and each of `lines`
/// in turn, as `click "Next"` or `expect "Report"`: `expect` passes for words out of
/// sight, `click` does not (DEC-072 (5)). Each line waits for all tasks. Gives the
/// state at the end.
pub(crate) fn play(discover: Discover, lines: &[&'static str]) -> Gui {
    let steps: Vec<Step> = lines.iter().map(|line| Step::Line(line)).collect();
    play_steps(discover, steps)
}

/// `play`, with lines that must fail and scrolling. `emu.update` runs its task unwaited
/// when nothing is pending, so a scroll goes on when its one action has been performed.
pub(crate) fn play_steps(discover: Discover, steps: Vec<Step>) -> Gui {
    let program = gui::program(discover, None);
    let (events, mut heard) = emulator_channel::channel(1);
    let mut emulator = Emulator::new(events, &program, Mode::Zen, WINDOW);
    let mut steps = steps.into_iter();
    let started = Instant::now();
    let (mut go_on, mut must_fail, mut scrolling) = (false, false, false);
    loop {
        if go_on {
            go_on = false;
            must_fail = false;
            match steps.next() {
                None => return emulator.into_state().0,
                Some(Step::Line(line)) => {
                    emulator.run(&program, Instruction::parse(line).unwrap());
                }
                Some(Step::Fails(line)) => {
                    must_fail = true;
                    emulator.run(&program, Instruction::parse(line).unwrap());
                }
                Some(Step::ScrollEnd) => {
                    scrolling = true;
                    emulator.update(&program, M::ScrollEndForTest);
                }
            }
        }
        match heard.try_recv() {
            Ok(Event::Action(action)) => {
                emulator.perform(&program, action);
                if scrolling {
                    scrolling = false;
                    go_on = true;
                }
            }
            Ok(Event::Failed(line)) => {
                assert!(must_fail, "line failed: {line}");
                go_on = true;
            }
            Ok(Event::Ready) => {
                assert!(!must_fail, "a line that should have failed passed");
                go_on = true;
            }
            Err(TryRecvError::Closed) => panic!("the runtime stopped"),
            Err(TryRecvError::Empty) => {
                assert!(started.elapsed() < LIMIT, "nothing happened for too long");
                sleep(Duration::from_millis(1));
            }
        }
    }
}
