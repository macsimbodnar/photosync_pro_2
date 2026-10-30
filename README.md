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
(TUI); it calls itself "phoosync pro 2" (DEC-041). It walks through five steps: pick the cards, the drive, the library
folder on it (default `<drive>/Photos`; it must exist), an optional name,
then check and start. It shows the progress with an ETA, then the report,
which is printed again when you quit. The exit code is the same as `sync`'s.
After the report, `n` takes the next card (one card reader: swap the card)
into the same session: the same library, name and folder, straight to the
check. The report then shows every card of the session and the totals of
all. After a refusal you can go back and try again, or take the next card.
The exit code and the message on quitting are the worst of the session:
a sync that was not complete is named, and no later sync hides it. To
change the library or the name, quit and start again. Without a terminal
(stdin or stdout redirected), `photosync` alone prints the usage and exits
2.

| Screen | Keys |
|---|---|
| cards | `↑` `↓` move, `Space` pick (several cards are copied at once), `Enter` next (picks the card under the cursor if none is picked; for the next card of a session, straight to the check), `r` look again, `q` quit |
| drive | `↑` `↓` move, `Enter` pick, `Esc` back, `r` look again, `q` quit |
| library folder, name | type to edit, `Backspace`, `Enter` next, `Esc` back |
| check | `Enter` start copying, `Esc` back, `q` quit |
| copying | `c` or `Esc` cancel; `Ctrl-C` cancels, a second `Ctrl-C` quits at once |
| refused | `Enter` back to fix it, `n` next card, `q` quit |
| report | `↑` `↓` `PgUp` `PgDn` scroll, `n` next card, `q` quit |

`Ctrl-C` quits from any screen but the copy. Quitting during a copy is safe
(no partial file is left under a photo's name), but exits 1: run the same
sync again.

The commands, for scripts:

```sh
photosync sources        # mounted camera cards (volumes with DCIM), one per line
photosync drives         # mounted drives you can sync to, one per line
photosync sync --source /Volumes/X-T5 --dest /Volumes/SSD/Photos --name Tirana
photosync sync --dest /Volumes/SSD/Photos --name Tirana --one-by-one
```

With `--one-by-one`, after each card photosync asks for the next on
stderr: swap the card and press Enter (it copies the cards it finds
mounted), or type a card's path. `q`, Ctrl-D or Ctrl-C finishes. Without
`--source` it asks for the first card too. All cards go into one session
and one folder; after several, a summary names each sync that was not
complete, and the exit code is the worst of them. A sync that copied a
photo from the other card is "complete, but 1 could not be read on
<card> ...: check that card", and the session's result names that card
too. It asks only when
stdin is a terminal: with stdin redirected (`</dev/null`, a pipe) it
syncs the `--source` cards and ends, so a script never waits; with no
`--source` there it prints "No card was synced." and exits 2.

The X-T5 over a USB cable shows up as a card only in its "USB card reader"
connection mode, not in the default PTP mode. `--source` can be given
several times; the cards are copied at once, each on its own thread (two
sources on one card are read one after the other), into one sync folder,
`<library>/<date>[ - <name>]/<RAW|JPEG|HEIF|VIDEO>/`. A name reuses the
folder that has it, whatever its date; the filesystem decides what is the
same name, so "Tiranë" finds its folder on an exFAT or HFS+ drive, where
macOS lists it decomposed, and "tirana" finds "Tirana" on a
case-insensitive drive. The progress line
counts each card, and a cancel stops them all. Two cards' different photos
of the same name give one copy and one name collision, never an overwrite. A photo on two cards
(backup slots) is copied once; the other card's copy is a "duplicate in
session". If one card cannot be read, the photo is copied from the other,
checked against what was read before: the report names the card it came
from and shows the read error, and its result names the card that could
not be read ("check that card"), complete or not. Only another card (another
device) stands in: the same photo twice on one card, or on two sources that
are one card, is a duplicate of the first, and fails with it. A copy from the other card that is in place but whose
folder flush failed is "in place, flush not confirmed". The library folder must exist. It is never on a card, nor on the
same drive as a source. Photos already anywhere in the library are
skipped. A photo that changed on the card after it was compared with the
library fails, and so does its copy on the other card. A file that reads as
more or fewer bytes than the scan listed fails too, and nothing is placed. A `--name` that
cannot be a folder name is refused before any card is read. Ctrl-C cancels
between chunks and still prints the report; a second Ctrl-C quits at once.
photosync refuses to run as root.

Progress goes to stderr, the report to stdout:

```text
scanned /Volumes/X-T5: 3 photos or videos (24 B), 1 ignored, 0 unreadable, device 16777239
copying 0/3 files, 0 B of 24 B (0%), ETA --
copying 3/3 files, 24 B of 24 B (100%), ETA 0s
Report
Library /Volumes/SSD/Photos
Folder  /Volumes/SSD/Photos/2026-10-28 - Tirana

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
reason, and so does one copied from its twin ("read error ... copied from
its twin ..."); a copy whose folder flush failed shows as "in place, flush not
confirmed". A card with duplicates splits its skipped by reason, and lists
each duplicate with the card it is on the drive from:

```text
  of the skipped, 0 already on drive, 1 duplicate in session
  duplicate in session  100_FUJI/DSCF0001.RAF, on drive from /Volumes/CARD1
```

If anything went wrong on any card, or the library has a line of its own,
the duplicates wait until after all of it, under a line "Duplicates in session
on <card>", so a backup card's long list buries nothing. Files already on the drive are counted, not listed. On a terminal the progress line is cut to its width. A closed
stdout (e.g. `| head`) does not stop the sync: photosync says on stderr that
the report could not be written and keeps the exit code.

Exit codes:

- `0` every photo and video is on the drive (copied, or already there).
- `1` some files failed or were cancelled, or part of a card could not be
  read. Do not format the card; fix the cause and run the same sync again.
- `2` usage error, or the sync was refused before copying anything.

`sources` and `drives` exit 1 if their list cannot be written to stdout.

## Layout

- `crates/photosync-core`: all sync logic, UI-free (INV-8). A `Session`
  runs one sync after another into one library and folder: it scans,
  plans, copies (each card on its own thread) and reports each, removes
  stale temp files once, before any copy starts, knows
  which photos earlier cards put on the drive, and keeps the worst
  outcome (`Session::verdict`). Front-ends drive it through `Watch`.
- `crates/photosync`: the `photosync` binary. `app` is the interactive
  model (screens, keys, what each message changes) and `worker` runs a sync
  on its own thread; no file outside `src/tui/` names ratatui or crossterm
  (`tests/ui_split.rs` checks every file), so a GUI can reuse them with
  `flow` (the session the front-ends share) and `text` (sentences, the
  report). The model keeps one session for the whole run and lends it to
  the worker for each card. `tui` and `tui/view` are the terminal shell
  around them.

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

### Linux (first run, S025)

Linux is built in but not yet verified on a real machine. What is known:

- Cards and drives are looked for under `/media/$USER` and
  `/run/media/$USER` (most desktops auto-mount there). `$USER` must be set.
- A drive whose root folder is owned by root (e.g. a fresh ext4) is not
  offered as a destination until you own it: `sudo chown $USER <mountpoint>`.
- The no-replace rename is `renameat2(RENAME_NOREPLACE)`. On a file system
  that lacks it (EINVAL, e.g. some exFAT drivers), photosync falls back to a
  locked check + rename (DEC-018); any other error fails the copy.
- `scripts/test_exfat.sh` is macOS-only (hdiutil). On Linux, run
  `cargo test --workspace`; the exFAT-only tests say SKIPPED.

Hand check (see `adocs/plan_todo/S025_linux_checks.md`):

```sh
cargo build
cargo test --workspace
./target/debug/photosync sources      # the card, under /media/$USER/...
./target/debug/photosync drives       # the external drive, not the card
mkdir /media/$USER/<DRIVE>/Photos
./target/debug/photosync sync --source /media/$USER/<CARD> \
    --dest /media/$USER/<DRIVE>/Photos --name LinuxCheck; echo "exit $?"
# run it again: expect 0 copied, exit 0
stat -c %d /media/$USER/<CARD> /media/$USER/<DRIVE> /   # three different numbers
./target/debug/photosync              # the TUI
sudo ./target/debug/photosync drives  # must refuse, exit 2
```

For the camera over cable: X-T5 off, MENU > Setup > CONNECTION SETTING >
PC CONNECTION MODE > USB CARD READER, connect a USB-C data cable, switch on,
then repeat the sync. Write down each command's output and exit code.

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
cancelled and a full sync there, then the same card again as the next
card of the session, with scripted keys on ratatui's `TestBackend`.
`tests/exfat_e2e.rs`'s `two_cards_on_two_devices_copy_at_once` syncs two
cards on the twin images (below) to that drive at once, one thread per card.
`tests/exfat_session.rs` runs `sync --one-by-one` over five cards in one
session, one of them failing, typed at its prompt on a pseudo-terminal.
The script also mounts an HFS+ image and runs the sync name tests in
`tests/copy_engine.rs` on it; HFS+, like exFAT, lists a name decomposed.
The binary's unit tests `text::tests::a_session_names_the_card_a_file_was_copied_from_its_twin_for`
and `app::tests::the_result_box_names_the_card_a_file_was_copied_from_its_twin_for`
run a session whose first sync copies a photo from its twin (the planned
file is removed once copying starts), into a folder on that drive. A twin
must be on another device, so those two cards are on two more small images
(`PHOTOSYNC_TEST_TWIN_A`, `PHOTOSYNC_TEST_TWIN_B`). Plain `cargo test` skips these
tests and says so; it covers the binary's help, usage errors and refusals
on temp dirs. The tests reach `photosync sources` and `drives` through a
hidden `--search-root` option.

photosync-core's `copy::threads_tests` (S019) copy several cards at once
in a session; each meets the copy threads at a test hook, so what runs at
the same time is fixed by the test, not by timing. They run under plain
`cargo test`, and on exFAT through the script.

The tests turn on photosync-core's `test-support` feature (through a
dev-dependency of the crate on itself) for a seam that fakes a source's
device. A normal `cargo build` never has it.

```sh
scripts/test_exfat.sh
```
