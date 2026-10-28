# photosync

Copies photos from a Fujifilm X-T5 (SD cards, or the camera over USB in "USB
card reader" mode) to an external drive. RAW, JPEG, HEIF and video go to
separate folders under one folder per sync, optionally named
(`2026-10-28 - Tirana/RAW/`). Photos already in the library are skipped. Mac
first, then Linux.

Status: early development. The binary only prints its version so far. What it
must do is in `adocs/specs.md`, and the plan is in `adocs/plan.md`.

## Layout

- `crates/photosync-core`: all sync logic, UI-free (INV-8).
- `crates/photosync`: the `photosync` binary (CLI, later the TUI).

## Build and test

Needs a Rust toolchain (rustup). If `cargo` is not on `PATH`, load it first:

```sh
source "$HOME/.cargo/env"        # bash/zsh
source "$HOME/.cargo/env.fish"   # fish
```

Then:

```sh
cargo build
cargo test --workspace
```

### exFAT (macOS)

`scripts/test_exfat.sh` runs the `photosync-core` tests on a temporary exFAT
disk image (the common format for a Mac + Linux drive). It needs `hdiutil`,
works from any directory, keeps the image in a temp dir outside the repo, and
always detaches it. The tests that need Unix permission bits, which exFAT does
not have, are skipped; the script lists them.

```sh
scripts/test_exfat.sh
```
