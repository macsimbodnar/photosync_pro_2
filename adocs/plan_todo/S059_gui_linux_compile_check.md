id:         S059
goal:       prove from this Mac that the GUI compiles for Linux; nothing runs there (DEC-070 (3))
accepts:    `scripts/check_linux_from_mac.sh`: makes a throwaway `RUSTUP_HOME` under `$TMPDIR`, installs there the toolchain of `rustc --version`, `--profile minimal`, targets `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-gnu`, `--no-self-update`, runs `cargo check --workspace --all-targets` for both with its own `CARGO_TARGET_DIR`, leaves the user's `~/.rustup` as it was and removes what it made; exit code says it; each error in code that is really Linux-only becomes a fix if trivial or a step of its own; the stamp says what this does not prove: no window opened, no test run, on Linux; no screen changes here, and no screen check is claimed: S061 sees the Linux window; suite passes
touches:    scripts, README
excludes:   running anything on Linux; the AppImage
closes:     
paused_by:
author:
done:
