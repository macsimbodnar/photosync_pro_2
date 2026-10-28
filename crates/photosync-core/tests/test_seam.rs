//! S016: a front-end cannot fake a source's device. `SourceScan::device` is
//! read-only outside the crate, and the seam the tests use to fake it exists
//! only with the `test-support` feature, which only the dev-dependency on
//! this crate turns on (DEC-027).

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;
use common::TempDir;

/// Checks a tiny crate that uses photosync-core as a front-end does, with
/// no features: reading the device compiles; writing it, through the field
/// or the seam, does not. Needs the registry crates of this workspace in
/// the local cargo cache (`--offline`): building this workspace put them
/// there.
#[test]
fn source_device_is_read_only_in_a_normal_build() {
    let dir = TempDir::new();
    let probe = dir.0.join("probe");
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(probe.join("src")).unwrap();
    fs::write(
        probe.join("Cargo.toml"),
        format!(
            "[package]\nname = \"probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nphotosync-core = {{ path = {:?} }}\n\n[workspace]\n",
            core.display().to_string()
        ),
    )
    .unwrap();
    // The workspace's lock, so `--offline` finds the same versions it built.
    fs::copy(core.join("../../Cargo.lock"), probe.join("Cargo.lock")).unwrap();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let cases = [
        ("let _: u64 = scan.device();", None),
        ("scan.device = 1;", Some("E0616")),
        ("scan.set_device_for_test(1);", Some("E0599")),
    ];
    for (line, error) in cases {
        fs::write(
            probe.join("src/main.rs"),
            format!(
                "#[allow(unused_mut)]\nfn main() {{\n    let mut scan = \
                 photosync_core::scan_source(std::path::Path::new(\".\")).unwrap();\n    \
                 {line}\n}}\n"
            ),
        )
        .unwrap();
        let out = Command::new(&cargo)
            .args(["check", "--offline", "--quiet", "--message-format", "short"])
            .current_dir(&probe)
            .env("CARGO_TARGET_DIR", probe.join("target"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        match error {
            None => assert!(out.status.success(), "{line}: should compile:\n{stderr}"),
            Some(code) => {
                assert!(!out.status.success(), "{line}: compiled in a normal build");
                assert!(stderr.contains(code), "{line}: expected {code}:\n{stderr}");
            }
        }
    }
}
