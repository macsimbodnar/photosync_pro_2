//! Real syncs through the GUI's clicks, on disk images, as DEC-066 (3) has it. Strict INV-10
//! lets a sync through only across devices, so only images give a real report. Each test has
//! folders of its own, as they share the images and run side by side.
//!
//! A test that needs the session to compare with has the real worker, by `keeping_the_session`.
//! (b) and (g) have no such seam, so `Gui::run`'s own road is run too.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::slice;
use std::sync::mpsc::Receiver;
use std::thread::ThreadId;

use photosync_core::{Cancel, RunEnd, SourceScan, Verdict, Watch, discover_volumes};

use super::*;
use crate::common::images::{CARD, TWIN_A, TWIN_B, assert_may_delete};
use crate::common::{
    LoseOnCopy, OnDrive, at, date_command, disk_of_the_binary, image_path, manifest, test_drive,
    twin_cards, write_file,
};

/// What `tests::ignored_image_tests_pass_on_real_mounts` wants run.
#[cfg(target_os = "macos")]
pub(super) const MUST_RUN: [&str; 14] = [
    "tests::on_images::a_one_card_found_for_real_goes_under_its_name_and_the_same_sync_copies_nothing",
    "tests::on_images::b_two_cards_at_once_each_on_its_thread_and_the_counts_add_up",
    "tests::on_images::c_a_session_into_one_folder_says_all_of_it_and_each_screen_opens_at_its_top",
    "tests::on_images::c_a_real_result_too_tall_for_its_box_opens_the_next_report_at_its_top",
    "tests::on_images::d_a_twin_that_cannot_be_read_is_named_never_plain_complete",
    "tests::on_images::e_a_name_collision_fails_red_keeps_the_old_bytes_and_stays_in_sight",
    "tests::on_images::f_a_library_on_a_card_is_refused_and_nothing_is_written",
    "tests::on_images::g_a_cancel_that_cannot_miss_leaves_no_doubt_on_the_drive",
    "tests::on_images::h_a_complete_card_after_a_failed_one_hides_nothing",
    "tests::on_images::i_the_second_click_of_a_double_on_close_leaves_the_report_a_fast_sync_raised",
    "tests::on_images::j_a_card_read_only_in_part_is_not_green_and_says_do_not_format",
    "tests::on_images::k_a_real_read_failure_with_no_twin_fails_in_the_words_of_text",
    "tests::on_images::l_a_refused_sync_then_the_same_card_complete_clears_as_the_clis",
    "tests::on_images::l_a_swapped_card_at_that_path_clears_nothing",
];

const TAKEN: u64 = 1_700_000_000;
const THE_OLD_BYTES: &[u8] = b"the older photo of this name, which must stay";
const MIB: usize = 1024 * 1024;
const QUIT: &str = "Quit";

/// Removed on drop, failing or not: the images are shared by the tests. `OnDrive` names by its
/// process, and `<drive>/Photos` and a card's `DCIM` have names of their own.
struct Gone(PathBuf);

impl Gone {
    fn new(path: PathBuf) -> Gone {
        assert_may_delete(&path);
        let _ = fs::remove_dir_all(&path); // left over from a killed run
        fs::create_dir_all(&path).unwrap();
        Gone(path)
    }
}

impl Drop for Gone {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn shown(path: &Path) -> String {
    path.display().to_string()
}

fn click_on(words: &str) -> Step {
    Step::ClickOn(Arc::new(Mutex::new(words.to_string())))
}

/// A line of `Step::Fails` on words known only here. A test's life is the process's.
fn fails_on(words: &str) -> Step {
    Step::Fails(Box::leak(format!("click {words:?}").into_boxed_str()))
}

fn set(reach: impl Fn(&mut Gui) + Send + Sync + 'static) -> Step {
    Step::Set(Arc::new(reach))
}

/// Next card, and the same card, past the check, to the next report.
fn the_same_card_again() -> Vec<Step> {
    vec![
        Step::Line(r#"click "Next card""#),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
    ]
}

/// A card a literal list names, a drive that is the only one, its default library, and a name.
fn to_the_report_named(name: &'static str) -> Vec<Step> {
    let mut steps = vec![
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Next""#),
    ];
    if !name.is_empty() {
        steps.push(Step::Line(Box::leak(
            format!("type {name:?}").into_boxed_str(),
        )));
    }
    steps.extend([
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
    ]);
    steps
}

/// A drive of this test's own on the drive image, with its `Photos`.
fn a_drive(name: &str) -> (OnDrive, PathBuf) {
    let ours = OnDrive::new(&test_drive(), name);
    let library = ours.0.join("Photos");
    fs::create_dir(&library).unwrap();
    (ours, library)
}

/// A card of this test's own on an image.
fn a_card(image: &str, name: &str) -> OnDrive {
    let on = image_path(image);
    let card = OnDrive::new(&on, name);
    fs::create_dir(card.0.join("DCIM")).unwrap();
    card
}

fn photo_on(card: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    write_file(card, &format!("DCIM/100_FUJI/{name}"), bytes, at(TAKEN))
}

/// Not one byte like another's, so a file taken for another fails the compare.
fn big_photo(seed: u8) -> Vec<u8> {
    (0..2 * MIB + 7)
        .map(|at| (at as u8).wrapping_mul(31).wrapping_add(seed) ^ (at >> 11) as u8)
        .collect()
}

fn dated(library: &Path, name: &str) -> PathBuf {
    library.join(format!("{} - {name}", date_command(TAKEN)))
}

/// A different photo of `name` where this sync would put it: a name collision.
fn collision_in(library: &Path, folder: &str, name: &str) -> PathBuf {
    let rel = format!("{} - {folder}/RAW/{name}", date_command(TAKEN));
    write_file(library, &rel, THE_OLD_BYTES, at(TAKEN - 86400))
}

/// The files there are, and their bytes. A temp shows here too, by its dot.
fn files_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    manifest(root)
        .into_iter()
        .filter_map(|(path, (_, bytes))| bytes.map(|bytes| (path, bytes)))
        .collect()
}

fn no_temp_under(root: &Path) {
    for path in manifest(root).keys() {
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(!name.starts_with(".photosync-tmp"), "{}", path.display());
    }
}

type Kept = Arc<Mutex<Option<flow::Shared>>>;

/// The real worker, as `Gui::run` starts it, keeping the job's session to compare with: the
/// model never gives it out.
fn keeping_the_session(kept: &Kept) -> Step {
    let kept = kept.clone();
    set(move |gui| {
        let kept = kept.clone();
        gui.test_worker = Some(Arc::new(move |job: Job, send: WorkerSend| {
            *kept.lock().unwrap() = Some(job.session.clone());
            worker::spawn(job, move |msg| send.send(msg));
        }));
    })
}

fn the_session(kept: &Kept) -> flow::Shared {
    kept.lock().unwrap().clone().expect("no sync ran")
}

/// What the screen shows of a report, in its order: nothing lost, added or moved.
fn the_screen_says(lines: &[String], result: &str) -> Vec<String> {
    let mut wanted = vec![
        TITLE.to_string(),
        REPORT_HEADING.to_string(),
        result.to_string(),
    ];
    wanted.extend_from_slice(lines);
    wanted.extend(REPORT_BUTTONS.iter().map(|b| b.to_string()));
    wanted
}

/// The report the model has, as it is on the screen.
fn the_report(gui: &Gui) -> (Vec<String>, String, bool) {
    assert_eq!(gui.app.screen(), Screen::Report);
    let Some(Outcome::Report {
        lines,
        result,
        complete,
    }) = gui.app.outcome()
    else {
        panic!("no report: {:?}", gui.app.outcome());
    };
    assert_eq!(all_words(gui, WINDOW), the_screen_says(lines, result));
    (lines.clone(), result.clone(), *complete)
}

/// Byte for byte what `text` says of this session, and on the screen.
fn the_report_is_the_sessions(gui: &Gui, kept: &Kept) -> (Vec<String>, String, bool) {
    let (lines, result, complete) = the_report(gui);
    let shared = the_session(kept);
    let session = flow::lock(&shared);
    let report = text::session_report(&session).expect("a report");
    let told: Vec<String> = report.lines().map(String::from).collect();
    assert_eq!(lines, told);
    assert_eq!(result, text::session_result(&session));
    (lines, result, complete)
}

/// The box is drawn as `complete` says. Only the picture holds a colour on the real screen: it
/// is the same as the pure view's for that answer, and not as the other's.
fn drawn_as(gui: &Gui, complete: bool, name: &str) {
    let Some(Outcome::Report { lines, result, .. }) = gui.app.outcome() else {
        panic!("no report");
    };
    let id = gui.app.outcome_id();
    let real = fs::read(picture_at(gui, WINDOW, "harness", &format!("{name}_real"))).unwrap();
    let pure = |given: bool| {
        let mut ui = simulator(WINDOW, report_screen(id, lines, result, given));
        let name = format!("{name}_{given}");
        fs::read(picture_of(&mut ui, "harness", &name)).unwrap()
    };
    assert!(real == pure(complete), "the box is not drawn as it says");
    assert!(real != pure(!complete), "green and red draw the same");
}

/// The picture to read, and that all the words stand where they should at both sizes.
fn a_report_screen_check(gui: &Gui, name: &str) {
    let (lines, result, _) = the_report(gui);
    let body: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|line| !line.is_empty() && *line != REPORT_HEADING)
        .collect();
    screen_check_scrolling(
        gui,
        "S057",
        name,
        &[TITLE, REPORT_HEADING, &result, NEXT_CARD, QUIT],
        &body,
    );
    for size in [WINDOW, LEAST] {
        fixed_in_sight(gui, size, &[TITLE, REPORT_HEADING, NEXT_CARD, QUIT]);
        fixed_box_in_sight(gui, size, RESULT, &[&result]);
    }
}

fn line_saying<'a>(lines: &'a [String], words: &str) -> &'a str {
    line_with(lines, words)
}

/// A next card or a quit left this warning in sight, in the session's words.
fn the_banner_says(gui: &Gui, result: &str) {
    for size in [WINDOW, LEAST] {
        fixed_in_sight(gui, size, &[NOT_SAFE]);
        fixed_box_in_sight(gui, size, BANNER, &[result]);
    }
}

/// What the process writes and says once the window has gone.
fn the_process_ends_with(gui: &Gui, code: u8, report: &str) {
    let exit = gui.exit_slot.lock().unwrap().clone().expect("no Exit");
    assert_eq!(exit.code, code);
    assert_eq!(exit.report.as_deref(), Some(report));
    let (written, _, returned) = the_end_of(&exit);
    assert_eq!(written, report);
    assert_eq!(returned, code);
}

/// (a) Discovery is the real one, over the mount points, so the rows are what the binary lists.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn a_one_card_found_for_real_goes_under_its_name_and_the_same_sync_copies_nothing() {
    let card = image_path(CARD);
    // Not the first row, and the first has a `Photos` too: a Next that takes the first row would
    // copy happily into another drive. A mutant showed the first row passes this test.
    let drive = twin_cards().1;
    let another = Gone::new(test_drive().join("Photos"));
    // The script's card has run other binaries' tests, and this sync takes all of it. `DCIM`
    // itself stays: (f) needs it there to be refused.
    for left in fs::read_dir(card.join("DCIM")).unwrap() {
        let left = left.unwrap().path();
        assert_may_delete(&left);
        fs::remove_dir_all(left).unwrap();
    }
    let _photos = Gone::new(card.join("DCIM/900_S057"));
    let library_gone = Gone::new(drive.join("Photos"));
    let library = library_gone.0.clone();
    let on_card: [(&str, &str, &[u8]); 3] = [
        ("DSCF9001.RAF", "RAW", b"the raw of one"),
        ("DSCF9001.JPG", "JPEG", b"the jpeg of one"),
        ("DSCF9002.MOV", "VIDEO", b"a short video"),
    ];
    for (name, _, bytes) in on_card {
        write_file(&card, &format!("DCIM/900_S057/{name}"), bytes, at(TAKEN));
    }
    let before = manifest(&card.join("DCIM"));

    let root = card.parent().unwrap().to_path_buf();
    let discover: Discover = Arc::new(move || discover_volumes(slice::from_ref(&root)));
    assert_eq!(discover().sources, slice::from_ref(&card), "the real look");
    let row = discover().destinations.iter().position(|d| *d == drive);
    assert!(row.is_some_and(|row| row > 0), "the real look: {row:?}");
    let kept = Kept::default();

    let (library_str, folder) = (shown(&library), dated(&library, "Tirana"));
    let wanted_on_drive: BTreeMap<PathBuf, Vec<u8>> = on_card
        .iter()
        .map(|(name, kind, bytes)| (folder.join(kind).join(name), bytes.to_vec()))
        .collect();
    let (after_one, kept_one, wanted_one) = (library.clone(), kept.clone(), wanted_on_drive.clone());
    let steps = vec![
        keeping_the_session(&kept),
        Step::Line(r#"click "Next""#),
        click_on(&shown(&drive)),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"type "Tirana""#),
        Step::Line(r#"click "Next""#),
        set(move |gui| {
            assert_eq!(gui.app.screen(), Screen::Confirm);
            assert!(says(gui, &library_str), "the check names the library");
            assert_eq!(gui.app.picked(), slice::from_ref(&card_of(gui)));
        }),
        Step::Line(r#"click "Start copying""#),
        set(move |gui| {
            let (lines, result, complete) = the_report_is_the_sessions(gui, &kept_one);
            assert!(complete, "{result}");
            assert_eq!(
                result,
                "Result: all 3 photos or videos on the drive (3 copied, 0 already there)."
            );
            assert_eq!(gui.app.exit().code, 0);
            assert_eq!(gui.app.syncs(), 1);
            let named = format!("Library {}", shown(&after_one));
            assert!(lines.contains(&named), "{lines:#?}");
            assert_eq!(files_under(&after_one), wanted_one);
            drawn_as(gui, true, "s057_a_first");
            a_report_screen_check(gui, "a_first_report");
        }),
    ];
    let mut steps = steps;
    steps.extend(the_same_card_again());
    let gui = play_steps(discover, steps);

    let (lines, result, complete) = the_report_is_the_sessions(&gui, &kept);
    assert!(complete, "{result}");
    assert_eq!(gui.app.exit().code, 0);
    assert_eq!(gui.syncs_started, 2);
    let shared = the_session(&kept);
    {
        let session = flow::lock(&shared);
        let RunEnd::Report(second) = &session.runs()[1].end else {
            panic!("the second sync has no report");
        };
        let totals = second.totals();
        assert_eq!(totals.copied.files, 0, "INV-4: {lines:#?}");
        assert_eq!(totals.skipped.files, 3, "{lines:#?}");
    }
    let says_so = "  Result: all 3 photos or videos on the drive (0 copied, 3 already there).";
    assert!(lines.iter().any(|l| l == says_so), "{lines:#?}");
    assert_eq!(
        files_under(&library),
        wanted_on_drive,
        "INV-2, INV-5, nothing more"
    );
    no_temp_under(&library);
    assert!(files_under(&another.0).is_empty(), "the drive not clicked");
    assert_eq!(manifest(&image_path(CARD).join("DCIM")), before, "INV-1");
    drawn_as(&gui, true, "s057_a");
    a_report_screen_check(&gui, "a_report");
}

/// The only card the look gave.
fn card_of(gui: &Gui) -> PathBuf {
    gui.app.sources()[0].clone()
}

/// What hears the copy threads says who started and who finished, in order. Both cards' first
/// files must be in flight together: both started before either finished. A barrier here cannot
/// be met, as the core calls its watch under one lock, so the second card's `FileStarted` waits
/// for the first's to return (DEC-087 (3)). So the first holds that lock long enough for the other
/// thread to reach it, if there is one. Card by card, the other starts only after this card ends.
#[derive(Default)]
struct InFlight {
    order: Mutex<Vec<(PathBuf, bool)>>,
}

impl InFlight {
    /// Far above what a thread takes to reach its first file, and far below `LIMIT`.
    const HOLD: Duration = Duration::from_secs(1);

    fn started(&self, card: &Path) {
        let first = {
            let mut order = self.order.lock().unwrap();
            order.push((card.to_path_buf(), false));
            order.len() == 1
        };
        if first {
            thread::sleep(Self::HOLD);
        }
    }

    fn done(&self, card: &Path) {
        self.order.lock().unwrap().push((card.to_path_buf(), true));
    }

    /// The cards that had started when the first file of all was done.
    fn started_before_any_was_done(&self) -> BTreeSet<PathBuf> {
        let order = self.order.lock().unwrap();
        let seen = order.iter().take_while(|(_, done)| !done);
        seen.map(|(card, _)| card.clone()).collect()
    }
}

/// (b) No seam but what hears the worker, so `Gui::run`'s own road goes to the end.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn b_two_cards_at_once_each_on_its_thread_and_the_counts_add_up() {
    let (a, b) = (a_card(TWIN_A, "S057-b"), a_card(TWIN_B, "S057-b"));
    let (ours, library) = a_drive("S057-b");
    let chunk = |seed: u8| vec![seed; 300 * 1024];
    let mut on_drive = BTreeMap::new();
    for n in 1..=3u8 {
        let (name_a, name_b) = (format!("DSCF100{n}.RAF"), format!("DSCF200{n}.RAF"));
        photo_on(&a.0, &name_a, &chunk(n));
        photo_on(&b.0, &name_b, &chunk(n + 100));
        let folder = library.join(date_command(TAKEN)).join("RAW");
        on_drive.insert(folder.join(name_a), chunk(n));
        on_drive.insert(folder.join(name_b), chunk(n + 100));
    }
    let (before_a, before_b) = (manifest(&a.0), manifest(&b.0));
    let (card_a, card_b) = (shown(&a.0), shown(&b.0));
    let (root_a, root_b) = (a.0.clone(), b.0.clone());

    let threads = Arc::new(Mutex::new(BTreeMap::<PathBuf, BTreeSet<String>>::new()));
    let last = Arc::new(Mutex::new(None));
    let snapshots_wrong = Arc::new(Mutex::new(Vec::new()));
    let both = Arc::new(InFlight::default());
    let (seen_threads, seen_last, seen_wrong, seen_both) =
        (threads.clone(), last.clone(), snapshots_wrong.clone(), both.clone());
    let hear_all = set(move |gui| {
        let (threads, last, wrong) = (seen_threads.clone(), seen_last.clone(), seen_wrong.clone());
        let (root_a, root_b, both) = (root_a.clone(), root_b.clone(), seen_both.clone());
        gui.before_send = Some(Arc::new(move |msg| {
            let Msg::Progress(event) = msg else { return };
            let progress = &event.progress;
            if progress.phase != Phase::Copying {
                return;
            }
            let done = progress.sources.iter().fold((0, 0), |sum, card| {
                (sum.0 + card.done.files, sum.1 + card.done.bytes)
            });
            let total = progress.sources.iter().fold((0, 0), |sum, card| {
                (sum.0 + card.total.files, sum.1 + card.total.bytes)
            });
            if done != (progress.done.files, progress.done.bytes)
                || total != (progress.total.files, progress.total.bytes)
            {
                wrong.lock().unwrap().push(format!("{progress:?}"));
            }
            if let EventKind::FileStarted(path) = &event.kind {
                let card = if path.starts_with(&root_a) {
                    &root_a
                } else {
                    assert!(path.starts_with(&root_b), "{}", path.display());
                    &root_b
                };
                let thread: ThreadId = thread::current().id();
                threads
                    .lock()
                    .unwrap()
                    .entry(card.clone())
                    .or_default()
                    .insert(format!("{thread:?}"));
                both.started(card);
            }
            if let EventKind::FileDone(path) = &event.kind {
                let card = if path.starts_with(&root_a) { &root_a } else { &root_b };
                both.done(card);
            }
            *last.lock().unwrap() = Some(progress.clone());
        }));
    });
    let mut steps = vec![hear_all, click_on(&card_a), click_on(&card_b)];
    steps.extend(to_the_report_named(""));
    let gui = play_steps(literal(&[&card_a, &card_b], &[&shown(&ours.0)]), steps);

    let (lines, result, complete) = the_report(&gui);
    assert!(complete, "{result}");
    assert_eq!(gui.app.exit().code, 0);
    assert_eq!(gui.app.picked().len(), 2);
    assert_eq!(gui.syncs_started, 1);
    assert_eq!(result, "Result: all 6 photos or videos on the drive (6 copied, 0 already there).");
    assert!(lines.iter().any(|l| l.contains(&card_a)), "{lines:#?}");
    assert!(lines.iter().any(|l| l.contains(&card_b)), "{lines:#?}");

    let threads = threads.lock().unwrap();
    let of = |card: &Path| threads.get(card).cloned().unwrap_or_default();
    let (of_a, of_b) = (of(&a.0), of(&b.0));
    assert_eq!((of_a.len(), of_b.len()), (1, 1), "{threads:?}");
    assert_ne!(of_a, of_b, "each card on its thread");
    let at_once = both.started_before_any_was_done();
    assert_eq!(at_once.len(), 2, "one card waited for the other: {at_once:?}");

    let wrong = snapshots_wrong.lock().unwrap();
    assert!(wrong.is_empty(), "a snapshot does not add up: {wrong:#?}");
    let last = last.lock().unwrap().clone().expect("no snapshot came");
    assert_eq!(last.sources.len(), 2);
    assert_eq!((last.done.files, last.total.files), (6, 6));
    assert_eq!(last.done.bytes, last.total.bytes);
    assert_eq!(last.total.bytes, 6 * 300 * 1024);
    for card in &last.sources {
        assert_eq!((card.done.files, card.total.files), (3, 3), "{card:?}");
        assert_eq!(card.done.bytes, card.total.bytes, "{card:?}");
    }
    assert_eq!(files_under(&library), on_drive);
    no_temp_under(&library);
    assert_eq!(manifest(&a.0), before_a, "INV-1");
    assert_eq!(manifest(&b.0), before_b, "INV-1");
    drawn_as(&gui, true, "s057_b");
    a_report_screen_check(&gui, "b_report");
}

/// A card's folder named with what the fixed font loses (DEC-082 (3)). `hdiutil` takes the
/// label too, but a mount point here is named by the helper: a line holds the path, and a real
/// card's path is `/Volumes/<label>`, so a folder of that name gives the same words.
const KANJI: &str = "写真カード";

/// (c) Three syncs: a real failure, then 30 duplicates and a new photo into the one folder, then
/// the card with the kanji. So the second report and the third scroll, and the rows of the cards
/// do, so nothing can pass here by having nothing to scroll.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn c_a_session_into_one_folder_says_all_of_it_and_each_screen_opens_at_its_top() {
    let (a, b) = (a_card(TWIN_A, "S057-c"), a_card(TWIN_B, "S057-c"));
    let under_b = OnDrive::new(&twin_cards().1, "S057-c-kanji");
    let c = under_b.0.join(KANJI);
    fs::create_dir_all(c.join("DCIM")).unwrap();
    let (ours, library) = a_drive("S057-c");
    let old = collision_in(&library, "Trip", "DSCF0099.RAF");
    let a_collides = photo_on(&a.0, "DSCF0099.RAF", b"not the older photo");
    let photo_bytes = |n: u32| format!("the photo numbered {n}").into_bytes();
    for n in 1..=30 {
        let name = format!("DSCF{n:04}.RAF");
        photo_on(&a.0, &name, &photo_bytes(n));
        photo_on(&b.0, &name, &photo_bytes(n));
    }
    photo_on(&b.0, "DSCF0100.RAF", b"only on the second card");
    photo_on(&c, "DSCF0001.RAF", &photo_bytes(1));
    photo_on(&c, "DSCF0200.RAF", b"only on the third card");
    let before: Vec<_> = [&a.0, &b.0, &c].iter().map(|p| manifest(p)).collect();

    let (card_a, card_b, card_c) = (shown(&a.0), shown(&b.0), shown(&c));
    // The first row is the third card's, which a body left at its end would have out of sight.
    let mut rows = vec![card_c.clone(), card_a.clone(), card_b.clone()];
    rows.extend((0..40).map(|n| format!("/Volumes/ANOTHER_CARD_{n}")));
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let discover = literal(&rows, &[&shown(&ours.0)]);
    let kept = Kept::default();
    let first_line = format!("Library {}", shown(&library));

    let (kept_1, one_folder, wanted_a) = (kept.clone(), dated(&library, "Trip"), card_a.clone());
    let (a_failed, the_old) = (a_collides.clone(), old.clone());
    let after_first = set(move |gui| {
        let (lines, result, complete) = the_report_is_the_sessions(gui, &kept_1);
        assert!(!complete, "{result}");
        assert_eq!(gui.app.exit().code, 1);
        assert!(result.contains("1 failed"), "{result}");
        let told = format!(
            "  failed      {}: name collision with {}",
            shown(&a_failed),
            shown(&the_old)
        );
        assert!(lines.contains(&told), "{told} in {lines:#?}");
        assert!(lines.iter().any(|l| l.contains(&wanted_a)), "{lines:#?}");
        assert!(one_folder.join("RAW/DSCF0001.RAF").exists());
    });

    let (kept_2, line_2, wanted_a, wanted_b) =
        (kept.clone(), first_line.clone(), card_a.clone(), card_b.clone());
    let (one_folder, a_failed) = (dated(&library, "Trip"), a_collides.clone());
    let after_second = set(move |gui| {
        let (lines, result, complete) = the_report_is_the_sessions(gui, &kept_2);
        assert!(!complete, "{result}");
        assert_eq!(gui.app.exit().code, 1);
        assert!(
            result.starts_with(&format!(
                "Result: NOT COMPLETE: 1 of 2 syncs not complete: sync 1 ({wanted_a}): 1 failed"
            )),
            "{result}"
        );
        assert_eq!(lines[1], line_2);
        // The second card's photo goes where the first card's went.
        assert_eq!(
            fs::read(one_folder.join("RAW/DSCF0100.RAF")).unwrap(),
            b"only on the second card"
        );
        let at = |line: &str| {
            let found = lines.iter().position(|l| l == line);
            found.unwrap_or_else(|| panic!("{line:?} in {lines:#?}"))
        };
        let failure = line_saying(&lines, "name collision with");
        assert!(failure.contains(&shown(&a_failed)), "{failure}");
        let duplicate = format!("  duplicate in session  100_FUJI/DSCF0001.RAF, on drive from {wanted_a}");
        let problems = "1 problem line above the duplicates: read it.";
        let shown_problems = lines.iter().find(|l| l.contains("above the duplicates"));
        assert!(
            shown_problems.is_some_and(|l| l == problems),
            "{shown_problems:?}"
        );
        assert!(at(failure) < at(&duplicate), "a duplicate buries no failure");
        assert!(at(&duplicate) < at(problems));
        assert!(lines.iter().any(|l| l.contains(&wanted_b)), "{lines:#?}");
        drawn_as(gui, false, "s057_c_second");
    });

    let mut steps = vec![keeping_the_session(&kept), click_on(&card_a)];
    steps.extend(to_the_report_named("Trip"));
    steps.push(after_first);
    steps.extend([
        Step::Line(r#"click "Next card""#),
        click_on(&card_b),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
        after_second,
        // The report at its end: its first line is there, out of sight.
        Step::ScrollEnd,
        fails_on(&first_line),
        Step::Line(r#"click "Next card""#),
        // At its top, past a report left at its end: and it picks the third card.
        click_on(&card_c),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
        click_on(&first_line),
    ]);
    let (kept_3, wanted_c) = (kept.clone(), card_c.clone());
    let kanji_line = Arc::new(Mutex::new(String::new()));
    let told_line = kanji_line.clone();
    steps.push(set(move |gui| {
        let (lines, result, _) = the_report_is_the_sessions(gui, &kept_3);
        assert_eq!(gui.app.syncs(), 3);
        assert!(result.contains("1 of 3 syncs not complete"), "{result}");
        let on_c = line_saying(&lines, &format!("Sync 3 of 3: {wanted_c}"));
        *told_line.lock().unwrap() = on_c.to_string();
        // The real line, and the same with no kanji: the same rows, or `写真` is lost.
        let latin = on_c.replace(KANJI, "ABCDEFGHIJ");
        let both = [on_c.to_string(), latin.clone()];
        let mut ui = simulator(WINDOW, report_screen(1, &both, &result, false));
        let high = |ui: &mut Simulator<'_, M>, line: &str| ui.find(line).unwrap().bounds().height;
        assert_eq!(high(&mut ui, on_c), high(&mut ui, &latin));
        drawn_as(gui, false, "s057_c");
        a_report_screen_check(gui, "c_report");
        // The picture of the end, where the card's name stands over its duplicates.
        let (id, lines_now) = (gui.app.outcome_id(), lines.clone());
        let mut ui = simulator(WINDOW, report_screen(id, &lines_now, &result, false));
        wheel(&mut ui, BODY, 10000.0);
        at_its_end(&mut ui, BODY);
        let on_screen = lines_now
            .iter()
            .any(|l| l.contains(KANJI) && ui.find(l.as_str()).is_ok_and(|t| t.visible_bounds().is_some()));
        assert!(on_screen, "no line of the card in sight at the end");
        picture_of(&mut ui, "S057", "c_report_end");
    }));
    let (kept_4, seen_report) = (kept.clone(), Arc::new(Mutex::new(String::new())));
    let told_report = seen_report.clone();
    steps.push(set(move |_| {
        let shared = the_session(&kept_4);
        let session = flow::lock(&shared);
        *told_report.lock().unwrap() = text::session_report(&session).unwrap();
    }));
    steps.push(Step::Line(r#"click "Quit""#));
    let gui = play_steps(discover, steps);

    assert!(gui.quit, "Quit answers at once");
    assert!(!gui.before_quit(), "no Before you quit from the report");
    the_process_ends_with(&gui, 1, &seen_report.lock().unwrap());
    assert!(kanji_line.lock().unwrap().contains(KANJI));
    assert_eq!(fs::read(&old).unwrap(), THE_OLD_BYTES, "INV-2");
    no_temp_under(&library);
    // Every file on the drive, and each one's bytes as its own card holds them: the 30 from the
    // first card, the two only on a later one, the old photo, and nothing more.
    let raw = dated(&library, "Trip").join("RAW");
    let on_card = |card: &Path, name: &str| fs::read(card.join("DCIM/100_FUJI").join(name)).unwrap();
    let mut wanted = BTreeMap::from([(old.clone(), THE_OLD_BYTES.to_vec())]);
    for n in 1..=30 {
        let name = format!("DSCF{n:04}.RAF");
        wanted.insert(raw.join(&name), on_card(&a.0, &name));
    }
    wanted.insert(raw.join("DSCF0100.RAF"), on_card(&b.0, "DSCF0100.RAF"));
    wanted.insert(raw.join("DSCF0200.RAF"), on_card(&c, "DSCF0200.RAF"));
    let on_drive = files_under(&library);
    assert_eq!(
        on_drive.keys().collect::<Vec<_>>(),
        wanted.keys().collect::<Vec<_>>(),
        "the exact list"
    );
    assert!(on_drive == wanted, "a file is not its card's");
    for (card, before) in [&a.0, &b.0, &c].iter().zip(&before) {
        assert_eq!(&manifest(card), before, "INV-1: {}", card.display());
    }
}

/// (c) too: what only a tall real result can show. Five syncs of one long card, each failed, as
/// the result names each. A fresh Simulator has no offset of the Emulator's, so `scroll_probe`.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn c_a_real_result_too_tall_for_its_box_opens_the_next_report_at_its_top() {
    let on = OnDrive::new(&twin_cards().0, "S057-tall");
    let card = on.0.join("C".repeat(200));
    fs::create_dir_all(card.join("DCIM")).unwrap();
    let (ours, library) = a_drive("S057-tall");
    collision_in(&library, "Tall", "DSCF0099.RAF");
    photo_on(&card, "DSCF0099.RAF", b"not the older photo");
    let kept = Kept::default();
    let held = Scrolled::default();

    let mut steps = vec![keeping_the_session(&kept)];
    steps.extend(to_the_report_named("Tall"));
    for _ in 0..5 {
        steps.extend(the_same_card_again());
    }
    let at_the_end = held.clone();
    steps.extend([
        scroll_probe(RESULT, &held),
        set({
            let at_the_top = held.clone();
            move |_| {
                let (from, height, content) = at_the_top.lock().unwrap().expect("no box");
                assert_eq!(from, 0.0);
                assert!(content > height, "{content} fits in {height}: nothing to scroll");
            }
        }),
        Step::Send(
            M::TaskForTest(gui::ForTask(Arc::new(|| {
                iced::widget::operation::snap_to_end(RESULT)
            }))),
            1,
        ),
        scroll_probe(RESULT, &held),
        set(move |_| {
            let (from, height, content) = at_the_end.lock().unwrap().expect("no box");
            assert!(from > 0.0, "it did not move");
            assert!((from + height).ceil() >= content.floor(), "{from} {height} {content}");
        }),
    ]);
    steps.extend(the_same_card_again());
    steps.push(scroll_probe(RESULT, &held));
    let gui = play_steps(literal(&[&shown(&card)], &[&shown(&ours.0)]), steps);

    let (from, ..) = held.lock().unwrap().expect("no box");
    assert_eq!(from, 0.0, "the next report opens with its result at its top");
    let (_, result, complete) = the_report_is_the_sessions(&gui, &kept);
    assert!(!complete);
    assert!(result.contains("7 of 7 syncs not complete"), "{result}");
    assert_eq!(gui.app.exit().code, 1);
    picture_at(&gui, WINDOW, "S057", "c_tall_result");
}

/// Hands on what the session hears, as the real worker's does.
struct Heard(WorkerSend);

impl Watch for Heard {
    fn scanned(&mut self, scan: &SourceScan) {
        self.0.send(Msg::Scanned(text::scanned(scan)));
    }

    fn event(&mut self, event: CoreEvent) {
        self.0.send(Msg::Progress(event));
    }
}

/// (d) What the real worker does, with `LoseOnCopy` round the sync, as the TUI's test has it: a
/// file planning read, gone when it is to be copied. `worker::spawn_with` is the shared crate's.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn d_a_twin_that_cannot_be_read_is_named_never_plain_complete() {
    let (a, b) = (a_card(TWIN_A, "S057-d"), a_card(TWIN_B, "S057-d"));
    let (ours, library) = a_drive("S057-d");
    let lost = photo_on(&a.0, "DSCF0001.RAF", b"on both cards");
    photo_on(&b.0, "DSCF0001.RAF", b"on both cards");
    photo_on(&a.0, "DSCF0002.RAF", b"on the first alone");
    let before_b = manifest(&b.0);
    let (card_a, card_b) = (shown(&a.0), shown(&b.0));
    let kept = Kept::default();

    let (keeping, to_lose) = (kept.clone(), lost.clone());
    let losing = set(move |gui| {
        let (keeping, to_lose) = (keeping.clone(), to_lose.clone());
        gui.test_worker = Some(Arc::new(move |job: Job, send: WorkerSend| {
            *keeping.lock().unwrap() = Some(job.session.clone());
            let to_lose = to_lose.clone();
            thread::spawn(move || {
                let mut heard = Heard(send.clone());
                let cancel: &Cancel = &job.cancel;
                {
                    let mut session = flow::lock(&job.session);
                    session.sync(
                        &job.sources,
                        cancel,
                        &mut LoseOnCopy {
                            path: to_lose,
                            inner: &mut heard,
                        },
                    );
                }
                send.send(Msg::Finished);
            });
        }));
    });
    let mut steps = vec![losing, click_on(&card_a), click_on(&card_b)];
    steps.extend(to_the_report_named(""));
    let gui = play_steps(literal(&[&card_a, &card_b], &[&shown(&ours.0)]), steps);

    let (lines, result, complete) = the_report_is_the_sessions(&gui, &kept);
    assert!(complete, "every photo is on the drive: {result}");
    assert_eq!(gui.app.exit().code, 0);
    drawn_as(&gui, true, "s067_d");
    let warning =
        format!("1 could not be read on {card_a} and was copied from another card: check that card.");
    assert!(result.ends_with(&warning), "{result}");
    assert_ne!(
        result,
        "Result: all 2 photos or videos on the drive (2 copied, 0 already there).",
        "plain complete"
    );
    let own = line_saying(&lines, "copied from its twin");
    assert!(own.contains("DSCF0001.RAF"), "{own}");
    let folder = library.join(date_command(TAKEN)).join("RAW");
    assert_eq!(fs::read(folder.join("DSCF0001.RAF")).unwrap(), b"on both cards");
    assert_eq!(fs::read(folder.join("DSCF0002.RAF")).unwrap(), b"on the first alone");
    assert_eq!(files_under(&library).len(), 2);
    assert_eq!(manifest(&b.0), before_b, "INV-1");
    assert_eq!(gui.syncs_started, 1);
    a_report_screen_check(&gui, "d_report");
}

/// (e) Then what stands in for it on the cards and on "Before you quit".
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn e_a_name_collision_fails_red_keeps_the_old_bytes_and_stays_in_sight() {
    let a = a_card(TWIN_A, "S057-e");
    let (ours, library) = a_drive("S057-e");
    let old = collision_in(&library, "Trip", "DSCF0001.RAF");
    let old_before = manifest(&library);
    let new = photo_on(&a.0, "DSCF0001.RAF", b"another photo of that name");
    let before = manifest(&a.0);
    let kept = Kept::default();
    let said = Arc::new(Mutex::new(String::new()));

    let (kept_1, new_1, old_1, into) = (kept.clone(), new.clone(), old.clone(), said.clone());
    let mut steps = vec![keeping_the_session(&kept)];
    steps.extend(to_the_report_named("Trip"));
    steps.push(set(move |gui| {
        let (lines, result, complete) = the_report_is_the_sessions(gui, &kept_1);
        assert!(!complete, "{result}");
        assert_eq!(gui.app.exit().code, 1);
        assert!(result.starts_with("Result: NOT COMPLETE: 1 failed"), "{result}");
        assert!(result.contains("Do not format the card"), "{result}");
        assert!(result.contains("start photosync again"), "{result}");
        let told = format!(
            "  failed      {}: name collision with {}",
            shown(&new_1),
            shown(&old_1)
        );
        assert!(lines.contains(&told), "{told} in {lines:#?}");
        drawn_as(gui, false, "s057_e");
        a_report_screen_check(gui, "e_report");
        assert_eq!(gui.app.exit().message, None, "so the banner has the result");
        *into.lock().unwrap() = result;
    }));
    steps.push(Step::Line(r#"click "Next card""#));
    let told = said.clone();
    steps.push(set(move |gui| {
        assert_eq!(gui.app.screen(), Screen::Sources);
        the_banner_says(gui, &told.lock().unwrap());
        picture_at(gui, WINDOW, "S057", "e_banner");
    }));
    steps.push(Step::Line(r#"click "Quit""#));
    let told = said.clone();
    steps.push(set(move |gui| {
        assert!(gui.before_quit(), "the cards do not say why by themselves");
        assert!(!gui.quit);
        let words = all_words(gui, WINDOW);
        assert!(words.contains(&BEFORE_YOU_QUIT.to_string()), "{words:?}");
        assert!(words.contains(&told.lock().unwrap().clone()), "{words:?}");
        fixed_in_sight(gui, LEAST, &[NOT_SAFE, "Back", QUIT_ANYWAY]);
        picture_at(gui, WINDOW, "S057", "e_before_quit");
    }));
    steps.push(Step::Line(r#"click "Back""#));
    steps.push(Step::Line(r#"click "Quit""#));
    steps.push(Step::Line(r#"click "Quit anyway""#));
    let gui = play_steps(literal(&[&shown(&a.0)], &[&shown(&ours.0)]), steps);

    assert!(gui.quit);
    let shared = the_session(&kept);
    let report = text::session_report(&flow::lock(&shared)).unwrap();
    the_process_ends_with(&gui, 1, &report);
    assert_eq!(fs::read(&old).unwrap(), THE_OLD_BYTES, "INV-2");
    let now = manifest(&library);
    assert_eq!(now.get(&old), old_before.get(&old), "INV-2, its time too");
    assert_eq!(files_under(&library).len(), 1, "the new one is not there");
    no_temp_under(&library);
    assert_eq!(manifest(&a.0), before, "INV-1");
}

/// (f) The check names it, and the sync refuses: INV-10's sentence, nothing written.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn f_a_library_on_a_card_is_refused_and_nothing_is_written() {
    let a = a_card(TWIN_A, "S057-f");
    let card = image_path(CARD);
    let ours = OnDrive::new(&card, "S057-f");
    let library = ours.0.join("Photos");
    fs::create_dir(&library).unwrap();
    photo_on(&a.0, "DSCF0001.RAF", b"a photo");
    let (before, on_drive) = (manifest(&a.0), manifest(&ours.0));
    let kept = Kept::default();

    let mut steps = vec![keeping_the_session(&kept)];
    steps.extend(to_the_report_named("Trip"));
    let gui = play_steps(literal(&[&shown(&a.0)], &[&shown(&ours.0)]), steps);

    assert_eq!(gui.app.screen(), Screen::Refused);
    assert_eq!(gui.app.exit().code, 2);
    let sentence = the_sentence(&gui);
    assert_eq!(
        sentence,
        format!(
            "The library {} is on {}, which is a camera card (it has a DCIM folder). \
             Pick a library folder on your external drive (`photosync drives` lists them).",
            shown(&library),
            shown(&card)
        )
    );
    let shared = the_session(&kept);
    match &flow::lock(&shared).runs()[0].end {
        RunEnd::Refused(why) => assert_eq!(text::refusal(why), sentence),
        _ => panic!("not refused"),
    }
    assert_eq!(manifest(&ours.0), on_drive, "nothing written");
    assert_eq!(manifest(&a.0), before, "INV-1");
    assert!(!says(&gui, "Report"));
    screen_check(&gui, "S057", "f_report", &[TITLE, &sentence, REFUSED_BACK, QUIT]);
    fixed_in_sight(&gui, LEAST, &[TITLE, REFUSED_BACK, QUIT]);
}

/// The cancel's one chance: the copy thread waits here for the test, once, on the first file.
fn holding_the_first_file(held: Arc<AtomicBool>, go_on: Receiver<()>, waited_out: Arc<AtomicBool>) -> Step {
    let go_on = Arc::new(Mutex::new(go_on));
    set(move |gui| {
        let (held, go_on, waited_out) = (held.clone(), go_on.clone(), waited_out.clone());
        gui.before_send = Some(Arc::new(move |msg| {
            let Msg::Progress(event) = msg else { return };
            if event.progress.phase == Phase::Copying
                && matches!(event.kind, EventKind::FileStarted(_))
                && !held.swap(true, Ordering::SeqCst)
                && go_on.lock().unwrap().recv_timeout(LIMIT).is_err()
            {
                waited_out.store(true, Ordering::SeqCst);
            }
        }));
    })
}

/// How many the result says were cancelled.
fn cancelled_in(result: &str) -> u64 {
    let (before, _) = result
        .split_once(" cancelled")
        .unwrap_or_else(|| panic!("no cancelled in {result}"));
    before.rsplit(' ').next().unwrap().parse().unwrap()
}

/// (g) DEC-072 (6). Cancel's message comes from a click on the real screen, so a button drawn
/// held fails here too. "Hashes" are whole bytes, as `manifest`'s are: stricter, and no hash
/// of the core's is public.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn g_a_cancel_that_cannot_miss_leaves_no_doubt_on_the_drive() {
    let a = a_card(TWIN_A, "S057-g");
    let (ours, library) = a_drive("S057-g");
    let names = ["DSCF0001.RAF", "DSCF0002.RAF"];
    let bytes = [big_photo(1), big_photo(2)];
    assert_ne!(bytes[0], bytes[1]);
    for (name, bytes) in names.iter().zip(&bytes) {
        photo_on(&a.0, name, bytes);
    }
    let before = manifest(&a.0);
    let folder = dated(&library, "Trip").join("RAW");

    let (held, waited_out) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
    let (let_go, go_on) = mpsc::channel();
    let doing = Meanwhile {
        held: held.clone(),
        ask: Arc::new(|gui: &mut Gui| {
            assert_eq!(gui.app.screen(), Screen::Running);
            let messages = clicks(gui, "Cancel");
            assert!(
                matches!(messages[..], [M::Cancel]),
                "a click on Cancel sends {messages:?}"
            );
            messages
        }),
        after: Arc::new(|gui: &mut Gui| {
            assert!(gui.app.running().is_some_and(|run| run.cancelling), "not heard");
            fixed_in_sight(gui, LEAST, &[TITLE, DO_NOT_FORMAT, CANCELLING, CLOSE_AGAIN]);
            assert!(clicks(gui, "Cancel").is_empty(), "held once heard");
            picture_at(gui, WINDOW, "S057", "g_cancelling");
        }),
        release: Box::new(move || let_go.send(()).unwrap()),
    };
    let mut steps = vec![holding_the_first_file(
        held.clone(),
        go_on,
        waited_out.clone(),
    )];
    let mut to_report = to_the_report_named("Trip");
    to_report.pop();
    steps.extend(to_report);
    steps.push(Step::LineWhile(r#"click "Start copying""#, doing));

    let (library_1, folder_1) = (library.clone(), folder.clone());
    let cards_bytes: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|n| n.to_string())
        .zip(bytes.iter().cloned())
        .collect();
    let holds = cards_bytes.clone();
    steps.push(set(move |gui| {
        let (_, result, complete) = the_report(gui);
        assert!(!complete, "{result}");
        assert!(cancelled_in(&result) >= 1, "{result}");
        assert!(result.starts_with("Result: NOT COMPLETE:"), "{result}");
        assert_eq!(gui.app.exit().code, 1);
        drawn_as(gui, false, "s057_g_first");
        a_report_screen_check(gui, "g_first_report");
        // What is under a final name is the card's, to the byte. The rest is not there.
        let on_drive = files_under(&library_1);
        for (path, found) in &on_drive {
            let name = path.file_name().unwrap().to_str().unwrap();
            let on_card = holds.iter().find(|(n, _)| n == name);
            let on_card = on_card.unwrap_or_else(|| panic!("what is {}", path.display()));
            assert!(on_card.1 == *found, "{} is not the card's", path.display());
            assert_eq!(path.parent().unwrap(), folder_1);
        }
        assert!(on_drive.len() < 2, "the cancel cancelled nothing on the drive");
        no_temp_under(&library_1);
    }));
    steps.extend(the_same_card_again());
    let gui = play_steps(literal(&[&shown(&a.0)], &[&shown(&ours.0)]), steps);

    assert!(held.load(Ordering::SeqCst), "the copy thread was not held");
    assert!(!waited_out.load(Ordering::SeqCst), "no cancel came in time");
    let (lines, result, complete) = the_report(&gui);
    assert!(!complete, "{result}");
    assert!(result.contains("1 of 2 syncs not complete"), "{result}");
    assert_eq!(gui.app.exit().code, 1, "{lines:#?}");
    drawn_as(&gui, false, "s057_g");
    a_report_screen_check(&gui, "g_report");
    let on_drive = files_under(&library);
    assert_eq!(on_drive.len(), 2);
    for (name, wanted) in names.iter().zip(&bytes) {
        assert!(on_drive[&folder.join(name)] == *wanted, "{name}");
    }
    no_temp_under(&library);
    assert_eq!(manifest(&a.0), before, "INV-1");
}

/// (h) The later card is complete, and nothing of the first is lost: not on the report, not in
/// the code, not on the cards after it.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn h_a_complete_card_after_a_failed_one_hides_nothing() {
    let (a, b) = (a_card(TWIN_A, "S057-h"), a_card(TWIN_B, "S057-h"));
    let (ours, library) = a_drive("S057-h");
    let old = collision_in(&library, "Trip", "DSCF0001.RAF");
    photo_on(&a.0, "DSCF0001.RAF", b"another photo of that name");
    photo_on(&b.0, "DSCF0002.RAF", b"a photo with nothing wrong");
    let (before_a, before_b) = (manifest(&a.0), manifest(&b.0));
    let (card_a, card_b) = (shown(&a.0), shown(&b.0));
    let kept = Kept::default();
    let said = Arc::new(Mutex::new(String::new()));

    let mut steps = vec![keeping_the_session(&kept), click_on(&card_a)];
    steps.extend(to_the_report_named("Trip"));
    steps.extend([
        Step::Line(r#"click "Next card""#),
        click_on(&card_b),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
    ]);
    let (kept_2, named, into) = (kept.clone(), card_a.clone(), said.clone());
    steps.push(set(move |gui| {
        let (_, result, complete) = the_report_is_the_sessions(gui, &kept_2);
        assert!(!complete, "{result}");
        assert_eq!(gui.app.exit().code, 1);
        assert!(
            result.starts_with(&format!(
                "Result: NOT COMPLETE: 1 of 2 syncs not complete: sync 1 ({named}): 1 failed"
            )),
            "{result}"
        );
        assert!(result.contains("Do not format the cards of those syncs"), "{result}");
        drawn_as(gui, false, "s057_h");
        a_report_screen_check(gui, "h_report");
        let shared = the_session(&kept_2);
        let session = flow::lock(&shared);
        assert!(matches!(&session.runs()[1].end, RunEnd::Report(r) if r.totals().copied.files == 1));
        *into.lock().unwrap() = gui.app.exit().message.unwrap_or_default();
    }));
    steps.push(Step::Line(r#"click "Next card""#));
    let told = said.clone();
    steps.push(set(move |gui| {
        // The earlier warning is still on the next card.
        let message = told.lock().unwrap().clone();
        assert!(message.starts_with("Result: NOT COMPLETE"), "{message}");
        the_banner_says(gui, &message);
    }));
    let gui = play_steps(literal(&[&card_a, &card_b], &[&shown(&ours.0)]), steps);

    assert_eq!(gui.app.exit().code, 1);
    assert_eq!(fs::read(&old).unwrap(), THE_OLD_BYTES, "INV-2");
    assert_eq!(
        fs::read(dated(&library, "Trip").join("RAW/DSCF0002.RAF")).unwrap(),
        b"a photo with nothing wrong"
    );
    assert_eq!(manifest(&a.0), before_a, "INV-1");
    assert_eq!(manifest(&b.0), before_b, "INV-1");
}

/// (i) S066. A double click on close on a small sync: the first cancels, the sync ends at once,
/// and the second lands on the report. It must leave the report up (DEC-085 (1)).
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn i_the_second_click_of_a_double_on_close_leaves_the_report_a_fast_sync_raised() {
    let a = a_card(TWIN_A, "S066-i");
    let (ours, _library) = a_drive("S066-i");
    photo_on(&a.0, "DSCF0001.RAF", &big_photo(1));
    photo_on(&a.0, "DSCF0002.RAF", &big_photo(2));

    let (held, waited_out) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
    let (let_go, go_on) = mpsc::channel();
    let doing = Meanwhile {
        held: held.clone(),
        ask: Arc::new(|gui: &mut Gui| {
            assert_eq!(gui.app.screen(), Screen::Running);
            vec![M::CloseRequested]
        }),
        after: Arc::new(|gui: &mut Gui| {
            assert!(gui.app.running().is_some_and(|run| run.cancelling), "not heard");
        }),
        release: Box::new(move || let_go.send(()).unwrap()),
    };
    let mut steps = vec![
        set(|gui| clock_on(gui, Duration::ZERO)),
        holding_the_first_file(held.clone(), go_on, waited_out.clone()),
    ];
    let mut to_report = to_the_report_named("");
    to_report.pop();
    steps.extend(to_report);
    steps.push(Step::LineWhile(r#"click "Start copying""#, doing));
    let mut gui = play_steps(literal(&[&shown(&a.0)], &[&shown(&ours.0)]), steps);

    assert!(held.load(Ordering::SeqCst), "the copy thread was not held");
    assert!(!waited_out.load(Ordering::SeqCst), "no cancel came in time");
    let (_, result, complete) = the_report(&gui);
    assert!(!complete, "{result}");
    let _ = update(&mut gui, M::CloseRequested);
    assert!(!gui.quit, "the report would go before it is read");
    assert_eq!(gui.app.screen(), Screen::Report);
    assert!(gui.exit_slot.lock().unwrap().is_none());
    a_report_screen_check(&gui, "i_second_close");

    clock_on(&mut gui, CLOSE_GUARD);
    let _ = update(&mut gui, M::CloseRequested);
    assert!(gui.quit, "past the guard, a close on a report quits as before");
    let exit = gui.exit_slot.lock().unwrap().clone().expect("no Exit");
    assert_eq!(exit.code, 1);
}

/// Given no reading, and given it back on drop, on a panic too: a build folder holding a folder
/// of mode 000 could not be removed. Made after the card, so it drops first. Only a kill leaves
/// one (DEC-087 (4)).
struct Locked(PathBuf);

impl Locked {
    fn new(path: PathBuf) -> Locked {
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        Locked(path)
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        if let Err(e) = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700)) {
            eprintln!("{}: {e}: chmod it, or the build folder cannot be removed", self.0.display());
        }
    }
}

/// A card on the binary's own disk, with a real `DCIM`: chmod does nothing on exFAT. Its library
/// is on an image, so INV-10 lets it through.
fn a_card_with_chmod(name: &str) -> OnDrive {
    let card = OnDrive::new(&disk_of_the_binary(), name);
    fs::create_dir(card.0.join("DCIM")).unwrap();
    card
}

/// (j) What no image can give: a folder of the card that cannot be read, with the rest read. A
/// green box or code 0 here would tell the user to format a card with photos on it nobody saw.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn j_a_card_read_only_in_part_is_not_green_and_says_do_not_format() {
    let card = a_card_with_chmod("S067-j");
    let (ours, library) = a_drive("S067-j");
    photo_on(&card.0, "DSCF0001.RAF", b"read");
    let closed = card.0.join("DCIM/101_FUJI");
    write_file(&closed, "DSCF0002.RAF", b"never seen", at(TAKEN));
    let before = manifest(&card.0);
    let locked = Locked::new(closed.clone());
    let kept = Kept::default();

    let mut steps = vec![keeping_the_session(&kept)];
    steps.extend(to_the_report_named(""));
    let gui = play_steps(literal(&[&shown(&card.0)], &[&shown(&ours.0)]), steps);

    let (lines, result, complete) = the_report_is_the_sessions(&gui, &kept);
    assert!(!complete, "{result}");
    assert_eq!(gui.app.exit().code, 1);
    assert_eq!(
        result,
        "Result: NOT COMPLETE: 0 failed, 0 cancelled, 1 unreadable. Do not format the card. Fix \
         the cause and run the same sync again: files already on the drive are skipped."
    );
    let told = line_saying(&lines, "  unreadable  ");
    assert!(told.contains(&shown(&closed)), "{told}");
    assert!(told.contains("Permission denied"), "{told}");
    drawn_as(&gui, false, "s067_j");
    a_report_screen_check(&gui, "j_report");

    let raw = library.join(date_command(TAKEN)).join("RAW");
    let wanted = BTreeMap::from([(raw.join("DSCF0001.RAF"), b"read".to_vec())]);
    assert_eq!(files_under(&library), wanted, "only what was read");
    drop(locked);
    assert_eq!(manifest(&card.0), before, "INV-1");
}

/// (k) A file the scan lists and the copy cannot open, on the one card there is: failed, by the
/// real worker, with no seam.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn k_a_real_read_failure_with_no_twin_fails_in_the_words_of_text() {
    let card = a_card_with_chmod("S067-k");
    let (ours, library) = a_drive("S067-k");
    photo_on(&card.0, "DSCF0001.RAF", b"read");
    let closed = photo_on(&card.0, "DSCF0002.RAF", b"cannot be read");
    let before = manifest(&card.0);
    let locked = Locked::new(closed.clone());
    let kept = Kept::default();

    let mut steps = vec![keeping_the_session(&kept)];
    steps.extend(to_the_report_named(""));
    let gui = play_steps(literal(&[&shown(&card.0)], &[&shown(&ours.0)]), steps);

    let (lines, result, complete) = the_report_is_the_sessions(&gui, &kept);
    assert!(!complete, "{result}");
    assert_eq!(gui.app.exit().code, 1);
    assert_eq!(
        result,
        "Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable. Do not format the card. Fix \
         the cause and run the same sync again: files already on the drive are skipped."
    );
    let told = line_saying(&lines, "  failed      ");
    assert!(told.contains(&shown(&closed)), "{told}");
    assert!(told.contains("Permission denied"), "{told}");
    drawn_as(&gui, false, "s067_k");
    a_report_screen_check(&gui, "k_report");

    let raw = library.join(date_command(TAKEN)).join("RAW");
    let wanted = BTreeMap::from([(raw.join("DSCF0001.RAF"), b"read".to_vec())]);
    assert_eq!(files_under(&library), wanted, "no copy of what could not be read");
    no_temp_under(&library);
    drop(locked);
    assert_eq!(manifest(&card.0), before, "INV-1");
}

/// (l) A real refusal in a session: the first card's sync makes the session, so the check is
/// behind us, and then the library goes as a drive does. `between` runs at the refusal, with the
/// library back, and the same path synced again ends the walk on its report.
fn refused_then_again(
    first: &str,
    card: &str,
    library: &Path,
    kept: &Kept,
    between: impl Fn() + Send + Sync + 'static,
) -> Vec<Step> {
    let aside = library.with_file_name("Photos-aside");
    let (from, to) = (library.to_path_buf(), aside.clone());
    let mut steps = vec![keeping_the_session(kept), click_on(first)];
    steps.extend(to_the_report_named(""));
    steps.extend([
        set(move |gui| {
            assert_eq!(gui.app.exit().code, 0, "the first card is complete");
            fs::rename(&from, &to).unwrap();
        }),
        Step::Line(r#"click "Next card""#),
        click_on(card),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
    ]);
    let (back, to) = (library.to_path_buf(), aside);
    let shown_library = shown(library);
    steps.push(set(move |gui| {
        assert_eq!(gui.app.screen(), Screen::Refused);
        assert_eq!(gui.app.exit().code, 2);
        let sentence = the_sentence(gui);
        assert!(sentence.contains(&shown_library), "{sentence}");
        assert!(says(gui, NEXT_CARD), "a refusal in a session offers the next card");
        fs::rename(&to, &back).unwrap();
        between();
    }));
    steps.extend([
        Step::Line(r#"click "Next card""#),
        click_on(card),
        Step::Line(r#"click "Next""#),
        Step::Line(r#"click "Start copying""#),
    ]);
    steps
}

/// (l) DEC-050, as `a_refused_sync_fixed_and_retried_exits_0` has it of the CLI: every photo is
/// on the drive, so code 0, green, no "do not format", and the refusal stays in the report.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn l_a_refused_sync_then_the_same_card_complete_clears_as_the_clis() {
    let (first, again) = (a_card(TWIN_A, "S067-l-first"), a_card(TWIN_B, "S067-l-same"));
    let (ours, library) = a_drive("S067-l-same");
    photo_on(&first.0, "DSCF0001.RAF", b"on the first card");
    photo_on(&again.0, "DSCF0002.RAF", b"on the card refused");
    let (card_first, card) = (shown(&first.0), shown(&again.0));
    let kept = Kept::default();

    let mut steps = refused_then_again(&card_first, &card, &library, &kept, || {});
    steps.push(set(|gui| {
        drawn_as(gui, true, "s067_l_same");
        a_report_screen_check(gui, "l_same_report");
    }));
    steps.push(Step::Line(r#"click "Next card""#));
    steps.push(set(|gui| {
        assert_eq!(gui.app.screen(), Screen::Sources);
        assert!(!says(gui, NOT_SAFE), "nothing is left to warn of");
    }));
    steps.push(Step::Line(r#"click "Quit""#));
    let gui = play_steps(literal(&[&card_first, &card], &[&shown(&ours.0)]), steps);

    assert!(gui.quit, "code 0, so Quit asks nothing");
    assert_eq!(gui.app.exit().code, 0);
    let shared = the_session(&kept);
    let session = flow::lock(&shared);
    assert_eq!(session.verdict(), Verdict::Complete);
    assert_eq!(session.runs()[1].verdict(), Verdict::Refused, "it stays refused");
    assert!(session.completed_later(1));
    let report = text::session_report(&session).unwrap();
    let result = text::session_result(&session);
    assert!(!result.contains("Do not format"), "{result}");
    assert!(!result.contains("NOT COMPLETE"), "{result}");
    assert_eq!(
        result,
        "Result: all 2 photos or videos on the drive (2 copied, 0 already there), over 3 syncs."
    );
    assert!(report.contains("Its cards completed later."), "{report}");
    assert!(report.contains(&format!("Sync 2 of 3: {card}\n")), "{report}");
    the_process_ends_with(&gui, 0, &report);
    let raw = library.join(date_command(TAKEN)).join("RAW");
    assert_eq!(
        files_under(&library),
        BTreeMap::from([
            (raw.join("DSCF0001.RAF"), b"on the first card".to_vec()),
            (raw.join("DSCF0002.RAF"), b"on the card refused".to_vec()),
        ])
    );
}

/// (l) A swapped card comes back at that path, and its sync is complete. The refused card's
/// photos are on no drive, so nothing clears: code 2, red, "do not format", as the CLI has it.
#[test]
#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]
fn l_a_swapped_card_at_that_path_clears_nothing() {
    let (first, swapped) = (a_card(TWIN_A, "S067-l-other"), a_card(TWIN_B, "S067-l-swap"));
    let (ours, library) = a_drive("S067-l-swap");
    photo_on(&first.0, "DSCF0001.RAF", b"on the first card");
    photo_on(&swapped.0, "DSCF0002.RAF", b"on the card refused");
    let (card_first, card) = (shown(&first.0), shown(&swapped.0));
    let kept = Kept::default();

    let dcim = swapped.0.join("DCIM");
    let root = swapped.0.clone();
    let swap = move || {
        assert_may_delete(&dcim);
        fs::remove_dir_all(&dcim).unwrap();
        fs::create_dir(&dcim).unwrap();
        photo_on(&root, "DSCF0003.RAF", b"on another card, at that path");
    };
    let steps = refused_then_again(&card_first, &card, &library, &kept, swap);
    let gui = play_steps(literal(&[&card_first, &card], &[&shown(&ours.0)]), steps);

    let (_, result, complete) = the_report_is_the_sessions(&gui, &kept);
    assert!(!complete, "{result}");
    assert_eq!(gui.app.exit().code, 2);
    assert!(
        result.starts_with(&format!(
            "Result: NOT COMPLETE: 1 of 3 syncs not complete: sync 2 ({card}): refused, nothing \
             copied."
        )),
        "{result}"
    );
    assert!(result.contains("Do not format"), "{result}");
    let shared = the_session(&kept);
    {
        let session = flow::lock(&shared);
        assert_eq!(session.verdict(), Verdict::Refused);
        assert!(!session.completed_later(1));
    }
    drawn_as(&gui, false, "s067_l_swap");
    a_report_screen_check(&gui, "l_swap_report");
    let raw = library.join(date_command(TAKEN)).join("RAW");
    assert_eq!(
        files_under(&library),
        BTreeMap::from([
            (raw.join("DSCF0001.RAF"), b"on the first card".to_vec()),
            (raw.join("DSCF0003.RAF"), b"on another card, at that path".to_vec()),
        ]),
        "the refused card's photo is not there"
    );
}
