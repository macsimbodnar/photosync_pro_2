id:         S001
goal:       Cargo workspace with photosync-core (lib) and photosync (bin), test suite running
accepts:    `cargo test --workspace` green with at least one test per crate; `cargo build` builds both; photosync-core has no dependency on clap/ratatui/crossterm/iced (INV-8, checked by a test or a `cargo tree` check in the suite); README states what the tool is and how to build and test it; an INV-8 check fails if photosync-core depends on a UI or logging crate (incl. egui, log, tracing) or its source contains print!/println!/eprint!/eprintln!/dbg! or std::io::stdout/stderr; README states the `source ~/.cargo/env` setup needed for cargo
touches:    Cargo.toml, crates/photosync-core, crates/photosync, README.md, .gitignore
excludes:   any sync logic; the GUI crate
closes:     2026-10-28_adversarial-F11
paused_by:
author:
done:
