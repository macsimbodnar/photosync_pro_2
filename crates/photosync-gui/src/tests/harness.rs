//! How a GUI test sees a screen with no window (DEC-066). Later steps use these:
//! `screen_check` for a screen's words and pictures, `fixed_in_sight` for what may never
//! scroll, `fixed_box_in_sight` for a box of words that stands with it, `in_sight` for what
//! shows when the screen opens, `click` for a button that must answer and `clicks` to prove
//! one held, `play` for the real program, tasks and threads included, and
//! `blocked_discovery` to hold a look.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, Instant};

use iced::{Rectangle, Size, Theme};
use iced_test::core::Settings;
use iced_test::core::widget::Id;
use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::channel::mpsc::{self as emulator_channel, TryRecvError};
use iced_test::instruction::{Expectation, Interaction, Mouse, Target as ClickAt};
use iced_test::selector::{self, Candidate, Target};
use iced_test::{Instruction, Simulator};
use photosync_core::{ScanError, Volumes};

use crate::gui::{self, BODY, Discover, ForTest, Gui, LEAST, M, WINDOW, update, view};

/// A test that waits longer than this has found a fault: never hang the gate.
pub(crate) const LIMIT: Duration = Duration::from_secs(120);
/// DEC-066 (5a) names it beside the least size.
const COMMON: Size = Size::new(800.0, 600.0);
/// Two list rows and the space between. The body gets only what the fixed part left, so room
/// here proves the column did not run out. It proves nothing of a height bound on one child,
/// a fixed height on the body, or a body in a row: see `see_in_sight`.
const BODY_KEEPS: f32 = 67.0;
/// One line of words. A text a scrollable cuts shows at least this, or it is not in sight.
const ONE_LINE: f32 = 20.0;
/// Layout gives fractions of a pixel, and a bound can pass by less than that.
const ROUND: f32 = 0.5;

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

/// Hears the worker until the sync's end has reached the model, as `M::Wake` and `M::Gone`
/// would when the worker has sent it.
pub(crate) fn run_out(gui: &mut Gui) {
    let started = Instant::now();
    loop {
        let _ = update(gui, M::Gone);
        if gui.app.running().is_none() {
            return;
        }
        assert!(started.elapsed() < LIMIT, "the sync did not end");
        sleep(Duration::from_millis(1));
    }
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

/// A click on `words`, its messages fed to `update`. Their tasks are dropped. Panics if
/// it sends nothing, as a test that goes on would prove nothing: `clicks` shows one held.
pub(crate) fn click(gui: &mut Gui, words: &str) {
    let messages = clicks(gui, words);
    assert!(!messages.is_empty(), "a click on {words:?} sends nothing");
    for message in messages {
        let _ = update(gui, message);
    }
}

/// `sentence` is on the screen, by its whole words.
pub(crate) fn says(gui: &Gui, sentence: &str) -> bool {
    screen(gui, WINDOW).find(sentence).is_ok()
}

/// Each sentence shows at `size` when the screen opens, its scrollables at their tops. It
/// passes a text of the body, so it says nothing of what is left when the body has moved:
/// a warning or a button takes `fixed_in_sight`.
pub(crate) fn in_sight(gui: &Gui, size: Size, sentences: &[&str]) {
    if let Err(e) = see_in_sight(&mut screen(gui, size), size, sentences) {
        panic!("{e}");
    }
}

/// The body has room for rows at `size`. A screen with none has nothing to prove its fixed
/// part was not cut, so it fails too.
fn body_has_room(ui: &mut Simulator<'_, M>, size: Size) -> Result<(), String> {
    let found = ui
        .find(selector::id(BODY))
        .map_err(|e| format!("no body at {size:?}: {e}"))?;
    let Target::Scrollable { bounds, .. } = found else {
        return Err(format!("the body is no scrollable: {found:?}"));
    };
    if bounds.height < BODY_KEEPS {
        return Err(format!(
            "the body keeps {} at {size:?}: the fixed part is too tall, and may be cut",
            bounds.height
        ));
    }
    Ok(())
}

/// Each sentence is in sight at `size` and in no scrollable, so no scrolling can take it
/// away (DEC-071 (1), DEC-078): a warning, a button, anything of the fixed part.
pub(crate) fn fixed_in_sight(gui: &Gui, size: Size, sentences: &[&str]) {
    if let Err(e) = see_fixed_in_sight(&mut screen(gui, size), size, sentences) {
        panic!("{e}");
    }
}

/// Each sentence shows in the scrollable of `id`, at its top, and that box is in the fixed
/// part (DEC-071 (2), (3)). So the box's words can scroll, inside it, and no further.
pub(crate) fn fixed_box_in_sight(gui: &Gui, size: Size, id: &'static str, sentences: &[&str]) {
    if let Err(e) = see_fixed_box(&mut screen(gui, size), size, id, sentences) {
        panic!("{e}");
    }
}

/// One place the walk found what it was looking for.
struct Place {
    bounds: Rectangle,
    seen: Option<Rectangle>,
    /// The id and view of each scrollable the walk had met by then.
    views: Vec<(Option<Id>, Option<Rectangle>)>,
}

impl Place {
    /// The walk tells no parents, and each scrollable it meets comes before what is in
    /// it. A thing in a scrollable shows inside its view, or not at all. A thing outside
    /// shows its whole bounds, as the walk's first viewport is infinite, and so runs past
    /// the view of one over it or beside it, however far that one's content goes. So only
    /// a thing drawn on top of a scrollable can be taken for one in it.
    fn held_by(&self, view: Option<Rectangle>) -> bool {
        match (view, self.seen) {
            (Some(view), Some(seen)) => {
                seen.x >= view.x - ROUND
                    && seen.y >= view.y - ROUND
                    && seen.x + seen.width <= view.x + view.width + ROUND
                    && seen.y + seen.height <= view.y + view.height + ROUND
            }
            _ => false,
        }
    }

    fn in_any_scrollable(&self) -> bool {
        self.views.iter().any(|(_, view)| self.held_by(*view))
    }

    /// In the scrollable of `id`, in sight or not. A thing with no height has no room
    /// anywhere, and is in none.
    fn in_scrollable(&self, id: &Id) -> bool {
        self.views.iter().any(|(found, view)| {
            found.as_ref() == Some(id)
                && (self.held_by(*view) || (self.seen.is_none() && self.bounds.height > 0.0))
        })
    }

    /// A scrollable can cut it, and must leave a line.
    fn shows_a_line(&self) -> bool {
        self.seen
            .is_some_and(|seen| seen.height >= ONE_LINE.min(self.bounds.height - ROUND))
    }
}

/// Every place `wanted` holds, in a walk of the whole tree.
fn places_of(
    ui: &mut Simulator<'_, M>,
    wanted: impl Fn(&Candidate<'_>) -> bool + Sync,
) -> Vec<Place> {
    let mut places = Vec::new();
    let mut views = Vec::new();
    // Selects nothing, so the walk goes over the whole tree.
    let _ = ui.find(|candidate: Candidate<'_>| {
        if wanted(&candidate) {
            places.push(Place {
                bounds: candidate.bounds(),
                seen: candidate.visible_bounds(),
                views: views.clone(),
            });
        }
        if let Candidate::Scrollable {
            id, visible_bounds, ..
        } = candidate
        {
            views.push((id.cloned(), visible_bounds));
        }
        None::<()>
    });
    places
}

/// Where the words stand, as `find` sees words: a text's, or a field's.
fn places_of_words(ui: &mut Simulator<'_, M>, sentence: &str) -> Vec<Place> {
    places_of(ui, |candidate| match candidate {
        Candidate::Text { content, .. } => *content == sentence,
        Candidate::TextInput { state, .. } => state.text() == sentence,
        _ => false,
    })
}

/// `fixed_in_sight`, on any element. A card can be named as a button is, so one place in
/// the fixed part is enough.
pub(crate) fn see_fixed_in_sight(
    ui: &mut Simulator<'_, M>,
    size: Size,
    sentences: &[&str],
) -> Result<(), String> {
    body_has_room(ui, size)?;
    for sentence in sentences {
        let places = places_of_words(ui, sentence);
        if places.is_empty() {
            return Err(format!("{sentence:?} is not on the screen at {size:?}"));
        }
        if places
            .iter()
            .any(|place| place.shows_a_line() && !place.in_any_scrollable())
        {
            continue;
        }
        if places.iter().any(|place| place.seen.is_some()) {
            return Err(format!(
                "{sentence:?} is in a scrollable at {size:?}: it can scroll away"
            ));
        }
        return Err(format!("{sentence:?} is out of sight at {size:?}"));
    }
    Ok(())
}

/// `fixed_box_in_sight`, on any element.
pub(crate) fn see_fixed_box(
    ui: &mut Simulator<'_, M>,
    size: Size,
    id: &'static str,
    sentences: &[&str],
) -> Result<(), String> {
    body_has_room(ui, size)?;
    let wanted = Id::new(id);
    let boxes = places_of(ui, |candidate| {
        matches!(candidate, Candidate::Scrollable { .. }) && candidate.id() == Some(&wanted)
    });
    let Some(the_box) = boxes.first() else {
        return Err(format!("no box {id:?} at {size:?}"));
    };
    if the_box.in_any_scrollable() || the_box.seen.is_none() {
        return Err(format!(
            "the box {id:?} is in a scrollable or out of sight at {size:?}"
        ));
    }
    for sentence in sentences {
        let places = places_of_words(ui, sentence);
        let mut inside = places.iter().filter(|place| place.in_scrollable(&wanted));
        let Some(first) = inside.next() else {
            return Err(format!("{sentence:?} is not in the box {id:?} at {size:?}"));
        };
        if !first.shows_a_line() && !inside.any(Place::shows_a_line) {
            return Err(format!(
                "{sentence:?} is out of sight in the box {id:?} at {size:?}"
            ));
        }
    }
    Ok(())
}

/// `in_sight`, on any element. A scrollable cuts a text's visible bounds, and must leave a
/// line. A text with no room left has no height, so no visible bounds. A height bound cuts
/// too, and this cannot see it, nor can `fixed_in_sight` or `fixed_box_in_sight`: it shrinks
/// the text's own bounds, so what is seen still equals them, and a text past the window's edge
/// shows whole, as the walk's first viewport is infinite. `tests/gui_split.rs` holds that no
/// screen has one (DEC-080), and `a_height_bound_passes_every_check_here` that this is so.
pub(crate) fn see_in_sight(
    ui: &mut Simulator<'_, M>,
    size: Size,
    sentences: &[&str],
) -> Result<(), String> {
    body_has_room(ui, size)?;
    for sentence in sentences {
        let found = ui
            .find(*sentence)
            .map_err(|e| format!("{sentence:?} at {size:?}: {e}"))?;
        let out = || format!("{sentence:?} is out of sight at {size:?}: {found:?}");
        let seen = found.visible_bounds().ok_or_else(out)?;
        let cut = seen.height < found.bounds().height - ROUND;
        if cut && seen.height < ONE_LINE {
            return Err(out());
        }
    }
    Ok(())
}

/// Each sentence is a text inside the body's scrollable, in sight or not. The walk meets
/// the body and then what is in it, and after it, what is under it, whose visible bounds
/// the body does not hold. With no room for the body, `body_has_room` has failed already.
fn see_in_body(ui: &mut Simulator<'_, M>, size: Size, sentences: &[&str]) -> Result<(), String> {
    body_has_room(ui, size)?;
    let body = Id::new(BODY);
    for sentence in sentences {
        let places = places_of_words(ui, sentence);
        if !places.iter().any(|place| place.in_scrollable(&body)) {
            return Err(format!("{sentence:?} is not in the body at {size:?}"));
        }
    }
    Ok(())
}

/// What `screen_check` asks at one size.
pub(crate) fn screen_words(
    ui: &mut Simulator<'_, M>,
    size: Size,
    sentences: &[&str],
    body: &[&str],
) -> Result<(), String> {
    see_in_sight(ui, size, sentences)?;
    see_in_body(ui, size, body)
}

/// A SCREEN CHECK (DEC-066 (5), DEC-076). At 800x600 and the least size, each sentence is
/// found by its whole words and in sight, and the body has room. Then the pictures for the
/// agent to read: `target/gui_screens/<step>_<screen>.png` at the window's size, the size
/// that first opens, and `<step>_<screen>_least.png` at the least size.
pub(crate) fn screen_check(gui: &Gui, step: &str, name: &str, sentences: &[&str]) {
    screen_check_scrolling(gui, step, name, sentences, &[]);
}

/// `screen_check`, with texts that are in the body and may be scrolled out of sight.
pub(crate) fn screen_check_scrolling(
    gui: &Gui,
    step: &str,
    name: &str,
    sentences: &[&str],
    body: &[&str],
) {
    for size in [COMMON, LEAST] {
        if let Err(e) = screen_words(&mut screen(gui, size), size, sentences, body) {
            panic!("{step}_{name}: {e}");
        }
    }
    picture_at(gui, WINDOW, step, name);
    picture_at(gui, LEAST, step, &format!("{name}_least"));
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

/// The words a line clicks on. Only a click on words can fail for being out of sight.
fn clicked_words(line: &str) -> String {
    match Instruction::parse(line).unwrap() {
        Instruction::Interact(Interaction::Mouse(Mouse::Click {
            target: Some(ClickAt::Text(words)),
            ..
        })) => words,
        _ => panic!("Step::Fails takes a click on words: {line}"),
    }
}

/// One thing `play_steps` does.
pub(crate) enum Step {
    /// An Emulator line that must pass.
    Line(&'static str),
    /// A click on words that are on the screen and out of sight. A click fails on words
    /// that are gone too, so `expect` runs first, or a script would pass on those.
    Fails(&'static str),
    /// The body to its end.
    ScrollEnd,
    /// Reaches the state the Emulator holds, as to set what a later click will use.
    Set(Arc<dyn Fn(&mut Gui) + Send + Sync>),
}

/// Runs the real program with no window, tasks and threads included, and each of `lines`
/// in turn, as `click "Next"` or `expect "Report"`: `expect` passes for words out of
/// sight, `click` does not (DEC-072 (5)). A click that sends nothing passes here, as the
/// Emulator shows no messages: a script ends by asserting on the state it gives. Each line
/// waits for all tasks.
pub(crate) fn play(discover: Discover, lines: &[&'static str]) -> Gui {
    let steps: Vec<Step> = lines.iter().map(|line| Step::Line(line)).collect();
    play_steps(discover, steps)
}

/// `play`, with clicks that must fail and scrolling. `emu.update` runs its task unwaited
/// when nothing is pending, so a scroll goes on when its one action has been performed.
pub(crate) fn play_steps(discover: Discover, steps: Vec<Step>) -> Gui {
    let program = gui::program(discover, None);
    let (events, mut heard) = emulator_channel::channel(1);
    let mut emulator = Emulator::new(events, &program, Mode::Zen, WINDOW);
    let mut steps = steps.into_iter();
    let started = Instant::now();
    let (mut go_on, mut must_fail, mut scrolling) = (false, false, false);
    // The click of a `Step::Fails` whose `expect` is running.
    let mut click_next = None;
    loop {
        if go_on {
            go_on = false;
            must_fail = false;
            if let Some(line) = click_next.take() {
                must_fail = true;
                emulator.run(&program, Instruction::parse(line).unwrap());
                continue;
            }
            match steps.next() {
                None => return emulator.into_state().0,
                Some(Step::Line(line)) => {
                    emulator.run(&program, Instruction::parse(line).unwrap());
                }
                Some(Step::Fails(line)) => {
                    let words = clicked_words(line);
                    click_next = Some(line);
                    emulator.run(&program, Instruction::Expect(Expectation::Text(words)));
                }
                Some(Step::ScrollEnd) => {
                    scrolling = true;
                    emulator.update(&program, M::ScrollEndForTest);
                }
                Some(Step::Set(reach)) => {
                    // Its task is none, so nothing will say it is done.
                    emulator.update(&program, M::ForTest(ForTest(reach)));
                    go_on = true;
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
