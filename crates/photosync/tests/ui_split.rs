//! The model and the sync's plumbing must not depend on the terminal, so a
//! native GUI can reuse them as they are: only `src/tui/` may name ratatui
//! or crossterm (S009). Checked from the source text, as `no_test_seam.rs`
//! does: all modules are in one crate, which depends on ratatui as a whole.
//!
//! S018: every `.rs` file under `src/` but `src/tui/` is checked, so a new
//! module or submodule is checked without being listed. Outside `main.rs`
//! (which opens the TUI) none may reach into `tui` either, and `tui` may
//! not re-export a terminal type: either would bring the terminal in by
//! another name.

use std::fs;
use std::path::{Path, PathBuf};

const TERMINAL: &[&str] = &["ratatui", "crossterm"];

/// Every `.rs` file under `dir`, any depth.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `line` without its `//` comment: the files' docs name the crates they
/// must not use. A `//` inside a string cuts too early, which can only
/// hide code after it on the same line.
fn code(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// `word` as a whole identifier: `ratatui::x` and `ratatui ::x` count,
/// `my_ratatui` does not.
fn names(code: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    code.match_indices(word).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + word.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}

/// The lines of `path` whose code matches `bad`, as `path:line: text`.
fn lines_where(path: &Path, bad: impl Fn(&str) -> bool) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap();
    text.lines()
        .enumerate()
        .filter(|(_, line)| bad(code(line)))
        .map(|(n, line)| format!("{}:{}: {}", path.display(), n + 1, line.trim()))
        .collect()
}

fn names_terminal(code: &str) -> bool {
    TERMINAL.iter().any(|t| names(code, t))
}

/// A path into the `tui` module, or a file pulled in from elsewhere, which
/// this walk would not see.
fn reaches_outside(code: &str) -> bool {
    names(code, "tui") || code.contains("#[path")
}

/// A re-export (`pub use`, `pub(crate) use`) or a public alias.
fn re_exports(code: &str) -> bool {
    let code = code.trim_start();
    let Some(rest) = code.strip_prefix("pub") else {
        return false;
    };
    let rest = match rest.trim_start().strip_prefix('(') {
        Some(scoped) => scoped.split_once(')').map_or("", |(_, r)| r),
        None => rest,
    };
    let rest = rest.trim_start();
    rest.starts_with("use ") || rest.starts_with("type ")
}

struct Split {
    src: PathBuf,
    /// Front-end neutral: everything but `main.rs` and `src/tui/`.
    shared: Vec<PathBuf>,
    terminal: Vec<PathBuf>,
}

fn split() -> Split {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut all = Vec::new();
    rust_files(&src, &mut all);
    let (terminal, rest): (Vec<_>, Vec<_>) = all
        .into_iter()
        .partition(|p| p.starts_with(src.join("tui")));
    let shared = rest
        .into_iter()
        .filter(|p| *p != src.join("main.rs"))
        .collect();
    Split {
        src,
        shared,
        terminal,
    }
}

#[test]
fn model_worker_flow_and_text_never_name_the_terminal() {
    let Split {
        src,
        shared,
        terminal,
    } = split();
    let mut found: Vec<String> = shared
        .iter()
        .flat_map(|f| lines_where(f, |c| names_terminal(c) || reaches_outside(c)))
        .collect();
    found.extend(lines_where(&src.join("main.rs"), names_terminal));
    found.extend(
        terminal
            .iter()
            .flat_map(|f| lines_where(f, |c| re_exports(c) && names_terminal(c))),
    );
    assert!(
        found.is_empty(),
        "the terminal reached outside src/tui:\n{}",
        found.join("\n")
    );

    // Not vacuous: the walk finds the known modules on both sides, and the
    // view does name the crates, in the form looked for.
    for f in [
        "app.rs",
        "worker.rs",
        "flow.rs",
        "text.rs",
        "args.rs",
        "system.rs",
    ] {
        assert!(shared.contains(&src.join(f)), "{f} not checked");
    }
    for f in ["tui/view.rs", "tui/mod.rs"] {
        let path = src.join(f);
        assert!(terminal.contains(&path), "{f} not found");
        assert!(!lines_where(&path, names_terminal).is_empty(), "{f}");
    }
}

/// The matchers on the shapes they must catch, and not catch.
#[test]
fn the_matchers_catch_each_way_in() {
    for bad in [
        "use ratatui::style::Color;",
        "use ratatui :: style;",
        "    crossterm::terminal::enable_raw_mode()?;",
        "use {std::io, ratatui};",
        "extern crate crossterm;",
    ] {
        assert!(names_terminal(code(bad)), "{bad}");
    }
    for fine in [
        "//! no ratatui here",
        "let x = 1; // not crossterm",
        "let my_ratatui_free = 1;",
    ] {
        assert!(!names_terminal(code(fine)), "{fine}");
    }
    for bad in [
        "pub use crate::tui::view::render;",
        "use super::tui;",
        "use crate::{app, tui::Failed};",
        "#[path = \"../tui/view.rs\"]",
    ] {
        assert!(reaches_outside(code(bad)), "{bad}");
    }
    for bad in [
        "pub use ratatui::style::Color;",
        "pub(crate) use ratatui::Frame;",
        "  pub(super) use ratatui::text::Line;",
        "pub type Screen = ratatui::Frame<'static>;",
    ] {
        assert!(re_exports(code(bad)) && names_terminal(code(bad)), "{bad}");
    }
    assert!(!re_exports("use ratatui::Frame;"));
    assert!(!re_exports("pub fn render(app: &App, frame: &mut Frame) {"));
}
