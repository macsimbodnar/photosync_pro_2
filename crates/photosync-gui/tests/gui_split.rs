//! What the GUI must keep to, checked from the source text as `ui_split.rs` is
//! (DEC-066 (7), DEC-072 (1), (4)). Each matcher is a plain function, and a scratch
//! line of each kind shows it catches what it looks for.

use std::fs;
use std::path::{Path, PathBuf};

const HELPERS: &str = "widgets.rs";
/// The only reason an `#[ignore` may give under `src`.
const IMAGES_REASON: &str =
    r#"#[ignore = "needs disk images; on macOS ignored_image_tests_pass_on_real_mounts runs it"]"#;
const NOT_THE_GUIS: &[&str] = &[
    "ratatui",
    "crossterm",
    "copy_plan",
    "plan_sync",
    "set_device_for_test",
];
/// Draw their own words, past the helper.
const OWN_WORDS: &[&str] = &[
    "radio",
    "toggler",
    "pick_list",
    "combo_box",
    "text_editor",
    "tooltip",
    "markdown",
];
/// What a string literal may not be the first thing in.
const NO_LITERAL_IN: &[&str] = &[
    "button(",
    "container(",
    "center(",
    "scrollable(",
    "mouse_area(",
    "column![",
    "row![",
    ".push(",
];

fn gui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every `.rs` file under `dir`, any depth.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
    found
}

/// The GUI's non-test source. All unit tests are in `src/tests`.
fn product_files() -> Vec<PathBuf> {
    let src = gui_dir().join("src");
    let files: Vec<PathBuf> = rust_files(&src)
        .into_iter()
        .filter(|f| !f.starts_with(src.join("tests")))
        .collect();
    // Not vacuous: the walk found what is known to be there.
    for known in ["lib.rs", "main.rs", "gui.rs", HELPERS] {
        assert!(files.contains(&src.join(known)), "{known} in {files:?}");
    }
    files
}

/// Every file of the GUI but this one, which must name what it forbids.
fn all_gui_files() -> Vec<PathBuf> {
    let dir = gui_dir();
    let mut files = rust_files(&dir.join("src"));
    files.extend(rust_files(&dir.join("tests")));
    files.retain(|f| f.file_name().unwrap() != "gui_split.rs");
    assert!(
        files.contains(&dir.join("tests/smoke_window.rs")),
        "{files:?}"
    );
    assert!(
        files.contains(&dir.join("src/tests/harness.rs")),
        "{files:?}"
    );
    files
}

/// `source` with its `//` comments gone: the files' docs name what they must not use.
/// A `//` inside a string cuts too early, which can only hide code after it on that line.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_code(path: &Path) -> String {
    code(&fs::read_to_string(path).unwrap())
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where `word` stands with no identifier running on before it, and after it if it ends
/// in one: `ratatui::x` counts, `my_ratatui` does not.
fn places<'a>(source: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    source
        .match_indices(word)
        .map(|(at, _)| at)
        .filter(move |at| {
            let starts = !word.starts_with(is_ident)
                || !source[..*at].chars().next_back().is_some_and(is_ident);
            let ends = !word.ends_with(is_ident)
                || !source[*at + word.len()..]
                    .chars()
                    .next()
                    .is_some_and(is_ident);
            starts && ends
        })
}

fn names(source: &str, word: &str) -> bool {
    places(source, word).next().is_some()
}

/// Calls `iced`'s `text`, `button` or `checkbox`, or brings them in under another name.
fn reaches_word_widgets(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for widget in ["text", "button", "checkbox"] {
        for at in places(source, widget) {
            let after = source[at + widget.len()..].trim_start();
            if after.starts_with('(') || after.starts_with("::<") {
                found.push(format!("{widget}("));
            }
        }
        if names(source, &format!("widget::{widget}")) {
            found.push(format!("widget::{widget}"));
        }
    }
    if source.contains("widget::*") {
        found.push("widget::*".to_string());
    }
    // A `use iced::widget::{...}` group, over as many lines as it likes.
    for (at, _) in source.match_indices("widget::{") {
        let group = &source[at..];
        let group = &group[..group.find('}').unwrap_or(group.len())];
        for widget in ["text", "button", "checkbox"] {
            if names(group, widget) {
                found.push(format!("widget::{{{widget}}}"));
            }
        }
    }
    found
}

/// The forms that draw words past the helper (DEC-072 (4)).
fn draws_past_the_helper(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for form in [
        ".label(",
        "text!",
        "rich_text",
        "Text::new",
        "Text::from",
        "span(",
        "value(",
        "\".into()",
    ] {
        if names(source, form) {
            found.push(form.to_string());
        }
    }
    for widget in OWN_WORDS {
        if names(source, widget) {
            found.push((*widget).to_string());
        }
    }
    for opening in NO_LITERAL_IN {
        for at in places(source, opening) {
            if source[at + opening.len()..].trim_start().starts_with('"') {
                found.push(format!("{opening}\""));
            }
        }
    }
    found
}

/// Each `#[ignore` that does not give the images' reason.
fn other_ignores(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.contains("#[ignore") && line.trim() != IMAGES_REASON)
        .map(|line| line.trim().to_string())
        .collect()
}

fn found_in(files: &[PathBuf], matcher: impl Fn(&str) -> Vec<String>) -> Vec<String> {
    files
        .iter()
        .flat_map(|file| {
            matcher(&read_code(file))
                .into_iter()
                .map(move |what| format!("{}: {what}", file.display()))
        })
        .collect()
}

#[test]
fn no_shared_file_names_iced() {
    let shared = gui_dir().join("../photosync/src");
    let files = rust_files(&shared);
    assert!(files.iter().any(|f| f.ends_with("app.rs")), "{files:?}");
    assert!(
        files.iter().any(|f| f.ends_with("tui/view.rs")),
        "{files:?}"
    );
    let found: Vec<_> = files
        .iter()
        .filter(|file| names(&read_code(file), "iced"))
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn no_gui_file_names_the_terminal_or_what_skips_a_session() {
    let found = found_in(&all_gui_files(), |source| {
        NOT_THE_GUIS
            .iter()
            .filter(|word| names(source, word))
            .map(|word| word.to_string())
            .collect()
    });
    assert!(found.is_empty(), "{found:?}");
    let manifest = fs::read_to_string(gui_dir().join("Cargo.toml")).unwrap();
    for word in NOT_THE_GUIS.iter().chain(&["test-support"]) {
        assert!(!manifest.contains(word), "{word} in Cargo.toml");
    }
    for file in all_gui_files() {
        assert!(
            !read_code(&file).contains("test-support"),
            "{}",
            file.display()
        );
    }
}

#[test]
fn the_library_check_stays_on() {
    let product: String = product_files().iter().map(|f| read_code(f)).collect();
    assert!(product.contains("App::checking_library"), "DEC-050 (7)");
    assert!(!names(&product, "App::new"), "DEC-050 (7)");
}

#[test]
fn a_close_is_heard_not_obeyed() {
    let product: String = product_files().iter().map(|f| read_code(f)).collect();
    assert!(
        product.contains("exit_on_close_request(false)"),
        "DEC-069 (4)"
    );
    assert!(product.contains("close_requests"), "DEC-069 (4)");
}

#[test]
fn only_the_helpers_file_calls_text_button_and_checkbox() {
    let others: Vec<PathBuf> = product_files()
        .into_iter()
        .filter(|f| f.file_name().unwrap() != HELPERS)
        .collect();
    let found = found_in(&others, reaches_word_widgets);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_helper_is_the_one_call_of_text_and_it_wraps_by_glyph() {
    let helpers = read_code(&gui_dir().join("src").join(HELPERS));
    let calls = places(&helpers, "text")
        .filter(|at| helpers[at + "text".len()..].starts_with('('))
        .count();
    assert_eq!(calls, 1, "iced's text is called by one helper");
    assert!(helpers.contains("text(content).wrapping(Wrapping::WordOrGlyph)"));
}

#[test]
fn no_other_form_draws_words() {
    let found = found_in(&product_files(), draws_past_the_helper);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_ignore_under_src_has_the_images_reason() {
    let found = found_in(&rust_files(&gui_dir().join("src")), other_ignores);
    assert!(found.is_empty(), "{found:?}");
}

// Each matcher catches a line of each kind, and leaves a clean one alone.

#[test]
fn the_word_widget_matcher_catches_each_kind() {
    for line in [
        "let a = text(\"x\");",
        "let a = text (\"x\");",
        "let a = iced::widget::text(\"x\");",
        "let a = button(x);",
        "let a = checkbox(true);",
        "let a = text::<Theme>(x);",
        "use iced::widget::text as t;",
        "use iced::widget::button;",
        "use iced::widget::*;",
        "use iced::widget::{\n    column,\n    checkbox,\n};",
    ] {
        assert!(!reaches_word_widgets(line).is_empty(), "{line}");
    }
    for line in [
        "use photosync_front::text;",
        "let a = text::session_result(&s);",
        "use iced::widget::{column, text_input};",
        "let a = next_button(x);",
        "let a = words(x);",
    ] {
        assert_eq!(reaches_word_widgets(line), Vec::<String>::new(), "{line}");
    }
}

#[test]
fn the_form_matcher_catches_each_kind() {
    for line in [
        "a.label(\"x\")",
        "text!(\"x {y}\")",
        "rich_text([x])",
        "Text::new(x)",
        "Text::from(x)",
        "span(x)",
        "iced::widget::value(x)",
        "let e: Element<M> = \"x\".into();",
        "radio(x)",
        "toggler(x)",
        "pick_list(x)",
        "combo_box(x)",
        "text_editor(x)",
        "tooltip(x)",
        "markdown(x)",
        "button(\"x\")",
        "container(\"x\")",
        "center(\"x\")",
        "scrollable(\"x\")",
        "mouse_area(\"x\")",
        "column![\"x\"]",
        "row![\"x\"]",
        "a.push(\"x\")",
        "column![\n    \"x\",\n]",
    ] {
        assert!(!draws_past_the_helper(line).is_empty(), "{line}");
    }
    for line in [
        "column![words(x)]",
        "a.push(words(\"x\"))",
        "action(\"Next\", None)",
        "text_input(\"Name\", &n)",
        "my_value(x)",
        "x.to_string()",
    ] {
        assert_eq!(draws_past_the_helper(line), Vec::<String>::new(), "{line}");
    }
}

#[test]
fn the_ignore_matcher_catches_each_kind() {
    for line in [
        "#[ignore]",
        "#[ignore = \"slow\"]",
        "    #[ignore = \"needs disk images\"]",
    ] {
        assert!(!other_ignores(line).is_empty(), "{line}");
    }
    assert_eq!(
        other_ignores(&format!("    {IMAGES_REASON}")),
        Vec::<String>::new()
    );
}

#[test]
fn the_names_matcher_sees_whole_identifiers_and_comments_do_not_count() {
    assert!(names("ratatui::x", "ratatui"));
    assert!(names("use iced;", "iced"));
    assert!(!names("my_ratatui", "ratatui"));
    assert!(!names("sliced", "iced"));
    assert!(!names(&code("// ratatui is not named here"), "ratatui"));
    assert!(names(&code("ratatui // and here"), "ratatui"));
}
