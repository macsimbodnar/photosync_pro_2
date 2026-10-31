id:         S050
goal:       give the layer the front-ends share a library target, so a GUI crate can use it, with no behaviour change (DEC-067)
accepts:    `crates/photosync/Cargo.toml` gains `[lib] name = "photosync_front"`, `path = "src/lib.rs"`, `test = false`, `doctest = false`, and `[[bin]] name = "photosync"`, `path = "src/main.rs"`; new `crates/photosync/src/lib.rs` holds a doc comment saying why (DEC-067), `#![allow(dead_code)]`, `#![allow(clippy::new_without_default)]` and `pub mod app; pub mod args; pub mod flow; pub mod system; pub mod text; pub mod worker;` and nothing else; the proof is the suite unchanged: `git diff --exit-code HEAD~1 -- crates/photosync/tests crates/photosync-core scripts crates/photosync/src/main.rs crates/photosync/src/app.rs crates/photosync/src/args.rs crates/photosync/src/flow.rs crates/photosync/src/system.rs crates/photosync/src/text.rs crates/photosync/src/worker.rs crates/photosync/src/tui` is empty after the commit; `cargo test --workspace` passes with exactly as many passed and ignored as before the step (372 and 28 at S049; say both counts in the stamp); `scripts/test_exfat.sh` exits 0 and `hdiutil info` shows no image left; `cargo build -p photosync --lib` and `cargo clippy -p photosync --lib` give no warning; no iced in any Cargo file; `Cargo.lock` unchanged; no screen changes, and the TUI's drawn-screen tests, unchanged, are its screen check; README's layout paragraph names the library target
touches:    crates/photosync/Cargo.toml, crates/photosync/src/lib.rs, README
excludes:   the GUI crate; iced; moving any file; changing main.rs or any test
closes:     
paused_by:
author:
done:
