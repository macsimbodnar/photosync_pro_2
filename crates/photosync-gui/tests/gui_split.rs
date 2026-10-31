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
/// Macros and calls that turn a `&str` into an element, by `From<&str> for Element`: what a
/// string literal may not be an item of. A macro has a `!`. Every macro and function of
/// iced_widget's `helpers.rs` that takes an element, and the `new` of the widgets with one.
const NO_LITERAL_IN: &[&str] = &[
    "column!",
    "row!",
    "stack!",
    "grid!",
    "keyed_column!",
    "button",
    "container",
    "center",
    "center_x",
    "center_y",
    "right",
    "right_center",
    "bottom",
    "bottom_center",
    "bottom_right",
    "pin",
    "opaque",
    "hover",
    "sensor",
    "themer",
    "float",
    "scrollable",
    "mouse_area",
    ".push",
    "Button::new",
    "Container::new",
    "Scrollable::new",
    "MouseArea::new",
    "Pin::new",
    "Float::new",
    "Sensor::new",
    "Themer::new",
    "Element::from",
];
/// Macros whose result is a `&str` made at compile time: a literal under another name.
const LITERAL_MACROS: &[&str] = &["concat!", "stringify!", "include_str!", "env!"];
/// The one function that places the body.
const THE_BODY: &str = "around_the_body";
/// May bound a height: a box whose words scroll inside it, and the body (DEC-080).
const MAY_BOUND_A_HEIGHT: &[&str] = &["word_box", THE_BODY];
/// Bound a height, and so can cut a fixed warning with nothing wrong in its bounds, which no
/// check of the harness can see (DEC-080). A container's `center` and `center_y` set one too.
const HEIGHT_BOUNDS: &[&str] = &[".height", ".max_height", ".clip", ".center", ".center_y"];

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
/// A `//` inside a string cuts too early, which can hide the rest of that line, and with
/// the string left open, the rest of a macro from `literal_items`. None is in the GUI yet.
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

/// How long the string or char literal at the start of `rest` is, if one starts there.
fn literal_len(rest: &str) -> Option<usize> {
    if let Some(after) = rest.strip_prefix('"') {
        let mut escaped = false;
        for (at, c) in after.char_indices() {
            match c {
                '"' if !escaped => return Some(1 + at + 1),
                '\\' => escaped = !escaped,
                _ => escaped = false,
            }
        }
        return Some(rest.len());
    }
    if let Some(after) = rest.strip_prefix('r') {
        let hashes = after.len() - after.trim_start_matches('#').len();
        if after[hashes..].starts_with('"') {
            let close = format!("\"{}", "#".repeat(hashes));
            let inside = hashes + 1;
            return Some(match after[inside..].find(&close) {
                Some(at) => 1 + inside + at + close.len(),
                None => rest.len(),
            });
        }
        return None;
    }
    let after = rest.strip_prefix('\'')?;
    // A lifetime has no closing quote just after its one letter.
    let inside = after.chars().next()?;
    if inside == '\\' {
        // Past the escaped one, which can be a quote.
        return after.get(2..)?.find('\'').map(|at| 1 + 2 + at + 1);
    }
    let width = inside.len_utf8();
    after[width..].starts_with('\'').then_some(1 + width + 1)
}

/// The items of a macro or call, with `source` starting just past its opening bracket: cut
/// at its own commas, not those in a literal or a nested bracket.
fn items(source: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let (mut at, mut start, mut depth) = (0, 0, 0);
    while at < source.len() {
        let rest = &source[at..];
        if let Some(len) = literal_len(rest) {
            at += len;
            continue;
        }
        let c = rest.chars().next().unwrap();
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth == 0 => break,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                found.push(&source[start..at]);
                start = at + 1;
            }
            _ => {}
        }
        at += c.len_utf8();
    }
    found.push(&source[start..at]);
    found
}

/// An item that is a string literal, in brackets or a tuple or not.
fn is_literal(item: &str) -> bool {
    let item = item.trim_start();
    if let Some(inside) = item.strip_prefix('(') {
        return items(inside).iter().any(|inner| is_literal(inner));
    }
    let raw = item
        .strip_prefix('r')
        .is_some_and(|after| after.trim_start_matches('#').starts_with('"'));
    raw || item.starts_with('"') || LITERAL_MACROS.iter().any(|name| item.starts_with(name))
}

/// Each macro or call of `NO_LITERAL_IN` with a literal for an item.
fn literal_items(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for opening in NO_LITERAL_IN {
        let brackets: &[char] = if opening.ends_with('!') {
            &['(', '[', '{']
        } else {
            &['(']
        };
        for at in places(source, opening) {
            let after = source[at + opening.len()..].trim_start();
            let Some(inside) = after.strip_prefix(brackets) else {
                continue;
            };
            if items(inside).iter().any(|item| is_literal(item)) {
                found.push(format!("{opening} with a literal"));
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
    found.extend(literal_items(source));
    found
}

/// Where the function `name` stands in `source`, from its `fn` to its closing brace.
fn function_of<'a>(source: &'a str, name: &str) -> Option<(usize, &'a str)> {
    let start = source.find(&format!("fn {name}"))?;
    let opening = start + source[start..].find('{')?;
    let inside = &source[opening + 1..];
    let (mut at, mut depth) = (0, 0);
    while at < inside.len() {
        let rest = &inside[at..];
        if let Some(len) = literal_len(rest) {
            at += len;
            continue;
        }
        let c = rest.chars().next().unwrap();
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => return Some((start, &source[start..opening + 1 + at + 1])),
            '}' => depth -= 1,
            _ => {}
        }
        at += c.len_utf8();
    }
    Some((start, &source[start..]))
}

/// Each bound on a height outside the named helpers.
fn height_bounds(source: &str) -> Vec<String> {
    let mut left = source.to_string();
    for helper in MAY_BOUND_A_HEIGHT {
        if let Some((start, text)) = function_of(&left, helper) {
            left.replace_range(start..start + text.len(), "");
        }
    }
    let mut found = Vec::new();
    for form in HEIGHT_BOUNDS {
        for at in places(&left, form) {
            // A call, not the field of a size or a bound.
            let after = left[at + form.len()..].trim_start();
            if after.starts_with('(') || after.starts_with("::<") {
                found.push((*form).to_string());
            }
        }
    }
    found
}

/// What keeps the body from hiding a cut: it is one, in one function, the middle of a column,
/// with all the height that is left. A body in a `row!` leaves the warnings beside it to run
/// past the window, and a body of fixed height pushes what is under it out.
fn body_faults(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let ids = source.matches(".id(BODY)").count();
    if ids != 1 {
        found.push(format!("{ids} bodies"));
    }
    match function_of(source, THE_BODY) {
        None => found.push(format!("no {THE_BODY}")),
        Some((_, helper)) => {
            if !helper.contains(".id(BODY)") {
                found.push(format!("the body is not in {THE_BODY}"));
            }
            let after = helper.find(".id(BODY)").map_or("", |at| &helper[at..]);
            if !after.contains(".height(Length::Fill)") {
                found.push("the body's height is not Length::Fill".to_string());
            }
            if !helper.contains("column!") || names(helper, "row!") || names(helper, "row") {
                found.push(format!("{THE_BODY} is not a column"));
            }
        }
    }
    for opening in ["row!", "row"] {
        for at in places(source, opening) {
            let after = source[at + opening.len()..].trim_start();
            let Some(inside) = after.strip_prefix(['(', '[', '{']) else {
                continue;
            };
            if items(inside).iter().any(|item| {
                item.contains(THE_BODY) || item.contains(".id(BODY)") || item.contains("BODY)")
            }) {
                found.push(format!("the body in {opening}"));
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
fn no_screen_bounds_a_height_outside_the_named_helpers() {
    let found = found_in(&product_files(), height_bounds);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_one_body_takes_all_the_height_left_in_a_column() {
    let product: String = product_files().iter().map(|f| read_code(f)).collect();
    let found = body_faults(&product);
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
        "Button::new(\"x\")",
        "iced::widget::Button::new(\"x\")",
        "Element::from(\"x\")",
        "Element::from(\n    \"x\",\n)",
        "Container::new(\"x\")",
        "Scrollable::new(\"x\")",
        // Any item of a macro, in any of its brackets, over any lines (S064).
        "column![a, \"x\"]",
        "column![a, b, \"x\",]",
        "row![a, \"x\"]",
        "row![\n    a,\n    \"x\",\n]",
        "column!(\"x\")",
        "column!{\"x\"}",
        "row!(a, \"x\")",
        "column![words(\"a, b\"), \"x\"]",
        "column![words(\"a]\"), \"x\"]",
        "column![words(x.replace('\"', \"\")), \"x\"]",
        "column![r\"x\"]",
        "column![a, r#\"x\"#]",
        "column![a, (\"x\")]",
        "column![a, concat!(\"x\", \"y\")]",
        "column![a, stringify!(x)]",
        "column![a, include_str!(\"x\")]",
        "stack![\"x\"]",
        "stack![a, \"x\"]",
        "grid![\"x\"]",
        "grid![a, \"x\"]",
        "keyed_column![(0, a), (1, \"x\")]",
        // The widgets of iced's helpers that take an element.
        "center_x(\"x\")",
        "center_y(\"x\")",
        "right(\"x\")",
        "right_center(\"x\")",
        "bottom(\"x\")",
        "bottom_center(\"x\")",
        "bottom_right(\"x\")",
        "pin(\"x\")",
        "opaque(\"x\")",
        "hover(\"x\", a)",
        "hover(a, \"x\")",
        "sensor(\"x\")",
        "themer(None, \"x\")",
        "float(\"x\")",
        "iced::widget::pin(\"x\")",
        "container (\"x\")",
        "a.push(1, \"x\")",
        "column![x.replace('\\'', \"\"), \"x\"]",
        "column![x.replace('\\\"', \"\"), \"x\"]",
        "column![Foo::<'a>::new(), \"x\"]",
        "MouseArea::new(\"x\")",
        "Pin::new(\"x\")",
        "Float::new(\"x\")",
        "Sensor::new(\"x\")",
        "Themer::new(None, \"x\")",
    ] {
        assert!(!draws_past_the_helper(line).is_empty(), "{line}");
    }
    for line in [
        "column![words(x)]",
        "a.push(words(\"x\"))",
        "column![words(x), words(\"a, b\")]",
        "row![action(\"a, \\\"b\\\"\", None), x]",
        "column![x.replace('\"', \"\"), y]",
        "stack![a, b]",
        "grid![a, words(\"x\")]",
        "right(words(\"x\"))",
        "hover(a, words(\"x\"))",
        "a.right(5)",
        "a.center_x(Length::Fill)",
        "bottom_line(\"x\")",
        "my_column![\"x\"]",
        "action(\"Next\", None)",
        "text_input(\"Name\", &n)",
        "my_value(x)",
        "x.to_string()",
    ] {
        assert_eq!(draws_past_the_helper(line), Vec::<String>::new(), "{line}");
    }
}

#[test]
fn the_height_matcher_catches_each_kind_outside_the_helpers() {
    for line in [
        "container(words(WARNING)).max_height(30)",
        "words(WARNING).height(20)",
        "row![a, b].height(30)",
        "container(a).clip(true)",
        "row![a, b].clip(true)",
        "a\n    .height(30)",
        "a.height (30)",
        "a.max_height::<f32>(30.0)",
        "container(a).center_y(30)",
        "container(a).center(30)",
        // A helper's name elsewhere is not the helper.
        "fn my_word_box_too() { a.height(30) }",
        "fn other() { word_box(a); b.clip(true) }",
    ] {
        assert!(!height_bounds(line).is_empty(), "{line}");
    }
    for line in [
        "fn word_box() { container(a).max_height(most) }",
        "fn around_the_body() { scrollable(a).height(Length::Fill) }",
        "word_box(id, words, 104.0)",
        "shot.size.height",
        "a.height_of(x)",
        "a.width(Length::Fill)",
        "a.girth(10.0)",
        "a.align_y(Alignment::Center)",
    ] {
        assert_eq!(height_bounds(line), Vec::<String>::new(), "{line}");
    }
}

#[test]
fn the_body_matcher_catches_each_kind() {
    let good = "fn around_the_body() { let b = scrollable(a).id(BODY).spacing(8).height(Length::Fill); column![o, b, u].into() }";
    assert_eq!(body_faults(good), Vec::<String>::new());
    assert_eq!(
        body_faults(&format!(
            "{good} fn s() {{ column![around_the_body(a, b, c)] }}"
        )),
        Vec::<String>::new()
    );
    for bad in [
        // No body, a second, or one outside the helper.
        "fn around_the_body() { column![a] }".to_string(),
        format!("{good} fn s() {{ scrollable(a).id(BODY) }}"),
        "fn s() { scrollable(a).id(BODY).height(Length::Fill) }".to_string(),
        // Not all the height left.
        "fn around_the_body() { let b = scrollable(a).id(BODY).height(Length::Fixed(400.0)); column![o, b, u] }".to_string(),
        "fn around_the_body() { let b = scrollable(a).id(BODY).height(400.0); column![o, b, u] }".to_string(),
        "fn around_the_body() { let b = scrollable(a).id(BODY); column![o, b, u] }".to_string(),
        // Height set on something before the id is not the body's.
        "fn around_the_body() { let b = scrollable(a.height(Length::Fill)).id(BODY); column![o, b, u] }".to_string(),
        // In a row, inside or outside the helper.
        "fn around_the_body() { let b = scrollable(a).id(BODY).height(Length::Fill); row![o, b] }".to_string(),
        format!("{good} fn s() {{ row![warnings, around_the_body(a, b, c)] }}"),
        format!("{good} fn s() {{ row!(around_the_body(a, b, c), warnings) }}"),
        format!("{good} fn s() {{ row(vec![around_the_body(a, b, c)]) }}"),
    ] {
        assert!(!body_faults(&bad).is_empty(), "{bad}");
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
