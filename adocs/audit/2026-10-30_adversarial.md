# Adversarial audit, 2026-10-30, commit 790dc15

Scope: whole repository, code first. Platform: macOS only; nothing Linux was run.

Baseline: `cargo test --workspace` green; `scripts/test_exfat.sh` rc=0.

How measured. Real syncs through `target/debug/photosync` on scratch disk images
(exFAT, HFS+, APFS, FAT32), since detached and deleted. `$S` below is the session
scratchpad. Hand mutants M01-M31 ran on a `git archive` copy in the scratchpad,
never on the tree: one change, then `cargo test --workspace` (the TESTS gate), and
`scripts/test_exfat.sh` for survivors M08, M16 and M29 (not for M02, M03, M13). No hardware card, no
physical unplug, no real read error.

Checked with no finding: real copies into exFAT, HFS+, APFS and FAT32 libraries succeed;
`kill -9` mid-copy then re-run (no partial
file under a final name, stale temp removed, source hashes unchanged, every copied
hash equal); INV-10 refusals through the binary; name collision on the second run
copies nothing and overwrites nothing.

## 2026-10-30_adversarial-F01  Medium  A file that reads shorter than scanned is reported as copied, and the sync as complete

Status: closed

**Evidence**: `crates/photosync-core/src/copy.rs:899-917`: `fill_and_verify` stops
at the first `Ok(0)` and never compares the bytes read with `file.size`. The only
source check is `expected`, and `expected` is `None` for a file unique by name and
size (`plan.rs:248-250`), which is every new photo in a normal sync. The read-back
at `copy.rs:922` compares the copy with what was read, so it agrees with the short
read.

Test run on the scratch copy (scan, truncate the source to 1024 bytes, then plan,
copy, `Report::new`; the test is not in the tree):

```
scanned=3000000 on_drive=1024 complete=true copied_files=1 copied_bytes=3000000
reported complete with 1024 of 3000000 bytes
test result: FAILED. 0 passed; 1 failed
```

The report also counts the scanned 3 MB as copied, not the 1024 bytes on the drive.

**Impact**: The result says the card is safe to format, exit 0, with a truncated
photo on the drive. By code reading, a later sync of the whole file into that folder is a
name collision (sizes differ, so no candidate). Trigger not confirmed: the test shrinks the file by hand.
I could not confirm whether a damaged card or a card driver can return a short read
with no error, or whether a card swapped under one mount path can do it. It is the
one path found where the tool can claim a photo it does not hold.

**Suggested resolution**: Treat a read total that differs from the scanned size as a
failure (`SourceChanged`, or a new reason) before the rename. Add the test above.

## 2026-10-30_adversarial-F02  Medium  A non-ASCII sync name is not reused on HFS+ or exFAT, so one trip splits across folders

Status: planned

**Evidence**: `copy.rs:119-122`: `existing_named` matches the folder name to `--name`
byte for byte. macOS returns these names decomposed on HFS+ and exFAT. Two syncs,
`--name` "Tiranë" in NFC (composed), the new file of the second run dated a month later:

```
AUDHFS  $S/V/AUDHFS/LibU/2026-06-01 - Tiranë/RAW/DSCF1000.RAF
AUDHFS  $S/V/AUDHFS/LibU/2026-07-01 - Tiranë/RAW/DSCF1001.RAF
AUDEX   $S/V/AUDEX/LibU/2026-06-01 - Tiranë/RAW/DSCF1000.RAF
AUDEX   $S/V/AUDEX/LibU/2026-07-01 - Tiranë/RAW/DSCF1001.RAF
AUDAPFS $S/V/AUDAPFS/LibU/2026-06-01 - Tiranë/RAW/DSCF1001.RAF
AUDAPFS $S/V/AUDAPFS/LibU/2026-06-01 - Tiranë/RAW/DSCF1000.RAF
```

Bytes as listed: HFS+ and exFAT `Tirane\xcc\x88` (NFD), APFS `Tiran\xc3\xab` (NFC).
APFS reuses the folder; HFS+ and exFAT do not. Specs Layout: "if a folder
`<any date> - <name>` already exists in the library, it is reused ... so a re-run
after an interruption lands in the same folder". exFAT is the format DEC-018 calls
common.

**Impact**: Any user with an accented name on an exFAT or HFS+ drive gets a second
folder whenever the earliest new file has another date, including a re-run after an
interruption. No photo is lost; the layout promise fails. Not tested on Linux.

**Suggested resolution**: Compare names under one Unicode normalisation. No
normalisation crate is approved (DEPS), so this needs a decision. Add a test on the
exFAT image.

## 2026-10-30_adversarial-F03  Medium  The report never says "duplicate in session" and never names the card a duplicate was copied from

Status: planned

**Evidence**: `crates/photosync/src/text.rs:597-600`: `problem_line` returns `None` for
`DuplicateInSession` and `AlreadySynced`, and no other line renders them. Both the CLI
and the TUI use this text. The text test pins it (`text.rs:1067-1077`, asserted `None`).
A sync of two exFAT cards where the second holds a backup of the first's RAF, and a JPEG
is twice on the first:

```
Source $S/V/AUDCARD2
           copied   skipped   failed   cancelled
  RAW           0         1        0           0
  all           0         1        0           0
```

The output has no "duplicate" and no card name. Specs, Sources: "The report lists the
extra copies as 'duplicate in session', names the card whose file was copied". INV-6:
"Skipped holds 'already on drive' and 'duplicate in session', each with its own reason."
The README says the same. The core has the data (`FileOutcome::DuplicateInSession { of }`);
the only thing rendered is a merged "skipped" count.

**Impact**: A user with backup slots cannot see from the report why a card's file was
skipped, or which card's file is on the drive. A front-end report does not meet INV-6
as worded; only the core data does.

**Suggested resolution**: Show the two kinds apart, and a line or per-card count naming
the card, or reword the specs if counts are enough (a decision).

## 2026-10-30_adversarial-F04  Medium  The TESTS gate never runs a successful sync through the binary; nine tests pass with nothing checked

Status: planned

**Evidence**: `cargo test --workspace -- --nocapture`, each reported `ok`:

```
SKIPPED a_session_names_the_card_a_file_was_copied_from_its_twin_for
SKIPPED closed_stdout_never_panics_and_keeps_the_exit_code
SKIPPED inv10_library_on_a_mounted_card_is_refused_end_to_end
SKIPPED one_card_after_another_keeps_the_worst_outcome_on_exfat
SKIPPED sources_and_drives_list_the_mounted_card_and_drives
SKIPPED sync_from_a_mounted_card_to_a_drive_end_to_end
SKIPPED the_result_box_names_the_card_a_file_was_copied_from_its_twin_for
SKIPPED tui_sync_on_exfat_end_to_end
SKIPPED two_cards_on_two_devices_copy_at_once
```

`crates/photosync/tests/cli.rs:1-3` says it: "no
sync succeeds here". Result of the gap, mutant M29 (CLI exit code taken from the last
sync, not the session's worst): `cargo test --workspace` SURVIVED; only the exFAT script
killed it (`one_card_after_another_keeps_the_worst_outcome_on_exfat`). AGENTS.md TESTS
names only `cargo test --workspace`. A `#[ignore]` would show in the count; these show as passed.

**Impact**: A step can be stamped done, and the suite shown green, with the CLI's copy
path, closed-stdout rule, or "worst outcome" exit code broken. S025 hits this directly:
on Linux, where the script cannot run (macOS only), these tests stay vacuous, so
`cargo test --workspace` passing there proves no successful sync.

**Suggested resolution**: Either a seam or loopback fixture that lets the gate run one real
sync, or a suite that fails rather than passes when the variables are missing outside an
explicit opt-out; and name the script in TESTS. Give S025 a Linux equivalent of the images.

## 2026-10-30_adversarial-F05  Medium  Behaviours the specs name survive their removal: one-card serial read, session re-check, root refusal, flushes

Status: planned

**Evidence**: Hand mutants, `cargo test --workspace`:

```
M16 threads per source, not per device (copy.rs groups): SURVIVED; test_exfat.sh rc 0
M08 session re-check before cleanup removed (session.rs:308): SURVIVED; test_exfat.sh rc 0
M13 main no longer refuses root (main.rs:32): SURVIVED
M02 file flush removed (copy.rs:918, Ok(()) in place of out.sync_all()): SURVIVED
M03 sync_dir no longer flushes (copy.rs:931): SURVIVED
```

- M16: specs Sources "two sources on one card are read one file at a time"; DEC-040 (1).
  No test has two sources on one device copying. `threads_tests.rs:221` asserts it only
  for two cards.
- M08: INV-10 "Copying checks again before it writes or deletes anything". The session's
  check is what guards `remove_stale_temps` (`session.rs:308-314`). `copy_plan`'s own
  check is tested, this one is not.
- M13: only `refuses_user` is tested (`system.rs:156`), not that `main` calls it. Root
  refusal needs a root run: S017 and S025 list it as a hand check, neither done yet.
- M02, M03: `copy.rs:934-936` says "dropping the flush drops the stage too, and the INV-3
  test sees it". That holds only for deleting the whole `synced(...)` call. Replacing the
  flush by `Ok(())` keeps the stage, and nothing notices. For M03 the fault seam
  (`injected`) stays and only the flush goes.

The other mutants were killed: M01, M04-M07, M09-M12, M14, M15, M17-M28, M30, M31.

**Impact**: These lines can regress with a green suite. M16 and M08 cannot be killed by
the exFAT script either. M02 and M03 matter most: INV-3's flush has no check beyond reading
the code.

**Suggested resolution**: Add a Session test with two sources on one device asserting one
file in flight per device; a Session test with the library moved before cleanup; a seam
that records a real flush call on the file and folder. Keep the root check as a hand check,
or factor `main`'s guard so a test can reach it.

## 2026-10-30_adversarial-F06  Low  A sync name too long for a folder is not refused: every file fails, and in the TUI the name is then stuck

Status: planned

**Evidence**: `copy.rs:104-109`: `check_name` rejects only blank, `/` and NUL. A 300-byte
`--name` on APFS:

```
Folder  $S/V/AUDAPFS/Lib3/2026-10-01 - a...
  all           0         0        4           0
  failed      .../DSCF0001.JPG: cannot write .../2026-10-01 - a...
Result: NOT COMPLETE: 4 failed, 0 cancelled, 0 unreadable. Do not format the card. Fix the cause and run the same sync again ...
exit=1
```

Specs TUI: "A name that cannot be a folder name is refused before the session starts and
can be fixed". README: "A `--name` that cannot be a folder name is refused before any card
is read." Exit should be 2. Report line "Folder ..." names a folder that does not exist.
A name with `:` is not refused either; it worked on macOS exFAT here. Linux exFAT, and other
characters, not checked.

**Impact**: The user sees every photo failed and exit 1 for a typo-class input. In the TUI
the session has already started (`app.rs:650-653`), so the name cannot be fixed without
quitting. No data at risk.

**Suggested resolution**: Reject a name whose `<date> - <name>` exceeds 255 bytes, and decide
which characters other filesystems refuse. Then a test.

## 2026-10-30_adversarial-F07  Low  Fixing a refused sync and retrying in one session ends in exit 2 and "NOT COMPLETE: do not format" with every photo on the drive

Status: planned

**Evidence**: `sync --one-by-one` on a pty; the library is created at the prompt and the
same card is given again:

```
The library folder $S/V/AUDEX/LibR does not exist. Create it on your external drive first ...
Result: all 1 photo or video on the drive (1 copied, 0 already there).
  sync 1 ($S/V/AUDCARD2): refused, nothing copied
  sync 2 ($S/V/AUDCARD2): complete
Result: NOT COMPLETE: 1 of 2 syncs not complete: sync 1 ($S/V/AUDCARD2): refused, nothing copied. Do not format the cards of those syncs. Fix the cause and sync them again ...
exit 2
```

Exit 2 is documented as "refused before copying anything" (`args.rs:14-16`, "Nothing was
copied"; README), and here a file was copied. The advice to sync again can never clear it.
Same rule in the TUI, not driven here: `Session::verdict`, with the session made before
the library is checked (`app.rs:650-653`). The TUI's default library `<drive>/Photos`
(`app.rs:553`) is absent on a fresh drive and is never created, so a first TUI sync takes
this path. The README invites the retry: "After a refusal you can go back and try again".

Spec-conformant ("the session's worst"); DEC-034 (2) keeps the refused sync counting and
is not yet confirmed by the user (`status.md`).

**Impact**: A false alarm on the expected first run, and an exit code a script reads as
"nothing copied". Errs on the safe side, but teaches the user to ignore the warning.

**Suggested resolution**: User decision: a refused sync (nothing written) whose cards later
complete in the session stops counting, or the TUI checks the library before making the
session.

## 2026-10-30_adversarial-F08  Low  After a name collision, the result advises a re-run that cannot succeed

Status: planned

**Evidence**: Card with two different `DSCF0001.RAF` (counter wrap), same sync twice, second
output, exit 1 both times:

```
  failed      .../101_FUJI/DSCF0001.RAF: name collision with .../2026-10-01/RAW/DSCF0001.RAF
Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable. Do not format the card. Fix the cause and run the same sync again: files already on the drive are skipped.
```

`text.rs:690-693`. What works is in specs Layout (DEC-013), "The user can re-sync it under
another name": `--name Other` copied it, exit 0. Nothing the tool prints says so. In the TUI
the name stays for the session, so it means quitting.

**Impact**: A user whose counter wrapped, the case DEC-013 exists for, repeats a sync that
fails the same way. Safe but a dead end.

**Suggested resolution**: With a collision in the failures, the result names the way out: sync
again under another name.

## 2026-10-30_adversarial-F09  Low  A second photosync process deletes the first one's live temp file as "left by an interrupted sync"

Status: planned

**Evidence**: `copy.rs:656-684`: every `.photosync-tmp-*` under the library goes, with no check
that its pid is gone. Two processes on one exFAT library, the second started 0.12 s after the
first. Two runs out of two:

```
B: Removed 1 temp file left by an interrupted sync.
A:   failed      $S/bigcard/DCIM/100_FUJI/DSCF3002.MOV: cannot write .../.photosync-tmp-7110-1-DSCF3002.MOV: No such file or directory (os error 2)
A: Result: NOT COMPLETE: 1 failed ...
```

INV-2's exception covers "stale temp files"; this one is live. The temp name holds the pid
(`copy.rs:877`), but it is not used. `copy.rs:397` notes the limit for one process's copies;
the specs say nothing about two processes.

**Impact**: Two terminals, one reader each, one library: spurious failed files and a false
"interrupted" line. The failure is reported, a re-run fixes it, and no final file is touched.

**Suggested resolution**: Skip temps of a live pid, or state in the specs that one library
takes one process at a time.

## 2026-10-30_adversarial-F10  Low  CLI progress shows no bytes per source

Status: planned

**Evidence**: `text.rs:85`, the per-source part is files only:
`copying 0/3 files, 0 B of 15 B (0%), ETA -- [AUDCARD 0/3, AUDCARD2 0/0]`. Specs Progress:
"Shows files and bytes done out of the total, per source and overall." The TUI does it
(`tui/view.rs:255-272`).

**Impact**: A CLI user with two cards cannot see which is slow by bytes. Cosmetic.

**Suggested resolution**: Add bytes, within the line-width cut, or narrow the specs to the TUI.

## 2026-10-30_adversarial-F11  Low  Not reproduced: a bind mount of a non-source card's subfolder looks past the INV-10 card check

Status: planned

**Evidence**: Code reading only. `volume.rs:148-160` walks up from the library to the first
device change, and `plan.rs:396-400` then looks for `DCIM` at that top. With a library at
`/mnt/x`, a bind mount of `<card>/sub`, the top would be `/mnt/x`, with no `DCIM`; the card is
not a source, so the device check does not apply. INV-10: "So no second path to a card, and no
folder on it, gets past the check." I could not confirm this: bind mounts need Linux and root,
and this run had macOS only.

**Impact**: If it holds, writing to a card is possible in a contrived setup. Needs a deliberate
bind mount.

**Suggested resolution**: Try it in S025. If it passes, reword INV-10 or check mount info.

# Verdicts on prior findings (2026-10-28_adversarial)

Re-measured at 790dc15. This report does not edit the earlier one; moving its `Status:` lines
is for whoever plans from here.

| Finding | Verdict | Measured by |
|---|---|---|
| F01 dedup breaks INV-4 | closed | Collision case synced twice on exFAT: second run has no new file, 2 files on the drive, collision failed. Suffix rule gone (DEC-013). |
| F02 rename replaces | closed | M01 (plain rename) killed by 4 tests; M23 (no check in fallback) killed. Real copies succeed on exFAT, HFS+, APFS, FAT32. |
| F03 destination may be a source | closed | Binary refused with exit 2: library on a non-source card, inside the source, holding the source. M10, M11 killed. |
| F04 folder fixed when | closed | DEC-009; ASCII/APFS reuse measured. Reuse is by name only; an unnamed re-run may land elsewhere (`copy.rs:91-99`, by code reading), as specs Layout allows. Non-ASCII gap is new F02. |
| F05 stale temps | closed | `kill -9` left one temp; the re-run printed "Removed 1 temp file", none left. M24 killed. New F09 is about live temps. |
| F06 flush | closed in code; proof weak | `copy.rs:918,920,841`. No test notices removal (new F05: M02, M03). No physical unplug run: S004's stamp says check by hand, and neither S017 nor S025 lists it, so nothing plans that check. |
| F07 twin fallback | closed | Twin tests pass in `test_exfat.sh`; M05, M17, M19 killed. Read errors were injected or simulated, never from a real bad card. |
| F08 INV-1 tests | closed | Manifest before and after in `copy_engine.rs:314`, `progress.rs:283,331`, `session.rs:199`. Not mutated: no mutant wrote to a source. |
| F09 camera over USB unverified | open | Still unverified. S017 (closes it) and S025 are both in `plan_todo/`. |
| F10 scope drive to library | accepted | DEC-010; nothing changed. |
| F11 INV-8 check | closed | M31 (`eprintln!` in core) fails `inv8_ui_free`, exit 101. |
| F12 GUI framework status | accepted | `specs.md:292` agrees with DEC-011. |
