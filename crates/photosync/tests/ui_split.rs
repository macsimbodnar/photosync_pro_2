//! The model and the sync's plumbing must not depend on the terminal, so a
//! native GUI can reuse them as they are: only `src/tui/` may name ratatui
//! or crossterm (S009). Checked from the source text, as `no_test_seam.rs`
//! does: all modules are in one crate, which depends on ratatui as a whole.

use std::fs;
use std::path::Path;

/// Front-end neutral: the model, the worker thread, one sync, its text.
const SHARED: &[&str] = &["app.rs", "worker.rs", "flow.rs", "text.rs", "args.rs"];
const TERMINAL: &[&str] = &["ratatui::", "crossterm::", "use ratatui", "use crossterm"];

fn uses(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap();
    text.lines()
        .enumerate()
        .filter(|(_, line)| TERMINAL.iter().any(|t| line.contains(t)))
        .map(|(n, line)| format!("{}:{}: {}", path.display(), n + 1, line.trim()))
        .collect()
}

#[test]
fn model_worker_flow_and_text_never_name_the_terminal() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found: Vec<String> = SHARED.iter().flat_map(|f| uses(&src.join(f))).collect();
    assert!(
        found.is_empty(),
        "terminal crates outside src/tui:\n{}",
        found.join("\n")
    );
    // Not vacuous: the view does name them, in the form looked for.
    assert!(!uses(&src.join("tui/view.rs")).is_empty());
    assert!(!uses(&src.join("tui/mod.rs")).is_empty());
}
