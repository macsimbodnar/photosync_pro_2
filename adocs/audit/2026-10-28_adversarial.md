# Adversarial audit — plan and goals @ 4341842 (2026-10-28)


Scope: everything under `adocs/` plus `AGENTS.md`, at commit 4341842. No code
exists. Measured against the user's original request and follow-up answers
(quoted to the auditor in the conversation, not tracked in the repository).
Prior reports: none (`adocs/audit/` held only `.gitkeep`), so there are no
verdicts on earlier findings.

Reproductions use scratch files outside the repository:
`dedup_model.py` (a model of the specs' dedup and collision rules) and
`rn.rs` (a std::fs::rename probe), both in the session scratchpad. The Rust
toolchain is at `~/.cargo/bin` and is not on the default PATH.

## 2026-10-28_adversarial-F01  High  Dedup by camera name breaks INV-4 for every file renamed on collision

Status: open

**Evidence**: `adocs/specs.md` Layout: "If the target folder already holds a
different file with the same name (the counter wraps at `DSCF9999`), the new
file gets a suffix, `DSCF1234_1.RAF`". Already-synced detection: "already
synced if any library file anywhere has the same name and size and the same
content (hash). Name + size picks the candidates". A suffixed file never has
the source's name, so it is never a candidate. A model of exactly these two
rules, with one card holding two different same-size `DSCF0001.JPG` (counter
wrap, `100_FUJI` and `101_FUJI`) synced again and again into the same folder:

```
$ python3 -I dedup_model.py
run 1 same folder: copied=2
run 2 same folder: copied=1
run 3 same folder: copied=1
run 4 same folder: copied=1
['DSCF0001.JPG', 'DSCF0001_1.JPG', 'DSCF0001_2.JPG', 'DSCF0001_3.JPG', 'DSCF0001_4.JPG']
```

The same happens when a later sync reuses an existing `<date> - <name>`
folder. With a different target folder the second run still copies one extra
duplicate. INV-4 says "copies zero files the second time, whatever target
folder is picked". S004's accepts ("running the same sync twice copies zero
files the second time") does not name the collision case, so a test of it
can pass while the defect stays.

**Impact**: Any user whose counter has wrapped, or who syncs twice under one
name on one date, gets a duplicate on every sync, without end if the folder
is reused. The skipped count is wrong and the drive fills up. The specs name
counter wrap as the reason the suffix rule exists, so this is the expected
case, not an edge case.

**Suggested resolution**: Match candidates by size (or size plus the name with
any `_<n>` suffix removed), with the hash deciding. Add to S004/S003 accepts a
test that syncs a source with a collision twice and checks zero copies on the
second run.

## 2026-10-28_adversarial-F02  High  "Renamed into place" replaces an existing file; no-overwrite rename is not required, planned, or tested

Status: planned

**Evidence**: INV-3: "renamed into place only after that check". The Layout
collision rule is a check followed by a later rename. On Unix, Rust's
std::fs::rename silently replaces the target:

```
$ ~/.cargo/bin/rustc --edition 2021 -o rn rn.rs && ./rn d
rename result: Ok(())
exists_at_check=true final now = "new verified copy"
```

S010 copies two sources at the same time into the same `<TYPE>` folders, so
two workers can both see a name as free and both rename onto it. The second
rename then destroys the first file. Nothing in S004 or S010 accepts asks for
a rename that refuses to replace a file, or for a test of two writers on one
name. The OS primitives that refuse to replace (`renamex_np(RENAME_EXCL)` on
macOS, `renameat2(RENAME_NOREPLACE)` on Linux) need a `libc`-style crate,
which DEC-006 does not approve. The std-only fallback,
`hard_link`+`remove`, needs hard links. I believe exFAT and FAT32 external
drives do not support them, but I did not confirm this here.

**Impact**: The prime directive ("no file already on the destination drive is
ever overwritten") and INV-2 rest on a check-then-act race. This hits
concurrent multi-source syncs, and any file that appears between the check
and the rename.

**Suggested resolution**: Make INV-3 say the rename must never replace an
existing file. Choose the mechanism as a decision, including the dependency
question under DEPS. Add S004/S010 accepts: a test where the final name
appears before the rename, and the existing file must survive.

## 2026-10-28_adversarial-F03  High  Nothing stops the destination from being a source, or the library from sitting inside one

Status: planned

**Evidence**: S007 lists "destination drives" and sources from the same mount
roots (`/Volumes`, `/media/$USER`, `/run/media/$USER`). No line in
`specs.md` Destination or any step's accepts rejects a library on a source
volume, a library inside a `DCIM` tree, or a source volume that is also the
destination. An external drive that holds a `DCIM` folder (common on backup
drives) is listed as a source and as a destination.

**Impact**: If the user picks the SD card (or the camera in card-reader mode)
as the destination, the sync writes to the source, which breaks INV-1 and the
prime directive. If the library sits inside the scanned DCIM tree, the sync
scans its own copies, which can feed back on later runs.

**Suggested resolution**: Add a spec rule and an invariant test: the core
refuses a library on the same volume as any selected source, and refuses
overlapping source and library paths. Put it in S004 or S007 accepts.

## 2026-10-28_adversarial-F04  Medium  "One folder per sync, dated by earliest new file" cannot hold for one-by-one sessions or re-runs after interruption

Status: planned

**Evidence**: Layout: "One sync puts all its new files in one folder" and
"`<date>` is ... the earliest capture date among the new files". Sources:
"One source after another ... The session stays open between cards". Card 1
is copied before card 2 is inserted. If card 2 holds an earlier photo, the
date is already used in card 1's folder name. Either card 2 goes to a second
folder (breaks "one folder"), or card 1's folder must be renamed, which the
specs never allow. INV-7: "Re-running the sync finishes the job". On a re-run
the new files are only the ones not yet copied, so their earliest date can be
later, and the rest of the job lands in a different `<date> - <name>` folder.
User: "The whole new data ... will go under same name." S004 and S010 accepts
do not cover either case.

**Impact**: One trip's photos end up split across folders, with no rule for
which folder wins. Users of a single reader and users whose sync was
interrupted get different layouts from the same cards.

**Suggested resolution**: Decide (user decision) when the session folder name
is fixed. Options: at session start, from the first source; or from the
existing folder with the same name. Record it, update Layout, and add tests
for the one-by-one and interrupted re-run cases.

## 2026-10-28_adversarial-F05  Medium  Temp files left by an interruption cannot be cleaned up under INV-2, and nothing plans for them

Status: planned

**Evidence**: INV-3 writes copies "under a temporary name". INV-7 only
requires "no partial file under a final name". INV-2: "Every file present on
the destination before a sync is still there afterwards". A leftover temp
file from an earlier run is such a file, so the next run may not delete it.
INV-9 allows on the drive only "the photo files under the library folder", and
a temp file is not one. The specs do not say how the library scan treats temp
files, and no step (S004, S005) mentions cleanup of stale temp files. S005
covers only the current run's temp file ("removes the temp file").

**Impact**: Every kill, unplug, or crash leaves a file of up to several GB
(X-T5 video) that the tool is forbidden to remove, so these files pile up
without limit. Whether the scan ignores them depends on a temp naming scheme
that nobody has specified.

**Suggested resolution**: Specify the temp naming scheme. Specify that the
scan ignores temp files. Carve an explicit exception into INV-2 for the
tool's own temp files, which the tool may delete. Add a test to S004 accepts.

## 2026-10-28_adversarial-F06  Medium  Verify-then-rename without fsync does not make a copy safe from an unplug

Status: planned

**Evidence**: INV-3 checks "the destination copy's hash", and INV-7 covers
"unplugging a device". Neither the specs nor S004 requires the data to be
flushed (fsync of the file, then of the directory after rename). Without a
flush, the read-back hash can come from the OS page cache rather than the
disk, and after an unplug or power loss a file under its final name can be
truncated or empty. S004's "simulated interruption" test cannot catch this.
Not reproduced: it needs physical unplugging of a drive.

**Impact**: The tool reports a file as copied and verified when it is not on
disk. On the next run that file is a name+size candidate, and if it is
truncated the hash fails and the file is re-copied. If its size is right but
its content is zero-filled, nothing repairs it unless the hash catches it,
and the user may already have formatted the card.

**Suggested resolution**: Make INV-3 require sync_all on the temp file
before the hash read-back and rename, and a directory sync after the rename.
Note in S004 accepts that the unplug case is checked by hand.

## 2026-10-28_adversarial-F07  Medium  "Copied once" across sources defeats backup mode when the chosen copy fails

Status: planned

**Evidence**: Sources: "backup (identical cards) ... A file found on two
sources in one session is copied once". S003: "a file present on two sources
in one session is planned once". The specs do not say what happens if the
read from the chosen source fails. They also do not say which INV-6 category
the second source's copy of the file falls into ("skipped (already on
drive)" is false when the file was not on the drive at planning time).

**Impact**: Backup mode exists for the case where one card goes bad. As
planned, a read error on the chosen card reports the file as failed while the
good copy on the other card is never tried. The user may then format both
cards, believing the report.

**Suggested resolution**: Specify a fall-back to the other source on failure,
and define the INV-6 category for duplicates within a session. Test both in
S010.

## 2026-10-28_adversarial-F08  Medium  INV-1 (prime directive, source side) is tested only for the scan, not for copy, cancel, or sessions

Status: planned

**Evidence**: Only S002 accepts names INV-1 ("the scan leaves the source
manifest unchanged"). S004 lists "INV-2, INV-3, INV-4, INV-5 and INV-7", and
S005 and S010 do not mention INV-1. Those are the steps that open source
files for copying, cancel mid-file, and handle several sources. INV-1's
manifest is "hash and mtime" of the source. As worded, it does not say that
files added to the source (for example a temp or lock file) or removed from
it are caught.

**Impact**: The steps most likely to touch the source can be marked done
with no check of the prime directive.

**Suggested resolution**: Add an INV-1 manifest check (the file list plus
hash plus mtime) to the S004, S005 and S010 accepts. Reword INV-1 so the
manifest includes the set of paths.

## 2026-10-28_adversarial-F09  Medium  The camera-over-USB goal is never verified by any step

Status: planned

**Evidence**: User: "or directly by the cable from the fujifilm". The specs
say the X-T5 in "USB card reader" mode is covered because it mounts as a
volume. `research.md` sources that claim to "user reports". No step's accepts
checks it: S007's manual check is "with a real card on Mac", and S009's is "a
full sync from a real card". Not confirmed by me: whether the X-T5's card
reader mode presents a mass-storage volume on macOS, or an MTP/PTP device.

**Impact**: A goal the user asked for can ship unverified. If the mode does
not mount, the fallback the user accepted ("sd card reader only") applies,
but nobody will have told the user.

**Suggested resolution**: Add to S007 accepts a hand check with the X-T5 on a
cable in card-reader mode, with the result recorded in the stamp. If it does
not mount, record a decision.

## 2026-10-28_adversarial-F10  Low  Dedup scope narrowed from "the hard drive" to "the library" without a recorded user decision

Status: accepted

**Evidence**: Request: "The sync should scan the hard drive and avoid syncing
photos that were already synced". The specs scan "the whole library", and
INV-9 forbids reading anything on the drive outside the library folder.
DEC-003 records RAM-only state, but not the narrowing from drive to library.

**Impact**: Photos imported earlier into another folder on the same drive are
copied again. The user may expect them to be skipped.

**Suggested resolution**: Ask the user, then record the answer as a decision
(or widen the scan root).

## 2026-10-28_adversarial-F11  Low  INV-8's "never writes to stdout or stderr" has no checkable test planned

Status: planned

**Evidence**: S001 checks only for a dependency on
"clap/ratatui/crossterm/iced", not egui (proposed in DEC-002) or logging
crates. S005's "the core prints nothing" names no way to check it. Output
captured by the cargo test harness is not asserted on, so a test of this
would pass even with a `println!` in the core.

**Impact**: INV-8, the user's "no spaghetti between UI and core" rule, is not
enforced. An invariant with no check is, by the specs' own words, "a wish".

**Suggested resolution**: Add a suite test that fails on
`print!`/`println!`/`eprint!`/`eprintln!`/`dbg!` and on `std::io::stdout`/`stderr`
in the core's sources. Turn the dependency check into an allow-list rather
than a deny-list.

## 2026-10-28_adversarial-F12  Low  GUI framework status is inconsistent across specs, decisions and plan

Status: accepted

**Evidence**: `specs.md` Open items: "Native GUI framework: iced is proposed,
not chosen (DEC-002)". DEC-006: "user approved ... iced (GUI, later)".
`plan.md`: "GUI (iced)". DEC-002 still reads "iced or egui".

**Impact**: The specs, which take precedence over plan and status, contradict
a later user decision. A future agent may reopen a question the user has
already settled, or take iced as settled when the user only approved it as a
dependency.

**Suggested resolution**: Ask the user whether approving iced also chose it,
then align the specs Open items and DEC-002 with the answer.
