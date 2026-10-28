//! INV-8: the core depends on no terminal, GUI or logging crate and never
//! writes to stdout or stderr. Checked from Cargo.lock (transitive, all
//! dependency kinds) and from the core's source text.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const CORE: &str = "photosync-core";

// Families match by prefix (iced_core, egui_extras, tracing-subscriber, ...).
const FORBIDDEN_PREFIXES: &[&str] = &[
    "clap", "ratatui", "tui", "crossterm", "termion", "termcolor", "cursive", "console",
    "indicatif", "anstream", "nu-ansi-term", "ansi_term", "owo-colors", "iced", "egui",
    "eframe", "slint", "gtk", "winit", "tracing", "slog", "log4rs", "flexi_logger",
];
const FORBIDDEN_EXACT: &[&str] = &[
    "log", "env_logger", "pretty_env_logger", "simplelog", "simple_logger", "fern", "colored",
    "yansi",
];
const FORBIDDEN_TOKENS: &[&str] = &[
    "print!", "println!", "eprint!", "eprintln!", "dbg!", "stdout", "stderr",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Package name -> names of its dependencies, from Cargo.lock text. Versions
/// of one crate are merged: over-approximating the graph can only make the
/// check stricter.
fn parse_lock(text: &str) -> HashMap<String, Vec<String>> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for section in text.split("\n[[").skip(1) {
        let Some(block) = section.strip_prefix("package]]") else {
            continue; // e.g. [[patch.unused]]
        };
        let mut name = None;
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("name = ") {
                name = Some(v.trim_matches('"').to_string());
            } else if let Some(rest) = line.strip_prefix("dependencies = [") {
                in_deps = !rest.ends_with(']');
                deps.extend(dep_names(rest.trim_end_matches(']')));
            } else if in_deps {
                in_deps = line != "]";
                deps.extend(dep_names(line.trim_end_matches(']')));
            }
        }
        let name = name.expect("package block without name");
        graph.entry(name).or_default().extend(deps);
    }
    graph
}

// Entries look like "name" or "name 1.2.3", comma separated.
fn dep_names(list: &str) -> impl Iterator<Item = String> + '_ {
    list.split(',')
        .map(|e| e.trim().trim_matches('"'))
        .filter(|e| !e.is_empty())
        .map(|e| e.split(' ').next().unwrap().to_string())
}

fn closure(graph: &HashMap<String, Vec<String>>, root: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![root.to_string()];
    while let Some(pkg) = todo.pop() {
        if seen.insert(pkg.clone()) {
            todo.extend(graph.get(&pkg).cloned().unwrap_or_default());
        }
    }
    seen.remove(root);
    seen
}

fn is_forbidden(pkg: &str) -> bool {
    FORBIDDEN_EXACT.contains(&pkg) || FORBIDDEN_PREFIXES.iter().any(|p| pkg.starts_with(p))
}

fn has_token(code: &str) -> bool {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    FORBIDDEN_TOKENS.iter().any(|t| compact.contains(t))
}

/// (line number, line) of non-comment lines holding a forbidden token.
/// Whitespace is ignored, so `println !` and a macro split across lines are
/// caught; the latter is reported as line 0.
fn output_violations(source: &str) -> Vec<(usize, String)> {
    let code: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
        .collect();
    let mut found: Vec<(usize, String)> = code
        .iter()
        .filter(|(_, l)| has_token(l))
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect();
    let joined: Vec<&str> = code.iter().map(|(_, l)| *l).collect();
    if found.is_empty() && has_token(&joined.join("")) {
        found.push((0, "token split across lines".to_string()));
    }
    found
}

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

#[test]
fn core_has_no_ui_or_logging_dependency() {
    let lock = fs::read_to_string(workspace_root().join("Cargo.lock"))
        .expect("Cargo.lock missing; build the workspace first");
    let graph = parse_lock(&lock);
    assert!(graph.contains_key(CORE), "{CORE} not in Cargo.lock");
    let bad: Vec<_> = closure(&graph, CORE).into_iter().filter(|p| is_forbidden(p)).collect();
    assert!(bad.is_empty(), "INV-8: {CORE} depends on {bad:?}");
}

#[test]
fn core_source_never_writes_to_stdout_or_stderr() {
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(!files.is_empty());
    let mut bad = Vec::new();
    for f in files {
        for (n, l) in output_violations(&fs::read_to_string(&f).unwrap()) {
            bad.push(format!("{}:{n}: {}", f.display(), l.trim()));
        }
    }
    assert!(bad.is_empty(), "INV-8 violations:\n{}", bad.join("\n"));
}

// The checks above pass on a clean tree; these prove they would fail.

#[test]
fn checker_catches_transitive_forbidden_dependency() {
    let lock = r#"
[[package]]
name = "photosync-core"
version = "0.1.0"
dependencies = [
 "helper",
]

[[package]]
name = "helper"
version = "1.0.0"
dependencies = [
 "tracing-core 0.1.0",
]

[[package]]
name = "tracing-core"
version = "0.1.0"
"#;
    let deps = closure(&parse_lock(lock), CORE);
    assert!(deps.iter().any(|p| is_forbidden(p)));
    assert!(!is_forbidden("lock_api"));
    assert!(!is_forbidden("logos"));
}

#[test]
fn checker_catches_output_calls_but_not_comments() {
    let src = "// println! in a comment is fine\nfn f() { eprintln!(\"x\"); }\nlet o = std::io::stdout();\n";
    let found: Vec<_> = output_violations(src).into_iter().map(|(n, _)| n).collect();
    assert_eq!(found, vec![2, 3]);
}

#[test]
fn checker_merges_deps_of_duplicate_package_names() {
    let lock = r#"
[[package]]
name = "photosync-core"
version = "0.1.0"
dependencies = [
 "foo 1.0.0",
]

[[package]]
name = "foo"
version = "1.0.0"
dependencies = [
 "log",
]

[[package]]
name = "foo"
version = "2.0.0"

[[package]]
name = "log"
version = "0.4.0"
"#;
    assert!(closure(&parse_lock(lock), CORE).contains("log"));
}

#[test]
fn checker_reads_one_line_dependency_arrays_and_ignores_patch_sections() {
    let lock = r#"
[[package]]
name = "photosync-core"
version = "0.1.0"
dependencies = ["indicatif", "helper"]

[[patch.unused]]
name = "other"
version = "1.0.0"
"#;
    let graph = parse_lock(lock);
    assert!(!graph.contains_key("other"));
    let deps = closure(&graph, CORE);
    assert!(deps.contains("indicatif") && deps.contains("helper"));
}

#[test]
fn checker_knows_common_terminal_and_logging_crates() {
    for p in [
        "tui", "console", "indicatif", "termcolor", "colored", "owo-colors", "anstream",
        "nu-ansi-term", "ansi_term", "yansi", "slog", "log4rs", "flexi_logger",
    ] {
        assert!(is_forbidden(p), "{p} not forbidden");
    }
}

#[test]
fn checker_catches_output_calls_with_odd_whitespace() {
    assert_eq!(output_violations("fn f() { println !(\"x\"); }").len(), 1);
    assert_eq!(output_violations("fn f() {\n    print\n    !(\"x\");\n}").len(), 1);
}
