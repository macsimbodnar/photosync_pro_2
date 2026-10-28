//! INV-8: the core depends on no terminal, GUI or logging crate and never
//! writes to stdout or stderr. Checked from Cargo.lock (transitive, all
//! dependency kinds) and from the core's source text.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const CORE: &str = "photosync-core";

// Families match by prefix (iced_core, egui_extras, tracing-subscriber, ...).
const FORBIDDEN_PREFIXES: &[&str] = &[
    "clap", "ratatui", "crossterm", "termion", "cursive", "iced", "egui", "eframe", "slint",
    "gtk", "winit", "tracing",
];
const FORBIDDEN_EXACT: &[&str] = &["log", "env_logger", "simplelog", "fern"];
const FORBIDDEN_TOKENS: &[&str] = &[
    "print!", "println!", "eprint!", "eprintln!", "dbg!", "stdout", "stderr",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Package name -> names of its dependencies, from Cargo.lock text.
fn parse_lock(text: &str) -> HashMap<String, Vec<String>> {
    let mut graph = HashMap::new();
    for block in text.split("[[package]]").skip(1) {
        let mut name = None;
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("name = ") {
                name = Some(v.trim_matches('"').to_string());
            } else if line.starts_with("dependencies = [") {
                in_deps = !line.ends_with(']');
            } else if in_deps {
                if line == "]" {
                    in_deps = false;
                } else {
                    // Entries look like "name" or "name 1.2.3".
                    let entry = line.trim_end_matches(',').trim_matches('"');
                    deps.push(entry.split(' ').next().unwrap().to_string());
                }
            }
        }
        graph.insert(name.expect("package block without name"), deps);
    }
    graph
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

/// (line number, line) of non-comment lines holding a forbidden token.
fn output_violations(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
        .filter(|(_, l)| FORBIDDEN_TOKENS.iter().any(|t| l.contains(t)))
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect()
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
