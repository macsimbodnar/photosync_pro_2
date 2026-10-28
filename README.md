# photosync

Copies photos from a Fujifilm X-T5 (SD cards, or the camera over USB in "USB
card reader" mode) to an external drive. RAW, JPEG, HEIF and video go to
separate folders under one folder per sync, optionally named
(`2026-10-28 - Tirana/RAW/`). Photos already in the library are skipped. Mac
first, then Linux.

Status: early development. The CLI and the TUI work; a GUI comes later. What
it must do is in `adocs/specs.md`, and the plan is in `adocs/plan.md`.

## Usage

Run `photosync` with no command, in a terminal, for the interactive view
(TUI). It walks through five steps: pick the cards, the drive, the library
folder on it (default `<drive>/Photos`; it must exist), an optional name,
then check and start. It shows the progress with an ETA, then the report,
which is printed again when you quit. The exit code is the same as `sync`'s.
Without a terminal (stdin or stdout redirected), `photosync` alone prints
the usage and exits 2.

| Screen | Keys |
|---|---|
| cards | `↑` `↓` move, `Space` pick (several cards are copied one after another), `Enter` next (picks the card under the cursor if none is picked), `r` look again, `q` quit |
| drive | `↑` `↓` move, `Enter` pick, `Esc` back, `r` look again, `q` quit |
| library folder, name | type to edit, `Backspace`, `Enter` next, `Esc` back |
| check | `Enter` start copying, `Esc` back, `q` quit |
| copying | `c` or `Esc` cancel; `Ctrl-C` cancels, a second `Ctrl-C` quits at once |
| refused | `Enter` back to fix it, `q` quit |
| report | `↑` `↓` `PgUp` `PgDn` scroll, `q` quit |

`Ctrl-C` quits from any screen but the copy. Quitting during a copy is safe
(no partial file is left under a photo's name), but exits 1: run the same
sync again.

The commands, for scripts:

```sh
photosync sources        # mounted camera cards (volumes with DCIM), one per line
photosync drives         # mounted drives you can sync to, one per line
photosync sync --source /Volumes/X-T5 --dest /Volumes/SSD/Photos --name Tirana
```

The X-T5 over a USB cable shows up as a card only in its "USB card reader"
connection mode, not in the default PTP mode. `--source` can be given
several times; the cards are copied one after another into one sync folder,
`<library>/<date>[ - <name>]/<RAW|JPEG|HEIF|VIDEO>/`. The library folder
must exist. It is never on a card, nor on the same drive as a source.
Photos already anywhere in the library are skipped. Ctrl-C cancels
between chunks and still prints the report; a second Ctrl-C quits at once.
photosync refuses to run as root.

Progress goes to stderr, the report to stdout:

```text
scanned /Volumes/X-T5: 3 photos or videos (24 B), 1 ignored, 0 unreadable, device 16777239
copying 0/3 files, 0 B of 24 B (0%), ETA --
copying 3/3 files, 24 B of 24 B (100%), ETA 0s
Report

Source /Volumes/X-T5
           copied   skipped   failed   cancelled
  RAW           1         0        0           0
  JPEG          1         0        0           0
  VIDEO         1         0        0           0
  all           3         0        0           0
  size       24 B       0 B      0 B         0 B
  ignored 1 file (.txt 1)

Result: all 3 photos or videos on the drive (3 copied, 0 already there).
```

Every file that was not copied or skipped gets its own line with the
reason; a copy whose folder flush failed shows as "in place, flush not
confirmed". On a terminal the progress line is cut to its width. A closed
stdout (e.g. `| head`) does not stop the sync: photosync says on stderr that
the report could not be written and keeps the exit code.

Exit codes:

- `0` every photo and video is on the drive (copied, or already there).
- `1` some files failed or were cancelled, or part of a card could not be
  read. Do not format the card; fix the cause and run the same sync again.
- `2` usage error, or the sync was refused before copying anything.

`sources` and `drives` exit 1 if their list cannot be written to stdout.

## Layout

- `crates/photosync-core`: all sync logic, UI-free (INV-8).
- `crates/photosync`: the `photosync` binary. `app` is the interactive
  model (screens, keys, what each message changes) and `worker` runs a sync
  on its own thread; neither names ratatui or crossterm
  (`tests/ui_split.rs`), so a GUI can reuse them with `flow` (one sync) and
  `text` (sentences, the report). `tui` and `tui/view` are the terminal
  shell around them.

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

`scripts/test_exfat.sh` runs the workspace tests on a temporary exFAT
disk image (the common format for a Mac + Linux drive). It needs `hdiutil`,
works from any directory, keeps the image in a temp dir outside the repo, and
detaches it at the end. If the detach fails, it says so, leaves the image and
its temp dir in place, prints how to clean up, and exits non-zero. The tests that need Unix permission bits, which exFAT does
not have, are skipped; the script lists them. It also mounts a second small
exFAT image with a `DCIM` folder, a stand-in card, and passes its path in
`PHOTOSYNC_TEST_CARD` to the end-to-end test that a library on a card is
refused. A third image stands in for the external drive
(`PHOTOSYNC_TEST_LIBRARY`): the script also runs the `photosync` binary's
tests, and `tests/exfat_e2e.rs` syncs the stand-in card to it, re-syncs,
and checks the files and the report text; the binary's unit test
`tui::tests::tui_sync_on_exfat_end_to_end` drives the TUI's loop through a
cancelled and a full sync there, with scripted keys on ratatui's
`TestBackend`. Plain `cargo test` skips these
tests and says so; it covers the binary's help, usage errors and refusals
on temp dirs. The tests reach `photosync sources` and `drives` through a
hidden `--search-root` option.

The tests turn on photosync-core's `test-support` feature (through a
dev-dependency of the crate on itself) for a seam that fakes a source's
device. A normal `cargo build` never has it.

```sh
scripts/test_exfat.sh
```
