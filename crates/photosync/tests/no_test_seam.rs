//! DEC-027: core's `test-support` seam must never be used by the binary.
//! Workspace-wide test and all-targets builds unify dev-dependency features,
//! so the compiler alone does not stop the binary from reaching it there.

use std::fs;
use std::path::Path;

const SEAM: &[&str] = &["set_device_for_test", "test-support"];

fn seam_uses(dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            seam_uses(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            check_file(&path, out);
        }
    }
}

fn check_file(path: &Path, out: &mut Vec<String>) {
    let text = fs::read_to_string(path).unwrap();
    for (n, line) in text.lines().enumerate() {
        if SEAM.iter().any(|s| line.contains(s)) {
            out.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
        }
    }
}

#[test]
fn binary_never_uses_the_core_test_seam() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    seam_uses(&crate_dir.join("src"), &mut found);
    check_file(&crate_dir.join("Cargo.toml"), &mut found);
    assert!(found.is_empty(), "DEC-027: the binary uses the test seam:\n{}", found.join("\n"));
}
