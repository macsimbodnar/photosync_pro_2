id:         S001
goal:       Cargo workspace with photosync-core (lib) and photosync (bin), test suite running
accepts:    `cargo test --workspace` green with at least one test per crate; `cargo build` builds both; photosync-core has no dependency on clap/ratatui/crossterm/iced (INV-8, checked by a test or a `cargo tree` check in the suite); README states what the tool is and how to build and test it
touches:    Cargo.toml, crates/photosync-core, crates/photosync, README.md, .gitignore
excludes:   any sync logic; the GUI crate
closes:
paused_by:
author:
done:
