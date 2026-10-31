# Adversarial audit, 2026-10-31, commit 751c4b8

Scope: whole repository, code first, the GUI's first audit. Platform: macOS only; nothing Linux was run.

Result: no High. One Medium, seven Low. No path found that loses or overwrites a photo.

Baseline: `cargo test --workspace` 636 passed, 0 failed, 43 ignored (those run inside the gate's
`ignored_image_tests_pass_on_real_mounts`; every binary's ignored list was read against that code).
`scripts/test_exfat.sh` rc=0, on a throwaway copy of the commit.

How measured. Real syncs through `target/debug/photosync` on scratch images (exFAT, HFS+, FAT32; APFS
image creation failed here, so APFS was the boot disk's own), since detached. `$S` is the session
scratchpad. Two cards and two libraries were force-detached during real copies. Hand mutants ran on a
`git archive` copy, never on the tree: one change, then that crate's tests, and `cargo test --workspace`
if those passed; survivors also went through `scripts/test_exfat.sh`.

Mutants: 47 run, 45 killed, 2 survived (F08). Killed includes all five survivors of the last audit
(M02, M03, M08, M13, M16), M29, and 16 of 16 on the GUI's rules: quit guard, close guard, "Before you
quit", held Start, stale check and look answers, field routing, the drive's mark, banner, exit codes.

Checked with no finding: force-detach of the card mid-copy (rc=1, 61 failed, no temp left, re-run rc=0,
70 of 70 hashes equal); force-detach of the drive mid-copy (rc=1, "Folder none", one temp left, re-run
removed it, rc=0, hashes equal); disk full (rc=1, the two that fit in place, no temp left); zero-byte
file; closed stdout and stderr; no `._*` written on exFAT; accented and other-case `--name` reuse on
exFAT, HFS+, FAT32; a nested source in one sync (reported once each, rc=0).

Not run: a real card, a real window's clicks, Cmd-Q, `scripts/check_linux_from_mac.sh` (network),
`scripts/test_package_mac.sh` (writes to Launch Services' database, outside the repo).

## 2026-10-31_adversarial-F01  Medium  Exit 0 and "all N on the drive" with a picture on the card that was not copied

Status: planned

**Evidence**: A card with `DSCF1000.RAF`, `DSCF1000.JPG` and a `note.TIF`:

```
Source $S/v/ACE
  ...
  ignored 1 file (.TIF 1)
...
Result: all 3 photos or videos on the drive (2 copied, 1 already there).
rc=0
```

`crates/photosync-core/src/report.rs:199-205`: `is_complete`, "the only state in which a card is safe to
format", looks at failed, cancelled and unreadable, never at ignored. `text.rs:1037-1050` `result_line`
does not mention ignored. A card holding only ignored files gives "Result: no photos or videos found.",
exit 0, by the same code. `crates/photosync/src/args.rs:30`: "0 every photo and video is on the drive".
The types are per spec (`specs.md:314-319`), so this is where the spec and the promise of the exit code
disagree: a TIFF is a photo.

Could not confirm: that an X-T5 writes `.TIF` (in-camera RAW conversion) or `.WAV` (voice memo) is from
memory, with no camera here. No decision or step mentions either (`grep -i 'tif\|wav\|voice' adocs/`).

**Impact**: A user reads the green result, or a script reads 0, and formats the card. Any file of a type
not listed goes with it. The only sign is one line of the report, under the result box in the TUI and
GUI. It is the one road found from a "safe" verdict to a lost picture.

**Suggested resolution**: S017 lists what the camera really writes. Then either add those types, or make
the result name ignored files that are not known metadata ("1 file of another type was not copied:
.TIF"), a decision for the user.

## 2026-10-31_adversarial-F02  Low  In a session with an accented name on exFAT or HFS+, a later card's duplicate is not "duplicate in session" and no card is named

Status: planned

**Evidence**: `sync --one-by-one` on a pty, `--name`, two cards holding one photo, second card's report:

```
=== AEX Tiranë
Folder  /v/AEX/LD1/2026-10-31 - Tiranë
Result: all 1 photo or video on the drive (0 copied, 1 already there).
=== AHF Tiranë
Folder  /v/AHF/LD2/2026-10-31 - Tiranë
Result: all 1 photo or video on the drive (0 copied, 1 already there).
=== AEX Tirana
Folder  /v/AEX/LD3/2026-10-31 - Tirana
  of the skipped, 0 already on drive, 1 duplicate in session
  duplicate in session  100/DSCF3000.RAF, on drive from /c1
Result: all 1 photo or video on the drive (0 copied, 1 already there).
```

Cause: `session.rs:423-431` keys `on_drive` by `FileOutcome::Copied { to }`, built from the name as typed
(`copy.rs:114-118`, NFC). The next plan looks it up by the path the library scan listed (`library.rs:69`,
`plan.rs:348`), which these filesystems give decomposed. The bytes differ, so `earlier.get` misses.
Specs Sources (`specs.md:239-241`): "A photo an earlier card of the session put on the drive ... is a
'duplicate in session' on a later card, naming the card". S027 fixed the folder's reuse, S028 the report;
each is right alone, and the two meet here. By code reading the GUI and TUI share it; they were not run on it.

**Impact**: A user with backup cards and an accented sync name, on the drive format DEC-018 calls common,
cannot see which card a photo is on the drive from. The count is right and the verdict safe.

**Suggested resolution**: Key `on_drive` by what the filesystem says is one file, as DEC-044 does for the
folder, or take the folder's spelling from a listing once it is made. A test on the exFAT image.

## 2026-10-31_adversarial-F03  Low  INV-4 fails for an accented file name from a composed source to an exFAT or HFS+ library

Status: planned

**Evidence**: Source a plain folder on APFS holding `photo é.jpg` in NFC. Same sync twice:

```
== AHF
Result: all 1 photo or video on the drive (1 copied, 0 already there).
rc=0
Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable. Do not format the card. After a name collision, ...
rc=1
== AEX
(the same)
```

With `--name other` on HFS+ the "same" photo is copied again, rc=0:
`/AHF/L3/2026-05-01/JPEG/photo é.jpg` and `/AHF/L3/2026-05-01 - other/JPEG/photo é.jpg`.

`plan.rs:354-356` and `library.rs:69` compare file names byte for byte. `specs.md:41-42`, INV-4: "copies
zero files the second time, whatever target folder is picked". `specs.md:357-358`: "same name and size".

**Impact**: Needs a source that keeps NFC, so a plain folder or a Mac-side card, not a camera's ASCII name.
A false "Do not format", and a re-run that never clears it. No photo at risk.

**Suggested resolution**: Decide: compare names as the library's filesystem does (no normalisation crate is
approved, DEPS), or say in specs that INV-4 is for the camera's own names.

## 2026-10-31_adversarial-F04  Low  The shipped GUI overwrites any file a variable or flag names, and takes arguments the specs say it refuses

Status: planned

**Evidence**:

```
$ ls -la keep.RAF    ->  20 bytes
$ PHOTOSYNC_GUI_SEAM_SEARCH_ROOT=$S/emptyroot PHOTOSYNC_GUI_SEAM_SMOKE_PNG=$S/keep.RAF photosync-gui
rc=0
$ ls -la keep.RAF    ->  9362078 bytes; first bytes 8950 4e47 0d0a 1a0a (.PNG....)
```

`crates/photosync-gui/src/png.rs:42-43`: `fs::write`, which replaces. `args.rs:33-42` takes `--search-root`
and `--smoke-png` with no `cfg`, while its refusal says "photosync-gui takes no arguments." `specs.md:181`: "An
argument starting `-psn_` is ignored ...; any other is refused with exit 2", and that the variables "only narrow
the search or ask for a picture". The root replaces the search, as `lib.rs:39-46` shows, so it can
also widen it; "cannot make a plain folder a card" held on reading (`volume.rs:53`).

**Impact**: Takes a deliberate variable or flag. Then a photo on the drive can be replaced by a PNG, the one write
in the shipped binary that the prime directive's rename rule does not cover, and the window quits by itself.

**Suggested resolution**: `create_new` for the picture, so it never replaces a file; put both flags in the specs'
sentence.

## 2026-10-31_adversarial-F05  Low  Copies do not keep the camera's modification time, and nothing says so

Status: planned

**Evidence**:

```
2026-06-01 12:00:00 v/ACE/DCIM/100_FUJI/DSCF1000.RAF
2026-10-31 18:06:25 v/AEX/Lib/2026-06-01 - Tiranë/RAW/DSCF1000.RAF
```

`copy.rs:998-1002` creates the temp and nothing sets its times; `grep -rn 'set_times\|set_modified\|futimens' crates`
finds only test helpers. No entry in `decisions.md`, `specs.md` or the README takes this up. `specs.md:298`: "The
capture date is the file's modification time as the camera wrote it".

**Impact**: Once the card is formatted the one date the app trusts is gone from the file system. Sorting by date in
Finder shows the sync's day. EXIF inside a photo stays; the bytes are safe. An unrecorded behaviour more than a broken one.

**Suggested resolution**: A decision. If kept, `File::set_modified` before the flush, tolerating a failure, and a test.

## 2026-10-31_adversarial-F06  Low  A `DCIM` symlink gets a library past INV-10: on the source's drive, or even the source itself

Status: planned

**Evidence**: `$S/sd/DCIM -> $S/v/AEX/shots`, library `$S/v/AEX/L7`, same drive:

```
Result: all 1 photo or video on the drive (1 copied, 0 already there).
rc=0
16777237 v/AEX/shots/100/q.JPG
16777237 v/AEX/L7/2026-10-31/JPEG/q.JPG
16777231 sd
```

With `$S/sdl/DCIM -> $S/v/AEX/L8` and library `$S/v/AEX/L8`, the library's own files are the source: rc=0, "all 1
... already there". `source.rs:140` follows `DCIM` by design, but `source.rs:144-148` and `plan.rs:400-418` check only
the root's path and device. `specs.md:77-84`: "So no second path to a card, and no folder on it, gets past the check".
A real card behind such a symlink is still refused, by the DCIM at its top, by code reading (`plan.rs:422-433`).

**Impact**: Needs a hand-made symlink on a plain folder; FAT cards cannot hold one. A drive failure then takes source
and copy together, which is what the rule is for. In the second form, nothing is written.

**Suggested resolution**: Resolve `DCIM` too, and check its path and device as a root's.

## 2026-10-31_adversarial-F07  Low  Small report and name defects: "already there" counts duplicates; a long name that fits never copies

Status: planned

**Evidence**:

(a) First sync, empty library, RAF on two cards: the table says the opposite of the result, whose total merges both
reasons (`text.rs:1044-1046`, also `text.rs:522-527`):

```
  of the skipped, 0 already on drive, 1 duplicate in session
Result: all 3 photos or videos on the drive (2 copied, 1 already there).
```

(b) A file of 244 bytes, whose final name fits (`cp` to that library worked): its temp name does not
(`copy.rs:1048-1054`), on every run:

```
failed  ln/DCIM/a….JPG: cannot write .../.photosync-tmp-45975-0-a….JPG: File name too long (os error 63)
rc=1
```

**Impact**: (a) INV-6 wants each reason apart, and the last line is what `| tail` shows. (b) Plain-folder sources only:
a camera's names are short. Reported, and safe.

**Suggested resolution**: (a) Say "already there or duplicates", or the two counts. (b) A short temp name, kept
readable by its number.

## 2026-10-31_adversarial-F08  Low  Two rules survive their removal: a temp-named symlink or folder, and a lowercase `dcim`

Status: planned

**Evidence**: Hand mutants, `cargo test --workspace` and `scripts/test_exfat.sh` (rc=0, 22 result lines ok, both applied):

```
N38 copy.rs:796 `if temp && file_type.is_file()` -> `if temp`: SURVIVED
N34 plan.rs:392 `n.eq_ignore_ascii_case("DCIM")` -> `n == "DCIM"`: SURVIVED
```

N38: `copy_engine.rs:573` tests a folder named `.photosync-tmp-dir`, which holds no pid, so `stale_temp` already
says no, whatever the type. No test has a folder or symlink named as a stale temp. The code itself holds: a symlink
`.photosync-tmp-99999-0-x.RAF` stayed after a real sync, rc=0.

N34: `library_plan.rs:227` means to test `drive/dcim/lib`, but makes `drive/DCIM` first, on a filesystem that ignores
case here, and `fs::canonicalize` gives back `DCIM`. So the case its comment names ("cards ignore case") never runs on
macOS. A real `dcim` folder made first would.

**Impact**: Both lines can regress with a green gate. N38 is the edge of INV-2's one exception: with it gone, a
symlink, which is not photosync's, is deleted.

**Suggested resolution**: A stale-temp name on a folder and on a symlink, with a live target, kept by the cleanup; make
`dcim` before `DCIM`, in its own temp folder.

# Verdicts on prior findings

Re-measured at 751c4b8. This report does not edit the earlier ones; moving their `Status:` lines is for whoever plans
from here. All of 2026-10-30's still say `planned`, and S026 to S035 are done.

## 2026-10-30_adversarial

| Finding | Verdict | Measured by |
|---|---|---|
| F01 short read reported complete | closed | `copy.rs:1093`; N01 (check removed) killed by 4+ tests. |
| F02 accented name splits folders | closed | Rerun, later date, name given in NFD and lower case: one folder on exFAT, HFS+, FAT32. New F02 is what is left near it. |
| F03 report hides duplicates | closed | Real two-card run prints "duplicate in session ... on drive from". Not with an accented name: new F02. |
| F04 gate runs no real sync | closed on macOS | 43 ignored run inside the gate, count checked in `images.rs:386-393`; M29 now fails the gate. On Linux they stay ignored: S025, as the README says. |
| F05 behaviours survive removal | closed | M02, M03, M08, M13, M16 all killed, plus M02b (`sync_all` removed from `flush`). |
| F06 long name not refused | closed | 300-byte `--name`: refusal, rc=2. |
| F07 fixed refusal stays exit 2 | closed | pty run: library made at the prompt, same card: "its cards completed later", rc 0. |
| F08 re-run advice dead end | closed | Result now says start again under another name. |
| F09 live temp deleted | closed | N02 killed; `a_second_process_leaves_the_live_temp_of_the_first` in the gate. Not re-raced by hand. |
| F10 no CLI bytes per source | closed | `[c1 1/1, 2.0 kB of 2.0 kB; ACE 2/2, 8.0 kB of 8.0 kB]`. |
| F11 bind mount past INV-10 | open | Still not reproduced: needs Linux and root. S025 closes it; `volume.rs:148-160` unchanged. |

## 2026-10-28_adversarial

| Finding | Verdict | Measured by |
|---|---|---|
| F06 flush, "proof weak" | closed, proof now real | Flush mutants killed; force-detach runs above. No physical unplug: optional in S017. |
| F09 camera over USB unverified | open | No camera here. S017 waits for the user. |
| F01-F05, F07, F08, F11 | closed | No regression seen: re-runs, kill of M16, N05, N15, N17, N36 and the INV-1 tests in the gate. Real read errors still only injected or simulated. |
| F10, F12 | accepted | Nothing changed. |
