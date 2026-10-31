# Decisions

Ids stable, never reused, newest entry last, index on top. Entries hold
current reasoning and may be compacted or superseded by rewrite — git keeps
every earlier version. Record a decision before or alongside the change it
covers, never after. Decisions belong to the user; agents propose.

Entry format:

```
## DEC-001  YYYY-MM-DD  short title
Tags: topic, topic
Decision: what was chosen, and by whom. Rejected options and why, when they
were live.
Why: one line.
```

## Index

- DEC-001 2026-10-28 — adopt moltke workflow and project rules
- DEC-002 2026-10-28 — plain Rust, core library + CLI/TUI first, native GUI later
- DEC-003 2026-10-28 — no index file; scan the library into memory each sync
- DEC-004 2026-10-28 — layout, camera names, any slot setup, mounted volumes only
- DEC-005 2026-10-28 — one folder per sync, dated by file timestamp
- DEC-006 2026-10-28 — dependencies approved: clap, ratatui, crossterm, blake3, iced
- DEC-007 2026-10-28 — TESTS becomes gating on cargo test
- DEC-008 2026-10-28 — libc approved for no-replace rename
- DEC-009 2026-10-28 — sync folder fixed at session start, reused by name
- DEC-010 2026-10-28 — already-synced scan covers the library folder only
- DEC-011 2026-10-28 — iced: approved dependency, tentative GUI choice
- DEC-012 2026-10-28 — copy verification may read from the OS cache
- DEC-013 2026-10-28 — name collision fails the file; never rename
- DEC-014 2026-10-28 — dot-files and dot-folders on a source are ignored
- DEC-015 2026-10-28 — unreadable library candidate: copy, do not skip
- DEC-016 2026-10-28 — trash and hidden library entries never count as synced
- DEC-017 2026-10-28 — copy engine details left open by the specs
- DEC-018 2026-10-28 — exFAT: locked check + rename when no-replace rename is unsupported
- DEC-019 2026-10-28 — exFAT fallback details left open by DEC-018
- DEC-020 2026-10-28 — copy hardening details left open by S013
- DEC-021 2026-10-28 — progress and cancel details left open by S005
- DEC-022 2026-10-28 — sync report details left open by S006
- DEC-023 2026-10-28 — report hardening details left open by S014
- DEC-024 2026-10-28 — volume discovery and the INV-10 device check (S007)
- DEC-025 2026-10-28 — strict INV-10: library never on a source's disk or on any card
- DEC-026 2026-10-28 — strict INV-10 details left open by DEC-025 (S015)
- DEC-027 2026-10-28 — INV-10 enforcement: read-only device, test-support seam, re-check before copy (S016)
- DEC-028 2026-10-28 — CLI details left open by S008
- DEC-029 2026-10-28 — hardware hand checks deferred to S017
- DEC-030 2026-10-28 — TUI details left open by S009
- DEC-031 2026-10-28 — TUI hardening details left open by S018
- DEC-032 2026-10-28 — S010 first part: core Session API, and the split of S010
- DEC-033 2026-10-28 — S010 split accepted; worst outcome wins; F07 test change
- DEC-034 2026-10-28 — one card after another in the CLI and TUI (S020)
- DEC-035 2026-10-28 — S022: seam for a refused report, prompt poll, cancel during lookup
- DEC-036 2026-10-28 — twin fallback on a read error (S021, F07)
- DEC-037 2026-10-28 — twin-fallback reporting fixes (S023)
- DEC-038 2026-10-28 — a twin must be on another device, not only another source
- DEC-039 2026-10-28 — S024: device rule in planning, one-sync result names unread cards, twin images
- DEC-040 2026-10-29 — S019: several cards copied at once, one thread per card
- DEC-041 2026-10-29 — the TUI shows the app as "phoosync pro 2"
- DEC-042 2026-10-30 — a file that reads another length than scanned fails (S026)
- DEC-043 2026-10-30 — how the report shows duplicates and names the card (S028)
- DEC-044 2026-10-30 — a sync name is matched by the filesystem's lookup, not by bytes (S027)
- DEC-045 2026-10-30 — a later copy of an earlier sync's photo is resolved in the plan (S038)
- DEC-046 2026-10-30 — duplicates wait until after every failure and the library (S039)
- DEC-047 2026-10-30 — how tests see a real flush; root guard outside main (S030)
- DEC-048 2026-10-30 — the end of a report counts the problem lines above the duplicates (S040)
- DEC-049 2026-10-30 — the gate mounts its own images and runs the real syncs (S029)
- DEC-050 2026-10-30 — a refused sync whose cards completed later stops counting; the TUI checks the library first (S032)
- DEC-051 2026-10-30 — the images' watchdog has time limits and says when it gives up (S041)
- DEC-052 2026-10-30 — TUI: cards changed after a refusal before the session are not remembered
- DEC-053 2026-10-30 — which sync names are refused, the same on every platform; the Folder line (S031)
- DEC-054 2026-10-30 — stale temps: the pid in the name decides (S034)
- DEC-055 2026-10-30 — the exFAT fallback holds a kernel lock on the folder, across processes (S043)
- DEC-056 2026-10-30 — one sentence for the way out of a name collision; too long in any spelling has its own sentence (S033, S035)
- DEC-057 2026-10-30 — a cancel ends the wait for the folder's lock; one timeout fails the folder for the run (S044)
- DEC-058 2026-10-30 — the CLI's line: each source's bytes in its bracket, dropped whole by the cut; the TUI's line stays (S035)
- DEC-059 2026-10-30 — the harness stops a process before it kills its tree, and kills it last (S045)
- DEC-060 2026-10-31 — the closed-stdout test proves its pipe has no reader; the binary was not at fault (S046)
- DEC-061 2026-10-31 — the TUI's report wraps by itself and scrolls by rows; the result's box grows (S037)
- DEC-062 2026-10-31 — the TUI's other screens wrap and scroll as the report does; the copying screen's gauge goes on top (S047)
- DEC-063 2026-10-31 — the TUI keeps the report's wrapped rows in the view, keyed by the outcome's number and the body's area (S048)
- DEC-064 2026-10-31 — notes leave the list what it must show: the row under the cursor, or the empty list's sentence, boxless when too low (S049)
- DEC-065 2026-10-31 — the GUI is iced 0.14.0, software drawing only, its own font, one theme (S036)
- DEC-066 2026-10-31 — how a GUI screen is checked with no window and nobody looking; iced_test is allowed by the user (S036)
- DEC-067 2026-10-31 — the GUI is its own crate; the shared layer gets a library target and moves nowhere (S036)
- DEC-068 2026-10-31 — the GUI drives the model by its keys, and hears the worker by waking and draining (S036)
- DEC-069 2026-10-31 — what stands in for exit codes; closing, quitting and root in the GUI; its name (S036)
- DEC-070 2026-10-31 — packaging: an ad hoc signed .app and an AppImage, each its own step (S036)
- DEC-071 2026-10-31 — what must stay in sight in the GUI: the fixed part, the not-safe banner, boxes that cannot outgrow the window (S036, plan review)
- DEC-072 2026-10-31 — how the GUI's tests stay honest: the window test apart, rows the source check can read, a cancel that cannot miss (S036, plan review)
- DEC-073 2026-10-31 — the GUI's scroll starts at the top of each screen; looking for volumes is a state of its own (S036, plan review)
- DEC-074 2026-10-31 — small choices of the GUI's first step: screens to come, arguments, smoke timing, harness picture (S051)
- DEC-075 2026-10-31 — small choices of the cards and drive screens: stale clicks, scroll reset, the drive's mark, a box's end (S052)

## DEC-001  2026-10-28  adopt moltke workflow and project rules
Tags: workflow, rules
Decision: user adopted moltke 1.2.2 with defaults for every rule: GIT commit
freely, never push; AGENTS free; PLAN one active step; DOCS README; REVIEW
fast check per step; AUDIT on demand; DEPS ask first; COMMITS per step and
plan change. TESTS gating default chosen, but no suite exists yet, so the
`none` line is recorded; completion is a claim until a suite lands and TESTS
is changed via /moltke:rules. Repo created with git init; .moltke.local.md
declined.
Why: start with conservative defaults; tighten TESTS once code exists.

## DEC-002  2026-10-28  plain Rust, core library + CLI/TUI first, native GUI later
Tags: technology, architecture
Decision: user chose plain Rust with a native Rust UI. Tauri, Electron, and
any web view were rejected by the user. Start with CLI + TUI over a UI-free
core library (INV-8), and keep a GUI possible later. The UI will be reworked
once the tool works, so the core/UI split matters more than UI polish.
Frameworks are not chosen yet: proposed ratatui (TUI), clap (CLI), iced or
egui (GUI).
Why: one codebase that is testable without a UI, with swappable front-ends.

## DEC-003  2026-10-28  no index file; scan the library into memory each sync
Tags: dedup, security
Decision: user chose to keep the already-synced state in RAM only, rebuilt
from the library on every sync. No catalog on the drive or the card (INV-9).
An index file may come back later as an optimisation.
Why: no dependence on external state files, and so more secure; the cost
is a slower library scan.

## DEC-004  2026-10-28  layout, camera names, any slot setup, mounted volumes only
Tags: layout, sources
Decision: user chose `<library>/<date>[ - <name>]/<RAW|JPEG|HEIF|VIDEO>/`,
keeping the camera's file names. Any card-slot setup works, because each file
goes by its own type. Sources are the mounted volumes with a `DCIM` folder,
so the camera works in "USB card reader" mode. PTP is out for now.
Why: simplest logic that covers every slot setup and the cable without PTP.

## DEC-005  2026-10-28  one folder per sync, dated by file timestamp
Tags: layout
Decision: user chose that all new files of one sync go into one folder,
`<date>[ - <name>]`. Rejected for now: one folder per capture day. Naming a
selection may come later. The capture date is the file timestamp the camera
wrote, not EXIF. Agent picked the earliest new-file date as `<date>` (the
user said "date", without saying which); the user may override.
Why: simplest model; no EXIF dependency.

## DEC-006  2026-10-28  dependencies approved: clap, ratatui, crossterm, blake3, iced
Tags: dependencies
Decision: user approved clap (CLI), ratatui + crossterm (TUI), blake3 (copy
verification and dedup hash), and iced (GUI, later). Transitive dependencies
come with them. Any other crate still needs asking (DEPS).
Why: each replaces a lot of hand-written code; a byte compare would read the
card twice.

## DEC-007  2026-10-28  TESTS becomes gating on cargo test
Tags: rules, tests
Decision: user changed TESTS from `none` (DEC-001) to: the suite is green
before a step is done (`cargo test --workspace`), and a defect gets a
failing test first.
Why: a Rust workspace now exists in the plan, so completion can be checked.

## DEC-008  2026-10-28  libc approved for no-replace rename
Tags: dependencies, safety
Decision: user approved the `libc` crate, to rename without replacing
(`renamex_np(RENAME_EXCL)` on Mac, `renameat2(RENAME_NOREPLACE)` on Linux).
Renames are also serialised per destination inside the app. Rejected: a
check-then-rename under a lock alone, which does not protect against other
programs. Closes the plan side of audit 2026-10-28_adversarial-F02.
Why: `std::fs::rename` silently replaces an existing file on Unix (INV-2).

## DEC-009  2026-10-28  sync folder fixed at session start, reused by name
Tags: layout
Decision: user chose that the folder is computed once, from the first plan
of the session, and reused by later cards. A named sync reuses an existing
`<any date> - <name>` folder. Rejected: the sync date (a re-run on another
day splits the folder), and asking every time. Refines DEC-005. For audit
2026-10-28_adversarial-F04.
Why: one session and its re-runs stay in one folder.

## DEC-010  2026-10-28  already-synced scan covers the library folder only
Tags: dedup, scope
Decision: user chose to scan only the library folder, not the whole drive.
Rejected: the whole drive (slower, and it matches unrelated copies). Accepts
audit 2026-10-28_adversarial-F10.
Why: the library is where photosync writes, so it is what "already synced"
means.

## DEC-011  2026-10-28  iced: approved dependency, tentative GUI choice
Tags: gui, dependencies
Decision: iced is approved as a dependency (DEC-006) and is the tentative
GUI framework. It is confirmed when the GUI is planned, so the specs, plan
and decisions now say the same. Accepts audit 2026-10-28_adversarial-F12.
Confirmed as the agent's choice by DEC-065.
Why: the docs disagreed; no GUI work is due yet.

## DEC-012  2026-10-28  copy verification may read from the OS cache
Tags: safety
Decision: agent proposed, user accepted with the triage. Copies are
fsync'ed (file and folder) before verifying (S004). The verifying read may
still come from the OS page cache. Bypassing the cache is not portable, so
this is a known limit, not a defect. For audit 2026-10-28_adversarial-F06.
Why: fsync covers the real unplug risk; a cache bypass costs portability for
little gain.

## DEC-013  2026-10-28  name collision fails the file; never rename
Tags: layout, dedup
Decision: user chose that a different file with the same name already in
the target folder is not copied; it is reported as failed "name
collision". Rejected: an `_N` suffix (breaks dedup by name, audit
2026-10-28_adversarial-F01, and changes camera names), and dedup by content
only (on uncompressed RAF, hashes the whole library every sync). For
F01, planned in S004.
Why: camera names stay untouched, and dedup by name + size + hash stays
correct.

## DEC-014  2026-10-28  dot-files and dot-folders on a source are ignored
Tags: sources, layout
Decision: user chose to ignore every entry whose name starts with `.`. Such
entries are counted as ignored, and dot folders are not walked. Rejected:
ignoring only `._*` (too narrow), and copying them (the S002 fast check
found that `._DSCF0001.RAF` would be classed as RAW and copied).
Why: these are OS metadata, never photos.

## DEC-015  2026-10-28  unreadable library candidate: copy, do not skip
Tags: dedup, failures
Decision: agent's pick in S003; the user confirmed it ("pick the best and
more safe decision"). A library
file or folder that cannot be read is reported as a library failure. A
source file whose same name + size candidates cannot all be hashed, and
none matches, is planned for copy. Rejected: skipping it (the photo may have
no readable copy), and failing it (the user loses the backup). The library
folder must exist before planning; a missing one is an error.
Why: a second copy in the library is safe; a photo left uncopied is not.

## DEC-016  2026-10-28  trash and hidden library entries never count as synced
Tags: dedup, safety
Decision: the agent proposed it after the S003 fast check, and the user
delegated the choice to "the safest". The library scan skips dot entries and
`$RECYCLE.BIN` / `System Volume Information`. Rejected: indexing everything
(a photo deleted in Finder sits in `.Trashes`; it would count as synced and
then vanish when the trash is emptied), and refusing a volume root as the
library (still misses hidden folders).
Why: skipping a real copy costs only a second copy; counting a doomed one
can lose the photo.

## DEC-017  2026-10-28  copy engine details left open by the specs
Tags: layout, safety, copy
Decision: agent's pick in S004; the user confirmed all seven points.
(1) Several `<any date> - <name>` folders: the earliest is reused.
(2) Temp files are `.photosync-tmp-<pid>-<n>-<camera name>` in the target
type folder, created only if absent. The next sync deletes regular files of
that name wherever the library scan walks (not in dot or trash folders); a
folder or symlink of that name is never touched. Since S034 the next sync
deletes only temps whose writer is gone (DEC-054).
(3) No no-replace rename outside macOS and Linux: every copy there fails
with an "unsupported" error. Rejected: a compile error (blocks building and
testing the rest), a replacing rename (INV-2).
(4) A copy whose folder flush fails after the rename is reported failed,
though the file is in place; the next sync skips it as already synced.
(5) The copy keeps the copy time as its mtime; the camera's mtime is not set.
(6) INV-4 is read as: files the first sync copied or skipped are never
copied again. A file failed for name collision is copied by a later sync
into another folder, as Layout allows (DEC-013); into the same folder it
fails again and nothing is copied.
(7) The local date comes from libc `localtime_r` (DEC-008's libc); std has
no time zones.
Why: each is the smallest choice that keeps the prime directive; the user
may override any of them.

## DEC-018  2026-10-28  exFAT: locked check + rename when no-replace rename is unsupported
Tags: safety, filesystems
Decision: user chose this after the agent found, on a real exFAT disk image,
that macOS returns ENOTSUP (errno 45) for `renamex_np(RENAME_EXCL)`. Only
when the OS reports the no-replace rename as unsupported, photosync takes an
in-app lock per library, checks that the final name is free
(`symlink_metadata` gives NotFound), and renames. Every other path keeps the
atomic no-replace rename. Rejected: refusing exFAT drives (no format is
native on both Mac and Linux), and writing straight to the final name (a
crash leaves a partial file there, which breaks INV-7).
Why: exFAT is the common format for a Mac + Linux drive. The remaining race
exists only against another program creating the same name in the same
instant, and every copy is still hash-verified.
Amended by DEC-055: "in-app" was wrong for two photosync processes; the check
and rename now also hold a lock across processes.

## DEC-019  2026-10-28  exFAT fallback details left open by DEC-018
Tags: safety, filesystems, tests
Decision: agent's pick in S012; the user confirmed all four points.
(1) The fallback's lock is keyed by the target folder's canonical path
(the folder the file is renamed into), so every path into one folder shares
it, even from two libraries nested in each other; a folder that cannot be
canonicalised fails that file with the error. Locks live for the process
(one small entry per folder). Amended in S013: it was keyed by the library,
which the S012 fast check found lets two library paths into one folder take
different locks.
(2) A poisoned lock is still taken: it guards no data, so a panic elsewhere
leaves nothing half-done.
(3) Linux falls back on EINVAL only, not ENOSYS (a kernel without
`renameat2`): such a kernel fails every copy, which loses nothing.
(4) `scripts/test_exfat.sh` skips every test that uses the `Mode` helper,
including `only_name_and_size_candidates_are_hashed`, which passes on exFAT
without testing anything because chmod does nothing there.
Why: each is the smallest choice that keeps the prime directive.

## DEC-020  2026-10-28  copy hardening details left open by S013
Tags: safety, copy, tests
Decision: agent's pick in S013; the user confirmed it.
(1) A session or `<TYPE>` folder that is a symlink (or a file) fails each
file that would go there, as a write error (`NotADirectory`) at that
folder; other types still copy. The run is not refused as a whole. Folders
are made one level at a time, so a library removed after planning is not
recreated: its files fail.
(2) A copy in place whose final folder flush fails gets its own reason,
`FailReason::NotFlushed { to, error }`, which says the copy is in place.
(3) Fault injection for tests is a cfg(test) one-shot EIO, armed by the
stage hook, at the next source read or folder flush; a normal build has
none.
(4) `scripts/test_exfat.sh`, when the image stays mounted after both detach
attempts, prints the paths and the commands to clean up by hand, removes
nothing, and exits non-zero (the test status if that failed, else 1).
Why: each is the smallest choice that keeps the prime directive.
Known limit (S013 fast check): the INV-3 flush test proves that each flush
call is present, but not that it really flushes. Replacing one with `Ok(())`
stays green, and no hook can observe an fsync.

## DEC-021  2026-10-28  progress and cancel details left open by S005
Tags: progress, cancel, api
Decision: agent's pick in S005; the user confirmed it.
(1) API: `plan_sync` and `copy_plan` take a `&mut Monitor`, which holds a
`Send` observer `FnMut(Event)`, a `Cancel` (a shared `Arc<AtomicBool>`) and
an injectable clock. Each event carries a full `Progress` snapshot (overall
and per source, elapsed, ETA), so front-ends keep no counts. Rejected: a
trait observer (a closure is enough, and forwards to a channel in one line).
(2) Planning progress counts only the source files to hash. Library
candidates are hashed while their source file is current, cancellable but
uncounted: which ones get hashed is known only as it runs. Each is the same
name and size as its source file, so the ETA stays about right.
(3) A file that fails counts as done with its whole size, so done meets
total at the end. Bytes count up to the scanned size.
(4) One `Bytes` event per 1 MiB chunk; the core does not throttle, the
front-end may.
(5) Cancel is checked between files and before each chunk read, never
between verify and rename: a file past its last read finishes. A file cut
short loses its temp file; it and every later file go to the new
`CopyOutcome::cancelled`, not to `failed`. A cancelled plan returns
`PlanError::Cancelled`. `copy_plan` still removes stale temps when cancelled
at once.
(6) ETA is `elapsed * (total - done) / done` over bytes, per phase, `None`
until a byte is done.
(7) Copy progress finds a file's source by the deepest root holding it,
from the new `SyncPlan::sources`.
Why: each is the smallest choice that serves the CLI, TUI and a GUI worker
thread alike, and none touches the prime directive.

## DEC-022  2026-10-28  sync report details left open by S006
Tags: report, api, failures
Decision: agent's pick in S006; the user confirmed it. Points (1),
(5) and (9) amended in S014, prompted by the S006 fast check (DEC-023).
(1) API: `Report::new(scans, plan, outcome)` takes the scans, plan and
`CopyOutcome` by value, so each reason keeps its `io::Error` (not `Clone`)
and the report can move to a GUI thread. Totals (`Tally`, files and bytes
per bucket, plus `not_flushed` within failed) are computed from the
entries, never stored, per source, per type and overall. `Report`'s fields
are private, read through `sources()`, `library_failures()` and `temps()`,
so its checks cannot be bypassed. Rejected: a report that borrows (ties it
to the inputs' lifetime).
(2) Nothing to copy (`sync_folder` gives `None`): the caller runs
`remove_stale_temps` and passes `CopyOutcome::nothing_to_copy(temps)`.
(3) A cancelled plan (`PlanError::Cancelled`): `Report::plan_cancelled(scans)`
reports every supported file as cancelled, with no library failures or
temp cleanup, as nothing ran past planning.
(4) A read error while planning becomes `FailReason::Read` in the report,
like one while copying.
(5) `Report::new` refuses inputs that break INV-6: a file the scans do not
hold (`ReportError::NotInScans`), a supported file in neither the plan
nor the outcome (`ReportError::Unaccounted`), a source given twice
(`SourceTwice`), a file listed with another size or mtime than the scan
(`Changed`), and a duplicate whose original is not reported (`NoOriginal`).
Rejected: trusting the caller (a mixed-up plan would give a silently wrong
report).
(6) Scan failures are listed per source, outside the sum. "Contents unknown"
marks a folder whose listing failed and an entry whose type could not be
read (it may be a folder), from the new `SourceScan::failed_folders`. An
entry the listing types as a file is just that entry.
(7) Per-type totals cover supported files only; ignored files have no type
and are in the per-source and overall totals.
(8) Library failures and the temp cleanup are carried in the report.
(9) A source given twice, in any spelling, is refused by `Report::new`
(`ReportError::SourceTwice`), so no entry can land on the other twin.
Why: each is the smallest choice that keeps INV-6 checkable and serves the
CLI, TUI and GUI alike; none touches the prime directive.

## DEC-023  2026-10-28  report hardening details left open by S014
Tags: report, api, failures
Decision: agent's pick in S014; the user confirmed it.
(1) Twin sources are found by the new `SourceScan::resolved_root`, which
`scan_source` resolves while the card is there. `Report::new` does no I/O,
so a card pulled after the copy cannot block its report. macOS folds case
here; Linux keeps it (see the parked Linux exFAT note). Rejected:
resolving roots in `Report::new` (fails once a card is gone).
(2) A duplicate whose original failed or was cancelled is the new
`FileOutcome::DuplicateOfFailed { of }` or `DuplicateOfCancelled { of }`;
the original's own entry holds the cause. A duplicate of a not-flushed
original is `DuplicateOfFailed` and not counted in `not_flushed`: only the
original is in place.
(3) A duplicate whose original is in no list of the plan or outcome is
refused (`ReportError::NoOriginal`): nothing says whether it is safe.
(4) The size and mtime check covers every plan and outcome entry that
carries a file; a plan read failure has a path only. `plan.to_copy` is not
read by `Report::new`: the outcome accounts for those files.
(5) `Report::plan_cancelled` does not refuse a twin source: every entry is
cancelled, so it claims nothing safe.
Why: the report is what decides whether a card is safe to format, so each
choice errs toward "not safe" and toward refusing over guessing.

## DEC-024  2026-10-28  volume discovery and the INV-10 device check (S007)
Tags: sources, destination, safety, api
Decision: agent's pick in S007; the user confirmed it. Point on the device
check tightened by DEC-025 (S007 fast check).
(1) A volume is a direct entry of a search root that is a folder (a symlink
is not followed) and a mount point: on another device than its parent, or
`/`. The symlink rule keeps out the macOS boot volume (`Macintosh HD -> /`);
the mount rule keeps out a folder left by an unclean eject, which would
send photos to the boot disk. Rejected: matching the name "Macintosh HD"
(the user can rename it), and comparing with the device of `/` (misses
leftover folders).
(2) Dot entries in a search root (`/Volumes/.timemachine`) are skipped, as
DEC-014 does on a card.
(3) A destination is a volume without `DCIM` whose root the effective user
may write to (`faccessat(W_OK, AT_EACCESS)`): this drops read-only mounts,
locked cards, macOS NTFS drives, and root-owned internal volumes such as
`/Volumes/Recovery`. Known limits: there is no internal/external test (it
needs DiskArbitration or udev); a Linux drive whose root folder is
root-owned (e.g. a fresh ext4) is not listed. A volume whose `DCIM` cannot
be checked is a failure only, never a destination: it may be a card.
(4) A missing search root holds no volumes and is not a failure (Linux
usually has one of its two). The Linux roots come from `$USER`; unset or
empty is an error.
(5) `scan_source` records `SourceScan::device`: the `st_dev` of the
resolved root when it is a mount point, else `None`, taken while the card
is there (as `resolved_root`, DEC-023). `plan_sync` refuses a library on
that device as `PlanError::LibraryOnSource`. A plain-folder source is
checked by path only, so temp-dir tests (one device, no mount points) are
not refused; a test sets the public field to inject a device. Rejected: a
device check for every source (refuses every library on the same disk as a
folder source, every test included), and an injectable device function on
`plan_sync` (API for tests only).
(6) The mount test in discovery is injected through a private function, so
unit tests treat temp folders as volumes; the real one is tested on `/`,
`/dev` and a temp folder.
Why: each picks what keeps a photo off the wrong disk, with the least API.

## DEC-025  2026-10-28  strict INV-10: library never on a source's disk or on any card
Tags: safety, sources, destination
Decision: user chose "strict" after the S007 fast check found two ways to put
the library on a card from the CLI. A plan is refused if the library is on
the same device (`st_dev`) as any source, whatever folder the source is,
mount point or not; or if the library's volume (walk up to its mount point)
has a `DCIM` folder at its top, i.e. is a card, selected or not. Rejected:
the S007 rule (device only for sources that are mount points). The cost the
user accepted: no sync from a folder on the same drive as the library.
Why: the prime directive; a typed path must not be able to write to a card.

## DEC-026  2026-10-28  strict INV-10 details left open by DEC-025 (S015)
Tags: safety, sources, destination, api, tests
Decision: agent's pick in S015, not yet confirmed by the user.
(1) `SourceScan::device` is now `u64`: the `st_dev` of the resolved root,
always. Supersedes DEC-024 (5).
(2) New `PlanError` variants: `LibraryOnSourceDevice` (same device, paths
apart) and `LibraryOnCard { library, volume }`. `LibraryOnSource` now means
the path case only. Checks run in order: in `DCIM`, then per source
(under, contains, same device), then card.
(3) The library's volume is the topmost folder of the resolved library path
on the library's device, walking up by path. If that walk reaches `/`, the
volume is the boot disk and is never a card: `/DCIM` is not looked at.
A card is never mounted at `/`, and refusing every boot-disk library over a
stray `/DCIM` buys no safety. On macOS the temp dir and `/Users` show the
device of `/`, so this case is the common one there.
(4) A `DCIM` that is a file does not make a card (as discovery). A volume
top whose `DCIM` cannot be checked, or a library whose volume cannot be
found, is refused as `PlanError::Library`: it may be a card.
(5) Known limit: a bind mount of a card's subfolder (Linux) has its own
mount point with no `DCIM` at its top, so only the device check catches it
when the card is a source. Not a case the app offers.
(6) Tests: `common::scan_elsewhere` sets `device` to `OTHER_DEVICE`
(`0xdead_beef_dead_beef`, no real id) to declare "on another device than
the library"; every test not about the device rule uses it, and the device
rule's tests call `scan_source` as is. Core has no special value. The card
rule's tests inject the volume lookup (`check_library_with`, private), as
DEC-024 (6) does for discovery. `lib.rs` has `extern crate self as
photosync_core` under `cfg(test)` so the shared `common` names the crate
one way.
Why: each keeps a typed path off a card with the least API, and keeps the
fake device visible in every test that uses it.

## DEC-027  2026-10-28  INV-10 enforcement: read-only device, test-support seam, re-check before copy (S016)
Tags: safety, api, tests
Decision: agent's pick in S016, not yet confirmed by the user.
(1) `SourceScan::device` is a private field read through `device()`.
Supersedes the "test sets the public field" part of DEC-024 (5) and
DEC-026 (6).
(2) Tests fake it through `SourceScan::set_device_for_test`, compiled only
under `cfg(test)` (unit tests) or the Cargo feature `test-support`
(integration tests, which link the lib as an outside crate). Only
photosync-core's dev-dependency on itself turns the feature on, and
dev-dependencies never reach a normal build, so `cargo build` of the CLI
or any downstream crate has no such method. `tests/test_seam.rs` proves it
by checking a probe crate that depends on photosync-core without features:
reading `device()` compiles, `scan.device = …` fails (E0616), the seam call
fails (E0599). A crate that names `features = ["test-support"]` itself
opts in on purpose; that is not a normal build. Rejected: a
`#[doc(hidden)]` pub fn (any caller can use it), `cfg(test)` alone (the
integration tests cannot see it).
(3) `SyncPlan` records the library's device and volume as `check_library`
found them, and each source's device; the fields are `pub(crate)`, so a
`SyncPlan` can now only come from `plan_sync`. `copy_plan` re-checks before
anything else, the stale-temp cleanup included: the library's device, then
its volume, then no source device equal to the library's. New
`CopyError` variants: `LibraryDeviceChanged`, `LibraryVolumeChanged`,
`LibraryOnSourceDevice`, and `Library` (the library cannot be found or
stat'ed, e.g. unplugged; before, that failed file by file). The source
devices are compared as recorded at scan time, not stat'ed again: the
source may already be gone, and the library check is what moved.
(4) The card rule is tested end to end by `scripts/test_exfat.sh`: it
mounts a second exFAT image with `DCIM` at its top and names it in
`PHOTOSYNC_TEST_CARD`. Without the variable the test prints that it was
skipped and returns. With the lookup replaced by `|_| Ok(None)` the script
fails (checked in S016).
Why: no hand-edited scan or plan gets a write onto a card, and no front-end reaches the tests' seam.
Limit (S016 fast check): `cargo test --workspace` and `--all-targets`
builds unify dev-dependency features, so there the binary links a core with
`test-support` on. Only a plain `cargo build` excludes the seam by
construction. `crates/photosync/tests/no_test_seam.rs` therefore fails if
the binary's source or manifest mentions the seam.

## DEC-028  2026-10-28  CLI details left open by S008
Tags: cli, safety, exit-codes, tests
Decision: agent's pick in S008, not yet confirmed by the user.
(1) A missing `--dest` is refused ("does not exist, create it on your
external drive first"), never created. Rejected: creating it inside an
allowed volume after the INV-10 checks; the checks need the folder, so
the mkdir would come first, and a typo or an unplugged drive puts the
library on the boot disk.
(2) Exit codes: 0 every supported file copied or already on the drive;
1 the sync ran but some file failed or was cancelled, a scan entry was
unreadable (its contents are unknown), or the report was refused after
copying; 2 usage error (clap's own code) or refused before copying (root,
scan, twin source, INV-10, missing library, invalid name, copy re-check).
Library read failures and temp cleanup failures leave the code alone:
they hide no source file (DEC-015).
(3) A twin source (same `resolved_root`) is refused right after its scan,
before planning, in `flow::sync`; core's `Report::new` check stays.
(4) The report goes to stdout; scan lines, progress and refusals to
stderr. `sources` and `drives` print one path per line on stdout, notes
on stderr. Progress is throttled on the events' clock: a phase's start
and end always, between them one line per 250 ms rewritten in place on a
terminal, one line per second otherwise. The report lists every file not
copied or skipped, with its reason; copied and skipped files only count.
Ignored files are counted by extension ("hidden" for dot files).
(5) Ctrl-C: a `sigaction` SIGINT handler (libc, DEC-008) sets the core
`Cancel`, with `SA_RESETHAND`, so a second Ctrl-C kills the process at
once, which INV-7 makes safe. Rejected: the ctrlc crate (not approved).
(6) Root (euid 0) is refused for every command, after argument parsing,
so `--help` still works.
(7) `sources` and `drives` take a hidden `--search-root`, for tests only:
it changes where volumes are looked for, not what counts as one (mount
points only, DEC-024), and `sync` takes explicit paths anyway.
(8) The scan line prints each source's `st_dev` in decimal, so a hand
check can compare it with `stat -f %d <card>`.
(9) The binary has `args` (clap), `flow` (one sync, start to report,
through a `Watch` trait), `text` (rendering) and `system` (euid,
SIGINT); the TUI (S009) reuses `args` and `flow`. `--name` is checked by
core's `sync_folder`, after planning, not again in the binary.
(10) Tests: `tests/cli.rs` covers what one temp device allows (help,
usage, twin source, every INV-10 refusal but the card, missing library,
empty lists); `tests/exfat_e2e.rs` syncs for real on the images of
`scripts/test_exfat.sh`, which now mounts a third one, the library drive
(`PHOTOSYNC_TEST_LIBRARY`). Root refusal is tested on `refuses_user`
only: the suite never runs as root.
Why: each picks the reading that keeps a photo safe and the output
scriptable, with no new dependency.

## DEC-029  2026-10-28  hardware hand checks deferred to S017
Tags: plan, verification
Decision: the user can't do the hand checks now and asked to go on without
them. S008 is done on the automated proof (a real exFAT card image to a
drive image, end to end). Its hand checks (SD card, X-T5 over USB, sudo,
Ctrl-C) move to S017, which waits for the user. Audit F09 now closes with
S017.
Why: the user asked; the risk is noted, since real card mounts are proven
only on disk images until S017.

## DEC-030  2026-10-28  TUI details left open by S009
Tags: tui, gui, architecture, cli, exit-codes, tests
Decision: agent's pick in S009, not yet confirmed by the user.
(1) Layout, all in the binary crate: `app` (model: screens, `Key`, `Msg`,
`Effect`, `update`, exit code), `worker` (runs `flow::sync` on a thread,
sends `Msg` to a closure), `tui` (crossterm keys to `Key`, the loop, with
keys and discovery injected) and `tui/view` (ratatui only). `app`,
`worker`, `flow`, `text` and `args` never name ratatui or crossterm
(`tests/ui_split.rs`, a source-text check like `no_test_seam.rs`). The GUI
reuses them by moving them into a shared crate, or by living in this
binary. Rejected: a shared crate now (one user, and S009 touches only
`crates/photosync`). DEC-067 says how the GUI reaches them.
(2) The model takes keys, not commands: every printable key is
`Key::Char`, and its meaning depends on the screen, so a text field takes
`q`. A GUI adds its own messages beside `Key`.
(3) Ctrl-C is a key in raw mode: while copying, the first cancels and a
second quits at once (as the CLI's second Ctrl-C, INV-7); elsewhere it
quits. No SIGINT handler in the TUI. The model makes a new core `Cancel`
for each start and sets it itself, so a retry gets a fresh one.
(4) One sync per TUI run (S010 adds the session). After a refusal the user
can go back and start again; the report screen only quits. Superseded by
DEC-034 (2): one session per TUI run, and the report offers the next card. On quit the
report is printed to stdout and the refusal to stderr; the exit code is
`sync`'s; quitting mid-sync, or a worker that died before its result,
exits 1.
(5) The library is a text field, default `<drive>/Photos`. The TUI does
not check that it exists: the core refuses, and the screen shows the CLI's
sentence (DEC-028 (1)), which names `photosync drives`.
(6) No command without a terminal (stdin or stdout not a tty) is clap's
usage error, exit 2: the TUI would wait for keys nobody can type.
(7) Closed stdout: a report that cannot be written is said on stderr and
the sync keeps its 0/1 code, which is what says if the card is safe.
`sources` and `drives` exit 1 if their list cannot be written: it is their
whole output. A closed stderr is ignored: there is nowhere to say it.
(8) The CLI's in-place progress line is cut to the width of stderr's
terminal minus one column (`TIOCGWINSZ` on fd 2), read when a line is due;
unknown width keeps the whole line; log lines are never cut.
(9) ratatui without default features, `crossterm` only; crossterm is used
through `ratatui::crossterm`, so the two never disagree on its version.
(10) Tests: the binary's unit tests share `tests/common/mod.rs` (it finds
the binary with `option_env!`). The exFAT TUI test writes its photos to
`DCIM/900_TUI` on the shared card image and removes them when done:
`tests/exfat_e2e.rs` counts the card's files.
Why: the smallest TUI that keeps the model free of the terminal, reuses
the CLI's sentences and exit codes, and errs toward "not safe to format".

## DEC-031  2026-10-28  TUI hardening details left open by S018
Tags: tui, exit-codes, tests, architecture
Decision: agent's pick in S018, not yet confirmed by the user.
(1) The model's outcome is replaced only by a later sync's end
(`Finished`, `WorkerGone`, `Crashed`), never by going back or by
starting: the last sync wins. So a retry that is refused before copying
(exit 2) replaces an earlier worker death (exit 1); both say not safe.
Superseded by DEC-033 (2): across a session the worst outcome wins; S020
brings the TUI to that rule.
While a retry runs, quitting exits 1 as before (DEC-030 (4)).
(2) Worker panic: after `ratatui::try_init`, `tui::run` wraps the panic
hook so it acts for the TUI's own thread only. Another thread's panic is
not printed (stderr is the screen); `worker::spawn` catches it
(`catch_unwind`) and sends `Msg::Crashed(text)`, which the model shows
like a worker death, with the panic's text, exit 1. The panic's location
is not kept. Rejected: catch alone (the hook runs before unwinding, so it
would still restore the terminal).
(3) Confirm with no source picked stays on Confirm with a hint, rather
than going back to the cards.
(4) `tests/ui_split.rs` walks every `.rs` under `src/`: outside
`src/tui/` and `main.rs` no file may name ratatui, crossterm or the `tui`
module, nor use `#[path]`; `main.rs` may name `tui` but not the crates;
in `src/tui/` no `pub use` or `pub type` may name the crates. Words are
matched as identifiers, after cutting `//` comments. A renamed
dependency in `Cargo.toml` (`package = "ratatui"`) is not checked.
Why: each errs toward "not safe to format" and keeps the check from
depending on a list someone must remember to extend.

## DEC-032  2026-10-28  S010 first part: core Session API, and the split of S010
Tags: session, api, safety, plan, gui
Decision: agent's pick in S010, not yet confirmed by the user.
(1) API: `Session::new(library, name)` (refuses a bad name before any card
is read), `sync(roots, cancel, &mut dyn Watch)` or `sync_scans(scans, ..)`
for cards already scanned, `record_crash(roots, what)`, then `runs()`,
`folder()`, `totals()`, `by_type()`, `verdict()` and `not_complete()`. Each
`Run` keeps its sources and its end: `RunEnd::Report`, `Refused(SyncError)`
or `Crashed(text)`. `SyncError` and `Watch` moved from the binary's `flow`
into the core; `flow` is now a session of one sync.
(2) `Verdict` order, worst last: `Complete`, `Refused`, `NotComplete`. The
session's verdict is the worst of every run, none = `Complete`. A refused
run counts: its cards were not synced, and with one reader a later card at
the same path cannot be told from it. A refused report (`SyncError::Report`)
and a crash are `NotComplete`. The CLI maps them to 0, 2 and 1.
(3) Memory between cards: each library file a run copied or found already
synced, with the session file it holds. A later file whose library match is
one of them goes to the new `SyncPlan::seen_earlier` and is reported as
"duplicate in session" of the earlier card's file. A copy in place but not
flushed is not remembered: a later twin of it is "already on drive", as
the next sync would find it (DEC-017 (4)). `Report::new` refuses a plan with
`seen_earlier` (only a session can vouch for those).
(4) Stale temps are removed once per session, by its first run that gets
past planning, after the INV-10 re-check, even with nothing to copy. The
session copies through a crate-only `copy_files`, which never cleans;
`copy_plan` stays as a session of one plan and still cleans first.
(5) `SyncPlan::to_copy` holds `Planned`: the file, its source index, its
hash if planning read it, and its twins (the plan's duplicates of it).
Progress uses the index; path matching (`source_of`) and its unit test are
gone. A copy whose bytes read differ from the planning hash fails with the
new `FailReason::SourceChanged` and its duplicates take that failure. A
file that planning did not hash has no duplicate to protect and is not
read twice.
(6) INV-10 re-check before each new type folder; after one fails, before
every file. The file fails with the new `FailReason::Library(CopyError)`.
Tested through a test-only device fault that stays armed.
(7) Every rename takes the target folder's lock, not only the exFAT
fallback (DEC-008).
(8) The twin root check is in the session (after each scan, before
planning), not in `plan_sync`: a `plan_sync` refusal would break the kept
test `report_refuses_a_source_root_given_twice_in_any_spelling`.
(9) Split: S010 is too big for one change, and two of its items contradict
tests the user's rules say not to change without asking. Proposed: S019
copy several cards at once (concurrency), S020 one card after another in
the CLI and the TUI (supersedes DEC-031 (1) once the user drops or changes
`app::tests::a_later_sync_replaces_the_kept_failure`), S021 twin fallback
on a read error (F07; the user to say how
`duplicate_of_an_original_that_failed_to_read_is_failed_not_skipped`
changes). The user accepted the split (DEC-033), ordered S020, S021, S019,
and ruled on both tests; S010 closed on part 1.
Why: the core holds every session rule for the CLI, TUI and GUI alike;
each choice errs toward "not safe to format".

## DEC-033  2026-10-28  S010 split accepted; worst outcome wins; F07 test change
Tags: plan, session, exit-codes, tests, safety
Decision: user decided. (1) The split of DEC-032 (9) is accepted: S010
closes on its first part; order S020, S021, S019 (parallel copying last),
then S017. (2) Worst outcome wins across a session, superseding DEC-031
(1): in S020, `app::tests::a_later_sync_replaces_the_kept_failure` may be
rewritten to the session rule (exit 1, the message names the crashed run).
(3) F07: in S021, `duplicate_of_an_original_that_failed_to_read_is_failed_not_skipped`
may make both copies unreadable, which keeps its purpose, and a new test
proves a readable twin is copied by the fallback.
Why: a later sync must never hide an earlier one that was not safe, and
backup mode must use the good card.

## DEC-034  2026-10-28  one card after another in the CLI and TUI (S020)
Tags: cli, tui, session, exit-codes, tests, gui
Decision: agent's pick in S020, not yet confirmed by the user.
(1) CLI: `photosync sync --dest <library> [--name <name>] [--source
<card>...] --one-by-one`. After each sync it asks on stderr for the next
card and reads one line from stdin: Enter alone takes every mounted card
(discovery, as `photosync sources`) as the next sync, a line names one
card by path, `q`, the end of the input or Ctrl-C finishes. Without
`--source` it asks for the first card; none given is "No card was
synced.", exit 2. A Ctrl-C during a sync cancels it and ends the session.
Each sync's report is printed as it ends; after several, a session
summary: one line per sync, the totals of all, and the session's result
naming each sync that was not complete. The exit code is the session's
worst (DEC-033 (2)). stdin at its end (`</dev/null`, a script) ends the
session after its first card, so it never hangs. The prompt is read with
`read(2)` byte by byte, not std's `read_line`, which retries after a
signal and would make a Ctrl-C wait for Enter. `sync` takes the hidden
`--search-root` too, for tests (DEC-028 (7)). Rejected: waiting for a
newly mounted card by polling discovery (a swapped card often comes back
at the same path, so "new" cannot be told from a path, and the wait
needs a timeout to stay scriptable); syncing only cards not synced
earlier (same reason).
(2) TUI: one core `Session` per run, made by the first start; the library
and name are fixed then. A name that is not a folder name is refused
before the session is made, so it can still be fixed. The report and the
refused screens take `n`, the next card: back to the cards, nothing
picked, discovery again; from the cards Enter goes straight to Check, and
Esc from Check back to the cards; Esc on the cards does nothing in a
session. To change the library or the name, quit and start again.
Rejected: re-targeting a session whose syncs were all refused (more core
API for a typo; the refused sync would still count). Amended by DEC-050: it
stops counting once its cards complete later, and the TUI checks the library
before it makes the session.
(3) The model keeps the session as `flow::Shared` (`Arc<Mutex<Session>>`)
and lends it to the worker in each `Job`. The worker unlocks before it
sends `Finished` or `Crashed`; the model then reads the session's last
run, and records a crash (`Crashed`, or `WorkerGone` before an end) with
`Session::record_crash`. A lock poisoned by a panic is taken as is: the
session records a run only at its end and marks its cleanup done before
running it. `App::exit` never locks, as a sync may hold the session: the
exit is kept from the last end. Quitting mid-sync exits 1, prints the
session's report so far, and names this sync and every earlier one not
complete.
(4) Text: a session of one sync prints exactly the old report, refusal
or crash sentence, so single syncs read as before. Several: "Sync N of
M: <cards>" sections, "All syncs" totals, and "Result: NOT COMPLETE: k of
M syncs not complete: sync N (<cards>): <what>; ..." A refused sync is
named too (DEC-032 (2)).
(5) Session memory (S010 fast check): the latest sync wins a library
path it copied to or found; a file matching a library file that is its
own earlier sync's (same path) is "already on drive". A different card
at the same mount path holding the same photo is "already on drive" too:
both are skipped, so it is safe.
(6) Tests: `app::tests::a_later_sync_replaces_the_kept_failure` is
rewritten as `a_later_sync_never_replaces_the_kept_failure` (DEC-033
(2)). No real sync gives a refused report (`SyncError::Report`), and the
model's input is now the session: `a_report_that_does_not_add_up_exits_1`
checks that such a run maps to exit 1, and
`a_failure_is_kept_after_going_back_and_quitting_elsewhere` uses a crash
instead of it. App tests use real refused syncs on temp dirs and the
real worker. `tests/common` is included once at the crate root, for the
app and TUI tests. The CLI session on real copies is its own test binary,
`tests/exfat_session.rs`, so it never runs beside `exfat_e2e.rs`, which
counts the card's files.
Why: the session's rules stay in the core and each front-end only asks
for the next card, so no exit or message can say "safe to format" while
any sync of the session was not complete.

## DEC-035  2026-10-28  S022: seam for a refused report, prompt poll, cancel during lookup
Tags: tests, session, cli, api, safety
Decision: (1)-(3) agent's pick in S022, not yet confirmed by the user;
(4) user decided.
(1) The coverage S020 dropped (DEC-034 (6)) is back through two seams.
Core: a `#[cfg(test)]` thread-local switch in `session.rs` refuses the
next report after its copy, for the core's own unit test (not the
`test-support` feature, so nothing is exposed). Binary: the core gets a
public `Session::record_refusal(roots, SyncError)`, and the app tests
inject a sync through `worker::spawn_with` (now `pub(crate)`, its sync
takes `&mut dyn Watch`) that records a `SyncError::Report` with it. A
refusal is never `Complete`, so the call can only keep or worsen the
session's verdict. Rejected: a `test-support` seam (the binary may not use
it, DEC-027); scans crafted so a real sync's report fails (no such path
found; it would lean on the core's internals); a general `record(RunEnd)`
(it could record a complete report of cards never synced).
(2) The prompt waits with `poll(2)` for at most 100 ms, then checks the
cancel again, so a Ctrl-C that lands just before the wait is seen.
(3) Cards the lookup (Enter, discovery) returns after a Ctrl-C are
dropped: no sync runs and none is recorded. For the first card that is
"No card was synced.", exit 2; for a later one the session ends with its
summary.
(4) User decided (option A of three put to the user): with no terminal
on stdin, `--one-by-one` never prompts: it syncs the `--source` cards and
ends; with none, "No card was synced.", exit 2. This supersedes DEC-034
(1)'s "stdin at its end ends the session after its first card": a pipe
that stays open hung it. The tests that typed cards into a pipe
(`cli.rs` `one_by_one_without_input_ends_after_the_first_card`,
`one_by_one_takes_cards_until_q_and_sums_up_the_session`,
`ctrl_c_at_the_prompt_ends_the_session_with_its_summary`, and
`exfat_session.rs`) run on a pseudo-terminal (`tests/common` `Pty`,
libc `openpty`, echo off, Ctrl-D for the end of the input), their
assertions unchanged. Rejected: a bounded wait on a pipe (timing); reading
only what a pipe holds already (races).
Why: no test may lean on a seam the binary could ship, and no cancel may
turn into a recorded sync or a missed one.

## DEC-036  2026-10-28  twin fallback on a read error (S021, F07)
Tags: copy, report, progress, safety, api, gui
Decision: agent's pick in S021, not yet confirmed by the user.
(1) Only a read error of the planned file falls back to its twins
(`Planned::twins`, plan order); each twin is copied as the file is, its
read checked against the planning hash, so nothing is hashed again. A
twin that cannot be read, or reads otherwise (`SourceChanged`), is passed
for the next. A drive-side failure (write, flush, collision, INV-10
re-check) is not retried: of the planned file, nor after a twin's, which
ends the tries; the file then fails with its own read error and the
twin's drive error is not reported, except a twin's copy left in place
unflushed, which is (amended in S023, DEC-037 (2)). Twins are only on
another device: a same-hash file on the planned file's own card is a
plain duplicate (amended in S023, DEC-037 (1); by device in S024,
DEC-038). A planned file that read otherwise
than planned does not fall back: it fails and so do its duplicates, as
INV-3 and the kept test `a_file_changed_between_planning_and_copy_fails`
say. Rejected: falling back on `SourceChanged` too (the S021 brief
suggested it; it breaks that test, and the card's file is not what
planning read).
(2) Outcome: `CopyOutcome::fallbacks`, each `Fallback { original, error,
tried, copied }`. Report: the copied twin is `Copied`; the unread file,
and each tried twin with a read error, is the new
`FileOutcome::DuplicateUnread { of, error }` (skipped, `of` = the copied
twin), counted apart in `Tally::unread`; a tried twin that read otherwise
is `Failed(SourceChanged)`; untried twins are `DuplicateInSession` of the
copied twin. With no twin copied: the file `Failed(Read)`, its twins
`DuplicateOfFailed`. A cancel while a twin is read: `Cancelled`, its twins
`DuplicateOfCancelled`. INV-6 per card holds: each file is counted on
its own card. Rejected: a new bucket (breaks the INV-6 sum and every
front-end's table).
(3) A sync whose only problem is `DuplicateUnread` is complete, exit 0:
the drive holds the very bytes planning read from that file. The CLI and
TUI show its line ("read error ... copied from its twin ..."), a line
under the totals, and "check that card" in the result. After several
syncs, its state in the summary and the session's result name the card
too (amended in S023, DEC-037 (3)).
Amended in S023, prompted by the S021 fast check: (1), (3).
(4) Progress: a fallback stays one file of the planned file's card; no
event names the twin; the twin's bytes count only past what the failed
read counted, so total never changes and done never goes back.
Rejected: moving the file to the twin's card mid-copy (changes totals).
(5) `Report::new` now reads `plan.to_copy` for the twins (amends DEC-023
(4)) and refuses a twin the plan does not list (`ReportError::NotATwin`).
(6) Two tests built `CopyOutcome` by struct literal; they now start from
`CopyOutcome::nothing_to_copy`, no assertion changed.
Why: backup mode must use the good card, and a card with a read error
must never be hidden, nor a changed file be called safe.

## DEC-037  2026-10-28  twin-fallback reporting fixes (S023)
Tags: copy, report, plan, cli, tui, test
Decision: agent's pick in S023, not yet confirmed by the user. Prompted
by the S021 fast check.
(1) Planning adds a same-hash file as a twin only if it is on another
source than the planned file (INV-3). On the same card it stays a
duplicate in session of the planned file: if that one cannot be read,
it is `DuplicateOfFailed`, never read in its place (the card's read
error would likely hit it too). With a twin on another card copied, it
is `DuplicateInSession` of that twin.
(2) A twin's copy in place but not flushed (`FailReason::NotFlushed`)
ends the tries, as any drive-side failure, and is reported through the
new `CopyOutcome::unflushed_fallbacks` (`UnflushedFallback { original,
error, tried, twin }`): the planned file `Failed(Read)` with its own
error (nothing confirms the photo is on the drive), each tried twin
`Failed` with its reason (a read error too: not `DuplicateUnread`, as
nothing is confirmed), the twin `Failed(NotFlushed)` (in `not_flushed`,
under the twin's card), untried twins `DuplicateOfFailed` of the twin.
`Report::new` checks these twins against the plan as for `fallbacks`.
Rejected: a field on `Fallback` (its `copied` would then lie) or an
enum in it (breaks the struct literal in tests/report.rs).
(3) Wording: `run_state` appends ", but N could not be read on <cards>
and was/were copied from another card: check that card/those cards" to
"complete" or to the failed/cancelled counts; `session_result` appends
the same sentence over all syncs, complete or NOT COMPLETE, naming each
card once. A not-complete sync with an unread file thus names it twice
(in its state and at the end). The one-sync `result_line` is unchanged
(amended in S024, DEC-039 (2): it names the cards too).
The TUI box stays green (exit 0, DEC-036 (3)).
(4) The multi-sync tests need real syncs, so sources on another device
than the library: they run on the exFAT drive image of
`scripts/test_exfat.sh` (`PHOTOSYNC_TEST_LIBRARY`) and skip, saying so,
under plain `cargo test`. The read error is made by removing the planned
file once copying starts (`common::LoseOnCopy`): chmod does nothing on
exFAT.
Why: the report is what the user trusts before formatting a card; it
must say what is on the drive and which card had trouble.

## DEC-038  2026-10-28  a twin must be on another device, not only another source
Tags: fallback, sources, safety
Decision: agent's pick after the S023 fast check, not yet confirmed by the
user. A twin must be on another device (`st_dev`) than the planned file's
source. Two sources on one card (e.g. `/Volumes/A` and `/Volumes/A/OLD`)
give no twin, so a card that just failed to read is never read again in its
place. Rejected: comparing source indexes (lets one card count as two). The
cost: exFAT session tests need separate card images.
Why: matches INV-3 ("another card") and the strict device rule of DEC-025.

## DEC-039  2026-10-28  S024: device rule in planning, one-sync result names unread cards, twin images
Tags: fallback, plan, report, cli, tui, test
Decision: agent's pick in S024, not yet confirmed by the user.
(1) Planning applies DEC-038 by comparing the scans' `device()` of the
planned file's source and of the same-hash file's; nothing else changes
(copy and `Report::new` already follow `Planned::twins`).
(2) The one-sync `result_line` names each card with an unread file, with
the sentence of `run_state` and `session_result` (DEC-037 (3)): complete
"... already there). 1 could not be read on /Volumes/A and was copied
from another card: check that card."; NOT COMPLETE puts the same sentence
after the counts (and the not-flushed count), before "Do not format the
card." Supersedes the "on its card" wording, and two assertions of it
change to the named card: `a_file_copied_from_its_twin_shows_its_read_error`
and the per-sync line in `the_result_box_names_the_card_a_file_was_copied_from_its_twin_for`.
(3) Core tests give each card of a twin test its own fake device with
`common::scan_card(root, n)` (`OTHER_DEVICE + n`); `scan_elsewhere` stays
one device for all, so a test not about twins is unchanged.
(4) `scripts/test_exfat.sh` mounts two more 16 MB exFAT images, the twin
cards, passed in `PHOTOSYNC_TEST_TWIN_A` and `PHOTOSYNC_TEST_TWIN_B`
(`common::twin_cards`); the binary's two twin tests put cards a and b in a
fresh folder on each (`OnDrive`), card c stays in the temp dir. Rejected:
reusing the `PHOTOSYNC_TEST_CARD` image, which other tests count files on.
Why: the device is what makes a card another card, and a not-complete
result must still say which card had a read error.

## DEC-040  2026-10-29  S019: several cards copied at once, one thread per card
Tags: copy, progress, session, concurrency, safety, api, gui, test
Decision: agent's pick in S019, not yet confirmed by the user.
(1) Threads: a `Session` copies each card on its own scoped std thread;
a card is a device, so two sources on one card (DEC-038) share a thread
and that card is read one file at a time. Only one card with files: it
copies on the calling thread. `copy_plan` stays one thread, in plan
order, so its events keep `EventKind`'s strict order and the kept test
`copy_events_arrive_in_order_and_their_totals_match_the_plan` holds.
Rejected: one thread per source (reads one card from two threads), and
`copy_plan` concurrent too (breaks that test's order).
(2) Shared: the progress tracker behind one `Mutex`, the `Cancel`, a
"a thread panicked" flag, and a "the INV-10 re-check failed" flag. Per
thread: its folders made, its temp failures, its files. Renames into one
folder keep the folder lock (DEC-008, DEC-019).
(3) Events: the observer is called from the copy threads, one call at a
time, under the tracker's lock, each event a whole snapshot (sources add
up to the totals). `Started` comes from the calling thread before any
copy thread starts; `Finished` or `Cancelled` once all have ended. Each
card keeps the per-file order and has at most one file in flight.
Rejected: funnelling events to the calling thread over a channel (an
observer would act late, after its copy thread moved on).
(4) Fallback rule: each planned file belongs to the thread of its own
card. Its twins (F07) are the plan's duplicates of it, never planned
themselves, and are read only by that thread, even while the twin's card
is read by its own thread for its own files (only slower). So two
threads never copy one photo, and a fallback never races a planned copy
of its twin: there is none. Two cards' different photos of one name
race to the folder lock: one is copied, the other is a name collision;
which wins is timing.
(5) INV-10: each thread re-checks before its first file into each type
folder, even one another thread made; once a check failed in any thread,
every thread checks before every file.
(6) Cancel and panic: each thread checks "cancelled or another thread
panicked" between files, between twins and before each chunk read. A
panicking thread raises the flag in its unwind, the others stop as on a
cancel, every thread is joined, then the first panic (in card order)
goes on to the caller, which records the sync as crashed: not complete
(DEC-031, DEC-034). The panicked thread's temp file stays, as after a
kill (INV-7); the next session removes it. Rejected: the core catching
the panic into `RunEnd::Crashed` (a second crash path beside the
front-ends' own).
(7) Outcome: merged by index in `to_copy`, so its lists are in plan
order whatever the timing; temp failures in card order.
(8) Tests: a shared test hook (`Arc<dyn Fn>`) that copy threads take on
from the thread that starts them; the per-thread hook stays on its
thread. Stages only the shared hook sees: `Checking`, `FolderReady`,
`RenameLocked`, `RenameUnlocking`, `Halted`. Threads meet at hook stages
(a `Mutex` + `Condvar` meeting that fails after 10 s instead of hanging),
never sleeps. `Tracker` keeps its one-file-at-a-time methods for
planning and its kept unit test, beside per-source ones.
(9) Front-ends: the CLI line and the TUI draw from each event's
snapshot only, so several files in flight need no change; the TUI's
cancelling sentence says "files". A GUI forwards events to its UI
thread, as the TUI does, and must not assume one file in flight.
Why: the simplest design that keeps every invariant: one owner per photo,
one lock per folder, one lock for progress, and every thread joined.

## DEC-041  2026-10-29  the TUI shows the app as "phoosync pro 2"
Tags: ui, naming
Decision: the user's call. The TUI's title and text name the app "phoosync
pro 2", a joke name, spelling intended. The binary, crates, `--help`,
`--version` and the docs stay `photosync` for now.
Why: the user wants the joke name in the UI only.

## DEC-042  2026-10-30  a file that reads another length than scanned fails (S026)
Tags: copy, report, safety, api
Decision: agent's pick in S026, not yet confirmed by the user.
(1) The copy compares the bytes read with the scanned size, for every file,
before the hash check and the flush. Any difference fails with the new
`FailReason::SizeChanged { scanned, read }`. Rejected: reusing
`SourceChanged`: its text says "since it was planned", which is wrong for a
file shrunk before planning, and the two counts tell a bad card from an edit.
(2) A file that grew fails the same way, as the plan compared it by its old
size and the report counts that size. Rejected: copying what is there now.
The length is checked after the read, not during it: a file that keeps
growing ends in a write error at worst, and its temp is removed.
(3) A wrong length is not a read error, so the planned file does not fall
back to its twins, as `SourceChanged` does not (DEC-036): safe, as it fails
and the sync is not complete, but a re-sync is needed even with a good twin.
A twin that reads another length is passed for the next, and fails.
(4) The read-time check stands alone: it does not stat the file. A stat could
agree while the read came up short.
Why: a short read gives no error, and neither the read-back nor a planning
hash taken after the shrink can see it, so the size is the only witness.

## DEC-043  2026-10-30  how the report shows duplicates and names the card (S028)
Tags: report, cli, tui
Decision: agent's pick in S028, not yet confirmed by the user.
(1) Each duplicate in session gets its own line, as specs Sources and INV-6
ask, listed after the failed and unreadable lines: a backup card lists every
photo, and what went wrong must not get lost among them. Rejected: one count
per card, shorter, but it does not list the extra copies.
(2) A count line splits the skipped by reason, only when a source has
duplicates: with none, the skipped are all on the drive and the result says
"already there". Files already on the drive get no line of their own: a
re-run would list the whole card. A file copied from its twin counts as a
duplicate (INV-6) and keeps its "read error" line.
(3) The line gives the file under `DCIM` and only the card of the original,
to fit 80 columns, as the TUI's report cuts long lines. It says "on drive
from", not "copied from": an earlier card's file may have been found on the
drive.
(4) The card is the path above the file's last `DCIM`. The outcome holds a
path, not its source, and the source may be of an earlier sync; no core change
this way. A path without `DCIM` is shown whole. A `DCIM` folder inside a
card's `DCIM` would show that folder as the card: its path still leads there.
Why: the report must say why a card's file was skipped and which card's file
is on the drive, without burying a failure.

## DEC-044  2026-10-30  a sync name is matched by the filesystem's lookup, not by bytes (S027)
Tags: layout, unicode
Decision: agent's pick in S027, not yet confirmed by the user. The approach is
the one the step prefers. A dated folder is reused if its name is the sync's
name byte for byte, or if `<its date> - <name>`, as spelt, has the same device
and inode as the listed entry (lstat, so never through a symlink). Rejected:
a normalisation table or crate (DEPS, and it could not know each filesystem's
rule). Measured on macOS images: exFAT and HFS+ list `Tirane\u{308}` for a
folder made as `Tiran\u{eb}`, APFS lists what it was given; all three resolve
both forms to one inode. Consequences, also the agent's: (1) another case now
reuses the folder on a case-insensitive drive; it already did on the same
date, and never overwrites, as a collision fails the file. (2) Not found and
name too long are "no such folder"; any other lookup error refuses the sync,
as it says nothing about the folder and a guess could split the trip. (3)
Same bytes skip the lookup, so an inode that is not stable cannot split a plain
re-run. Linux exFAT, from an agent's recollection of the kernel's fs/exfat (source not
read), NOT measured (check in S025):
lookups fold case with the upcase table and do not normalise, and names are
stored as given. So one form reuses its folder, as listed bytes match; a folder
a Mac made decomposed and a name typed composed on Linux are two names and two
folders. No photo is lost or copied twice: already-synced detection goes by file
name, size and hash, in any folder.
Why: only the filesystem knows which names it treats as one folder.

## DEC-045  2026-10-30  a later copy of an earlier sync's photo is resolved in the plan (S038)
Tags: report, session, plan
Decision: agent's pick in S038, not yet confirmed by the user. Planning treats
every copy in a plan of a photo an earlier sync holds the same way as the first:
`seen_earlier` naming the earlier card, or "already on drive" if it is that
very file. So `duplicates` never points at a `seen_earlier` file, and
`Report::build` now refuses one that does (NoOriginal). Rejected: following
`of` through `seen_earlier` in the report, which keeps a chain that can name
the wrong card and cannot make the same card given again "already on drive".
Consequence, also the agent's: a card synced again along with a later one,
after it (`[B, A]`), is now "already on drive" on A, where it was a duplicate
of B.
Why: the line must name the card whose file is on the drive, and a plan that
says otherwise must be refused, not reported.

## DEC-046  2026-10-30  duplicates wait until after every failure and the library (S039)
Tags: report, cli, tui
Decision: agent's pick in S039, under the user's standing instruction, not yet
confirmed by the user. Amends DEC-043 (1), which put them last only within
their own source.
(1) When a report has a line a list could bury (a file or entry that went
wrong on any card, any library section line, a refused or crashed sync), all
duplicates move after everything else, before the result line: "Duplicates in
session on <card>", then that card's lines. The tables come first too.
(2) With no such line, each duplicate stays in its card's section. Rejected:
always at the end. One layout and less code, but it means editing
the_report_shows_duplicates_and_names_the_card, which pins each file in its
card's section, and the user's rule is not to edit a test to make it pass. The
user may prefer it; then that test's two section asserts change with it.
(3) A line keeps its wording and does not name its own source: the heading
does, as the 80-column test pins the line, and the line already names the card
it is on the drive from. This is how the agent read "still names its source's
card" in S039.
(4) The step named sources, not syncs. In the TUI's session report the
duplicates of every sync also wait until after every sync, before "All syncs":
a backup card's sync must not bury a later sync's failure. The CLI prints each
report as its sync ends, so it cannot; its last lines, the session summary, name
each sync that was not complete.
(5) Tests that need a library read problem keep that library next to the test
binary, off the images: chmod does nothing on exFAT. An unreadable scan entry
cannot be made on the images at all, so only the unit test on `body` covers it.
Why: the prime rule; what went wrong must be read before a long list of photos
that are safe.

## DEC-047  2026-10-30  how tests see a real flush; root guard outside main (S030)
Tags: tests, safety, inv-3
Decision: agent's pick in S030, not yet confirmed by the user.
(1) Every INV-3 flush goes through one small function, `copy::flush`. Under
`cfg(test)` it notes what the handle holds (inode, size, or folder), read from
the handle after `sync_all` returned Ok. No feature, no public API: the unit
tests in `copy` are the only readers, so INV-8 and `no_test_seam` are not
touched, and a normal build keeps nothing. Rejected: the `test-support`
feature (integration tests do not need this, and workspace builds would link
it into the binary, DEC-027); a replaceable flush function (the test would no
longer run the real flush).
(2) A note cannot prove the flush in the same function ran. So
`inv3_a_flush_reaches_the_os` flushes a pipe, which the OS refuses: taking
`sync_all` out of `flush` makes that call succeed. Still not seen: whether
the disk kept the data, which needs a physical unplug (S017, optional).
(3) `main` is now `run(Cli::parse(), effective_uid())`, and `run` holds the
root refusal, so a test passes euid 0 without being root. Still a hand check:
that `main` passes the real uid. `effective_uid` is tested against `id -u`, and
`sudo photosync drives` stays in S017 and S025. Rejected: an environment
switch to fake the uid, which any caller could use.
(4) The session's own INV-10 check is reached in tests through a `Watch` at
the end of planning, once by really moving the library, once by the existing
`Fault::Device`. A real change of device needs a second volume, and at that
moment no test can swap one.
Why: M02, M03, M08, M13 and M16 passed the whole suite when removed (audit F05).

## DEC-048  2026-10-30  the end of a report counts the problem lines above the duplicates (S040)
Tags: report, cli, tui, tests
Decision: agent's pick in S040, not yet confirmed by the user.
(1) A count, not the problems repeated: the step said pick the smaller. One
line right above the result, "2 problem lines above the duplicates: read them.",
so `| tail` shows it. Rejected: repeating them; 500 cancelled lines would bury
the result the same way.
(2) A problem line is a line from `wrong_lines`, a library entry or a temp file
that could not be read or removed. Headings do not count. "Removed 2 temp files"
does not count: it still holds duplicates back (DEC-046 (1)) but nothing is
wrong, and a pointer to it would teach the user to skip the pointer.
(3) No line unless a duplicate is listed: without one, nothing sits between the
problems and the result.
(4) The rule covers every report, so a one-sync TUI report stays word for word
the CLI's and a report of several says it too; there a refused or crashed sync
counts as one line. The step asked only for the CLI.
(5) The CLI's summary after several syncs says "N problem lines in the reports
above: read them." It is not in the step's words, but a library problem leaves
its sync "complete" and a later card's report pushes it off the screen, which
DEC-046 (4) did not cover. Refused and crashed syncs do not count there: they
go to stderr and the summary names them.
(6) The step asked for a unit test on `session_report` with duplicates-only
syncs. A test in the binary cannot make one: only a real sync on two devices
makes a `Report`, and the seam is barred (DEC-027). So `session_report` now maps
each run to a `SyncPart`, whose sources a test can build, and `several_syncs`
holds the rendering and the flipped arm. The test gives real `Run`s for the
refused and crashed ends. Rejected: a core API to build a `Report` or a session
from parts, as a report must never be built from unchecked inputs.
Why: the prime rule; a problem must not go unread because a list of safe photos
pushed it off the screen.

## DEC-049  2026-10-30  the gate mounts its own images and runs the real syncs (S029)
Tags: tests, gate, rules
Decision: agent's pick in S029, not yet confirmed by the user.
(1) A test that needs real mounts is `#[ignore]`d. One plain test in each test
binary with such tests, `ignored_image_tests_pass_on_real_mounts` (macOS only),
mounts images in a temp dir and runs its own binary again with `--ignored`, with
the variables and TMPDIR set. It passes only if every test `--list --ignored`
names passed, and each wrapper also names the tests that must be in the list.
Rejected: nested `cargo test` (it would have to repeat the profile and flags,
and shares the build folder); images mounted lazily in the same process (a
static has no `Drop`, and setting TMPDIR for tests already running is unsafe);
any seam in the binary (DEC-027).
(2) Run without its variables, an image test now fails instead of returning.
So 16 tests can no longer pass with nothing checked, and `--include-ignored`
outside the images fails loudly.
(3) On macOS there is no opt-out: if `hdiutil` fails, the gate fails. Safest for
the prime rule; the cost is that `cargo test` cannot pass where disk images
cannot mount (a sandbox, for one).
(4) Images are never left attached: `Drop` on a panic; a watchdog `sh` on a kill
or Ctrl-C of the test binary, as nothing runs in-process then; each command has
a time limit, and past it its process group is killed. The limits: hdiutil
create and attach 120 s, detach 30 s then -force 30 s, a binary's image tests
300 s. They end a hang; a slow machine must not fail on them. A folder is removed
only when `mount` and `hdiutil info` show nothing left, the script's rule.
(5) Each binary mounts its own images, only those it needs: 15 attaches a run,
and 3 more by the helper's own tests in `tests/test_images.rs`, about 40 s added to
the gate. Rejected: one shared set, since `exfat_session` must not run beside
`exfat_e2e`, which counts the card's files, and sharing needs a fixture that
outlives a test binary.
(6) `scripts/test_exfat.sh` stays: it runs the same tests with `--include-ignored`
and the whole suite on exFAT, and HFS+. It leaves out the tests that mount their
own. It gets time limits by bash (macOS has no `timeout`): hdiutil 120 s, each
cargo run 1200 s with its build. Its HFS+ run now prints when it ends, not live,
so the cleanup can reach it.
(7) Linux: the visible skip is `#[ignore]`, and the equivalent is ext4 loop
mounts and `--ignored`, in the README and S025. Ext4 rather than exFAT: it needs
no extra tools and has permission bits. Not run yet: there is no Linux here.
(8) TESTS rule, proposed wording, for the user to take or change (the rule itself
is not edited): "TESTS: `cargo test --workspace` and `scripts/test_exfat.sh` are
green before a step is marked done (on Linux, where the script cannot run, also
the README's `--ignored` run on loop mounts); a defect gets a failing test before
its fix." Why both: the plain run has permission bits and the script has exFAT's
and HFS+'s behaviour, and each skips what the other needs.
Why: the prime rule; a step could be stamped done with the copy path, the
closed-stdout rule or the worst-outcome exit code broken (F04, M29).

## DEC-050  2026-10-30  a refused sync whose cards completed later stops counting; the TUI checks the library first (S032)
Tags: session, exit-codes, tui, tests
Decision: agent's pick under the user's standing instruction, not yet confirmed
by the user. Amends DEC-034 (2).
(1) A sync with `Verdict::Refused` (it wrote nothing) stops counting in
`Session::verdict` and `not_complete` once each of its cards completed later in
the session. It stays in `runs`, and `Run::verdict` still says `Refused`.
(2) "Completed later" means: a later run whose whole report `is_complete()` read
a card at the same `resolved_root`, listing every supported file the refused
sync listed, with the same path under the root, size and mtime. More files are
fine: a photo taken since. Why a listing and not the path: a swapped card comes
back at the same path (DEC-034 (1)), and a path alone would call one card's
sync the other's. No hash: a refused sync may not have hashed anything, and two
cards alike in all of these, at one path, are the backup slot's. No `st_dev`: a
re-seated card may get a new one, and only a false alarm would come of it.
(3) A whole report, not the card's own part of it: a twin's failure can land in
either card's bucket (INV-6), and such a run keeps the session not complete
anyway, so this only costs a name in a result that is already NOT COMPLETE.
(4) Never cleared, the safest reading: a card the refusal could not scan (gone,
no `DCIM`), or read only in part: nothing says what it held. So a card that was
not mounted, then mounted at that path, still ends in exit 2. Also never: a
recorded refusal (`record_refusal`, it read no card), a report refused after
copying and a crash (files may be on the drive, nothing says which), a failed or
cancelled sync, and a refusal after the complete sync, not before it. Each card
counts alone: A+B refused, A complete, B complete in another sync clears it. A
backup card with the same photos at another path does not stand in for A.
(5) Text: "sync 1 (A): refused, nothing copied; its cards completed later" in the
summary, and "Its cards completed later." under the refusal in the TUI's report,
so what the user saw is not left unexplained. With only some done: "; still to
sync: B". A cleared refusal still counts as a problem line in the TUI's report:
the line is there, and the count says what is above the duplicates (DEC-048).
(6) TUI: `App::start` refuses a library `fs::canonicalize` cannot find or read
before it makes the session, with the CLI's sentence; the session is not made, no
card is read, the folder is not made, and Enter then Esc, Esc go back to the
library. Exit stays 2 if the user quits there. Only this first check moves (`check_library_found`
in the core): every sync still does all of INV-10, as it needs the scans. The
offer to make the folder is left out: S032 excludes making it unasked.
(7) The check is on in `App::checking_library`, which `tui::run_loop` uses, and off
in `App::new`. Why: 16 unit tests of `app.rs` and `tui/view.rs` drive `App::new` on
made-up libraries such as `/V/SSD/Photos` and expect a job, which pins the old
order; S032 does not allow editing them. So they test the model without the check,
and `tui::tests::a_fresh_drive_is_refused_before_the_session_starts` tests the
real loop. Cost: a new front-end must pick `checking_library`. For the user: those
tests could move to real temp folders, and `App::new` could then always check.
Why: F07: on the expected first run, every photo on the drive still ended in "do
not format", which teaches the user to ignore that warning. The prime rule keeps
every case where a card cannot be shown to have completed.

## DEC-051  2026-10-30  the images' watchdog has time limits and says when it gives up (S041)
Tags: tests, gate
Decision: agent's pick in S041, not yet confirmed by the user. Makes DEC-049 (4)
true of the watchdog.
(1) Every command the watchdog runs has 30 s, by a watcher in `sh` as macOS has no
`timeout`: `hdiutil` as DEC-049 (4) has it, and `mount` too at no extra cost. Past
that its process tree is killed. Each of 5 mount points gets 30 s then -force 30 s
even after a hang, so a full hang ends in about 6 minutes, not sooner. Why: a detach
that hangs may be one image's; a later one can still work.
(2) It says so in `watchdog.log` in the temp folder, not on stderr: it can outlive
what reads the test binary's stderr, and holding that pipe would block `cargo test`.
Cost: nobody sees it unless they look in the folder, and `hdiutil info` shows what is
attached.
(3) The folder is removed only if `hdiutil info` and `mount` both show nothing left,
checked after the detaches. It used to look at `mount` only, so a failing `hdiutil info`
removed the file of an image still attached. A check that fails or hangs now keeps the
folder.
(4) `Images::mount` looks for `hdiutil` and `mount` on PATH before it makes anything. So
a run without them names them, with no "FAILED to detach" and no temp folder. An unset
PATH counts as both missing, though a spawn would search a default path: rare, and the
message says what it saw.
(5) The README's Linux block uses `rmdir` on lost+found, not `rm -rf`: if that folder is
not empty something is wrong, and it should fail loudly.
(6) `scripts/test_exfat.sh` skips the 3 new tests, like the others in `test_images.rs`.
That file is outside S041's `touches`, and the skip is needed.
Why: S029's fast check: the README and DEC-049 (4) claimed a limit on every hdiutil
call, and the watchdog had none.

## DEC-052  2026-10-30  TUI: cards changed after a refusal before the session are not remembered
Tags: tui, session
Decision: the agent's choice under the user's standing instruction, not yet
confirmed by the user. A refusal before the session starts (missing library,
bad name; DEC-050) is recorded nowhere. If the user goes back and unpicks a
card, the session knows nothing of it, and a later sync can end complete
without naming that card (S032 fast check).
Why: no card was read and no sentence said it was safe, so this equals
unpicking a card before starting. Remembering picks would warn about a card
the user dropped on purpose.

## DEC-053  2026-10-30  which sync names are refused, the same on every platform; the Folder line (S031)
Tags: layout, cli, tui, report
Decision: the agent's choice, not yet confirmed by the user. (1) The same
characters are refused on every platform and drive, as a drive moves between
computers: `/ \ : * ? " < > |`, control characters, Unicode noncharacters, and a
name ending in a space or a dot. Measured by hand on macOS images (mkdir, then a
file inside): APFS, exFAT, HFS+ and FAT32 all took every one of those characters,
`:` included, and the endings, so no drive measured needed the refusal; it is for
the other computer. On exFAT and HFS+ a noncharacter U+FFFE or U+FFFF comes back
renamed (`%EF%BF%BF`); APFS refuses noncharacters. Not measured: Linux exFAT, where an
agent's recollection of the kernel says it refuses the FAT family and drops a trailing
dot; Windows. Check in S025. Rejected: refusing per drive, as a trip could then
be named on one drive and not on its backup; only what a Mac refuses, as the name
would then fail or change on Linux. A folder an older run made with such a name is no
longer reused by name: the sync is refused, and with another name files already
synced are still skipped by hash.
(2) `<date> - <name>` over 255 bytes is refused, so a name over 242 bytes, measured:
255 fits on all four, 256 gives ENAMETOOLONG. The rule is in `Session::new`, as
`name_too_long_for_a_lookup_reuses_nothing_and_is_not_an_error` holds `sync_folder` to
"no such folder"; that test stays as it was. Every front-end goes through a session.
(3) A session also looks up `<library>/0000-00-00 - <name>` and refuses on
ENAMETOOLONG: HFS+ counts decomposed UTF-16, so 100 U+0390 (213 bytes in all) fit
APFS and exFAT, not HFS+. The lookup reads only. Rejected: a folder made and removed
to ask, as a sync writes nothing before it plans.
(4) What this still lets through: APFS refuses code points Unicode has not assigned
(U+0378 measured). Knowing them needs a table or a crate and goes stale. Such a name fails
each file, exit 1, NOT COMPLETE: late, but true. A library path too long with the session
folder in it is refused, exit 2, for the path (DEC-056). One that takes the folder but not
a file under it, or a sync with no name, still fails each file, exit 1 (measured).
(5) The report's Folder line reads the drive when it is rendered, through
`SyncFolder::there` (lstat, a real folder only): otherwise "none (not made, or no
longer there)", as a folder is fixed before it is made. "Unknown: cannot check" if
the lookup itself fails.
Why: refusing before a card is read costs a retyped name; a name that fails or changes
later costs a failed sync or a folder that is not what the report says.

## DEC-054  2026-10-30  stale temps: the pid in the name decides (S034)
Tags: safety, copy, filesystems
Decision: agent's pick in S034, not yet confirmed by the user.
(1) A cleanup deletes a temp only if the pid in its name is gone: `kill(pid, 0)`
answers ESRCH. Any other answer (EPERM, an unknown error) keeps it. libc is already
a dependency of the core. A zombie and a pid reused by another program count as
alive, which only leaves a temp for a later sync. Rejected: stating in the specs that
one library takes one process at a time, as nothing would enforce it.
(2) This process's own pid counts as gone. A session cleans before its first copy,
so its own copies are not running, and such a temp is from an earlier run: an
interrupted one in this process, or a dead process that had this pid. Kept, that temp
could also block the name of this run's own temp, as temps are created only if absent.
Rejected: a list of this process's temps in flight, as no front-end runs two sessions
on one library at once. If one ever does (a GUI), the second's cleanup takes the
first's live temp: that copy fails and is reported, no photo is lost.
(3) A name not of the form `<pid>-<n>-<name>`, with a pid above zero that fits an
`i32`, stays and is not reported: photosync never wrote it, so INV-2 gives no leave to
delete it. Only a pid above zero goes to `kill`, where zero and below mean groups.
Before S034 such a name was deleted.
(4) Kept temps are not reported. Nothing is wrong, and a temp in flight is
gone when its writer ends.
(5) Limits. A temp from another computer whose pid is alive here waits for a later
sync. A writer the pid check cannot see, such as another computer on a shared drive
or another pid namespace, looks gone, so its copy fails and is reported, as before
S034. A pid that ends and is reused between the check and the delete could cost that
copy the same way.
(6) No lock across processes: the test on exFAT shows none is needed for two
processes copying different names. Not covered: DEC-018's fallback check and rename,
where the lock is per process, so on exFAT two photosync processes writing one final
name in the same instant could replace one with the other. Parked for the user.
Closed by DEC-055.
(7) Test fixtures that named pids 1, 5, 12 and 77 now name the test's own pid. Those
pids may be alive, and then a test that a temp goes, or stays for another reason,
would fail or prove nothing. What the tests assert did not change.
Why: a pid is enough to tell a live temp from a stale one, needs no new state on the
drive (INV-9), and every doubt keeps the file.

## DEC-055  2026-10-30  the exFAT fallback holds a kernel lock on the folder, across processes (S043)
Tags: safety, copy, filesystems
Decision: agent's pick in S043, not yet confirmed by the user. Amends DEC-018.
(1) The race is real. On exFAT images, two photosync processes copying 40 photos of
one name each replaced photos and both said copied, in 3 of 3 runs, and more surely in
a test that holds the window open. DEC-054 (6) called it not reproduced.
(2) The fallback's check and rename hold `flock(LOCK_EX)` on the target folder itself.
Rejected: a lock file. It would be a new file on the drive (INV-9) that the scan and
the cleanup must learn to skip. With the folder, nothing is left to count or clean.
The per-process folder lock stays, first: DEC-008 needs it on every path.
(3) Only the fallback takes it. Where the no-replace rename works the OS already
refuses, so that path is unchanged and a held lock does not stop it.
(4) Measured on images on macOS, 2026-10-30: `flock` on a folder and on a file excludes
another process, and is dropped when its holder is killed, on exFAT, HFS+, FAT32 and
APFS. `renamex_np(RENAME_EXCL)` says ENOTSUP on exFAT only; on HFS+ and FAT32 it refuses
a taken name with EEXIST. So on a Mac only exFAT reaches the fallback. Linux not run.
(5) It polls `LOCK_NB` for at most 60 seconds, then fails that copy, reported as a
write error, and a later sync redoes it. Rejected: a blocking `flock`, as a holder
stopped by hand would hang the sync past any cancel. The limit is far above one check
and rename. Two processes on one name wait for each other only that long.
(6) A lock that cannot be taken, by any error, fails the copy and renames nothing,
rather than go unguarded. So a filesystem with neither a no-replace rename nor `flock`
on a folder fails every copy, which loses nothing. Rejected: refusing such a drive up
front, as no drive in scope is known to be one. From an agent's recollection, not a
run: Linux NFS may be one. Check Linux exFAT in S025.
(7) Limits. The lock binds photosync processes on one computer. Another program can
still create the name in the window, as DEC-018 accepts, and so can another computer
on a network share.
(8) Tests. Two real processes, each the test binary running a helper test, hold the
window open by a test-only hook: a fixed outcome, not chance. The hook is apart from
`Stage`, as tests list a copy's stages and they must read the same on every filesystem.
The spawn with its output thrown away is in `tests/common`, as the INV-8 test forbids
naming the standard streams under `src`. The test on the real binary depends on chance
for the window, and catches the race only because the pairs are many.
Why: the kernel drops the lock when a process dies, so a crash leaves nothing stuck, and
no new file goes on the drive. Every doubt fails a copy that a later sync can redo.
Amended by DEC-057: (5)'s limit stands, but a cancel ends the wait, and one timeout ends
the waiting for that folder for the run.

## DEC-056  2026-10-30  one sentence for the way out of a name collision; too long in any spelling has its own sentence (S033, S035)
Tags: report, cli, tui, layout
Decision: the agent's choice, not yet confirmed by the user. (1) With a collision among
the failures, the result says: "After a name collision, fix any other cause, then start
photosync again to sync under another name: files already on the drive are skipped." One
sentence for every front-end, as the name stays for the session in the TUI and with
`--one-by-one`, so "start again" is true everywhere and no front-end needs its own. It
replaces "run the same sync again" even with other failures, as another name also picks
those up. It stays short as the TUI's result box holds three rows: it fits at 80 columns
(a test holds this) unless the line has more to say, as a session's does. The session's
result says the same of its syncs. A failed duplicate of a collided file adds nothing: its
original is in the report. Rejected: a wording per front-end.
(2) A lookup's ENAMETOOLONG in `check_name_fits` is told apart by asking again with plain
ASCII of the same length: too long as well gives `FolderError::PathTooLong`, its own
sentence, and otherwise the name's own spelling is at fault (`InvalidName`, as an HFS+ name
is). Corrected in S035: too long as well does not prove the path is at fault. It is that, or
a drive with a shorter limit on one name (eCryptfs, 143 bytes: from general knowledge, not
run). The answer cannot tell them apart, so the sentence names both: "Use a shorter name"
works for either, and a library nearer the top only if the path is long. It reads only and
needs no unsafe code, and the kernel judges the same path, symlinks in it included.
Rejected: comparing with `PATH_MAX`, a second copy of the kernel's rule; `fstatat` on the
library, as it needs the library open and unsafe code.
(3) Measured on macOS: a library that takes the session folder but not `<TYPE>/<file>`
under it, or a sync with no name, is not refused: each file fails, exit 1, with the reason.
Refusing it would mean guessing the longest temp name. True, if late.
Why: a dead end costs a user whose counter wrapped a useless re-run, and a sentence about
242 bytes sends a user with a short name looking for the wrong thing.

## DEC-057  2026-10-30  a cancel ends the wait for the folder's lock; one timeout fails the folder for the run (S044)
Tags: safety, copy, filesystems, cancel
Decision: the agent's choice, not yet confirmed by the user. Amends DEC-055 (5), whose
60 second limit stays.
(1) The wait asks for a cancel, or a panic in another copy thread, on each poll, after
each try for the lock. With the lock free, a verified photo is renamed, as on a filesystem
with a no-replace rename, so a cancel at one stage gives one outcome on both. With the lock
busy, the rename does not run, the temp goes and the photo is cancelled.
(2) The per-process folder mutex stays held through the wait, first, as DEC-055 (2) says.
The wait a cancel can end is the only long one under it, and a cancel reaches every copy
thread of the run, so the thread behind the mutex is freed too. Rejected: letting go of the
mutex for the wait and taking it back. It changes the order of two locks on the path that
guards against overwriting, to gain nothing inside one run.
Limit: another `Session` in this process, on the same folder, has its own cancel and stays
behind the mutex for up to the minute. No front-end runs two at once (status.md, Parked).
Rejected: a mutex wait that polls for a cancel, as it would also change the no-replace path.
(3) One timeout on a folder is kept in a list for the copy run. The folder's other photos
fail with the same reason before the temp is made and the card read. A copy thread behind
the mutex, its photo verified, fails at once too, even if the lock has come free. Rejected:
one more try for the lock on each photo. It may copy a few more photos, but a rule of one
timeout is simple to test and to explain, and a later sync redoes them all. A taken name is
checked first, so it is still reported as a name collision.
(4) The list lasts one copy run, not a session or process: the holder may be gone by the
next card, and each folder waits its minute at most once in a run. Up to four type folders
can each take a minute. Only a lock timeout goes on the list: an ETIMEDOUT from the rename or
the open itself is a plain failure of that copy.
(5) Tests force the fallback and a short limit through a value the copy run takes on its
start (`RenameWay`), not a stage hook: copy threads then need no hook of their own, and the
real exFAT path runs the same code under `scripts/test_exfat.sh`. The holder is the helper
from S043, reached by a symlink as it locks a fixed folder name.
Why: a user's cancel must not wait a minute, and a card should not be read in full, one
photo a minute, only to fail the same way. Every failure here is one a later sync redoes.

## DEC-058  2026-10-30  the CLI's line: each source's bytes in its bracket, dropped whole by the cut; the TUI's line stays (S035)
Tags: progress, cli, tui
Decision: the agent's choice, not yet confirmed by the user. Adds to DEC-030 (8).
(1) With several sources the CLI's line ends `[CARD 1/3, 25.0 MB of 90.0 MB; CARD2 0/1,
0 B of 10.0 MB]`, in the card's own place, so a name is not said twice. One source gets no
bracket, as before: its bytes are the overall bytes.
(2) Over the width, every source's bytes go at once, back to `[CARD 1/3, CARD2 0/1]`, then
the end of that is cut as before. Rejected: a plain cut, which can leave `of 9` for `of 90.0
MB`; dropping one card's bytes at a time, as the point is to compare the cards.
", cancelled" stays over the bytes, as it says more. A log and an unknown width keep the
whole line (DEC-030 (8)).
(3) `progress_line` stays as it was and the TUI keeps using it: the TUI lists each source's
bytes under that line already. The step excludes TUI changes.
(4) Two existing tests changed their expected text, as this format is the change asked for:
`progress_of_two_cards_at_once_shows_each_card` and `two_cards_on_two_devices_copy_at_once`.
Each now holds more than before. The first takes all after the `[`, so it could stay only if
a log showed no bytes. The second could stay with the bytes after the bracket, which says
each name twice and so fits fewer terminals.
Why: which card is slow shows in bytes (F10), but a line that wraps cannot be rewritten in
place, so the bytes are what a narrow terminal can spare.

## DEC-059  2026-10-30  the harness stops a process before it kills its tree, and kills it last (S045)
Tags: tests, gate
Decision: the agent's choice in S045, not yet confirmed by the user. Adds to DEC-051 (1).
(1) `kill_tree`, in `tests/common/images.rs` and `scripts/test_exfat.sh`, sends SIGSTOP to a
process, then handles its children, then sends SIGKILL to it. So a process past its limit can
no longer end by itself once the kill began: it cannot see a helper die, nor start another. It
counts as 137, and `-force` is tried.
(2) Rejected: killing the parent first, the plain reading of the bug. Run with the two tests
of `test_images.rs` over 3 runs, it failed all: `{pid} lives on`. The parent's death wakes the
shell's `wait`, which kills the watcher before the watcher has killed the children. Rejected:
raising a time limit, which does not touch the race; a flag file saying the watcher fired, not
tried: it would put the status right, but a tool could still act on its helper's death, or
start a helper after the list was taken.
(3) `scripts/test_exfat.sh` had the same order and is outside S045's `touches`, as is its skip
list, which needs the new test. Fixed there too. Shown by hand with its own `with_limit`: a
tool that ends well when its helper dies gave status 0 before, 137 after. No test covers that
file's copy: the two functions could drift apart.
(4) Still not covered: a tool that ends by itself just before the limit keeps its own status,
which is right, and a process stuck in the kernel (state U) ignores SIGKILL (Parked).
Why: S045: the first detach counted as a success, and `-force` was skipped, in 2 of 6 gate runs,
then 1 of 4, then 1 of 6 with a trace in. The trace showed the fake's `wait` returning 0 between
the two kills, and `limited` seeing status 0.

## DEC-060  2026-10-31  the closed-stdout test proves its pipe has no reader; the binary was not at fault (S046)
Tags: tests, gate
Decision: the agent's choice in S046, not yet confirmed by the user.
(1) `closed_run` in `exfat_e2e.rs` takes its pipe from `closed_pipe`: drop the reader, write one byte,
and use the pipe only if that write gets EPIPE; a pipe whose write succeeds is thrown away for a new
one, at most 1000 times, then the test panics. Once a write fails, no process holds the read end and
none can get it, so a run on that pipe that reports no failed write is the binary's fault.
The assertions did not change.
(2) The defect was in the test. Its pipe could keep a reader: the binary imports `pipe`, no `pipe2`
(`nm`), and `lsof` showed a spawned child holding both ends of a pipe the parent had dropped the
reader of. std's source was not read (rust-src is not installed), so "made and marked in two calls"
is inferred from those two.
(3) Measured with the real binary's `sources` on a mounted image, 4 threads each making a pipe, dropping
the reader, writing one byte first, then running the binary (scratch program, not kept):
no spawners, 63084 runs; 64 threads spawning `sleep 0.1`, 244025 runs. A read end gone before the
spawn: 0 runs of 306045 missed it, all said "Cannot write the list". A read end alive before the spawn:
117 and 947 runs, of which 45 and 403 gave code 0 and an empty stderr, the rest had the holder end first.
With the pipes of (1), 64 spawners: 236390 runs, all noticed, 953 pipes thrown away.
`sync` by hand: a live reader that never reads gives code 0 and a stderr that stops after
`copying 1/1 files`, the failure seen in the gate; a reader gone gives the sentence.
On real mounts, the old helper under load, `sync`: 7512 runs, 0 missed.
(4) The binary is right: a write to a pipe with a live reader succeeds, and it cannot know that
reader will never read. No change to `main.rs`.
(5) Rejected: `pre_exec` making the pipe in the child, which also cannot leak, but takes `unsafe` and
moves this one run off the path the others spawn on, and proves nothing when it fails; a lock around
every spawn in the tests, which one forgotten spawn undoes.
(6) Not reproduced: the gate's own failure did not come back. The old helper with the probe of (3) added
ran 50 gates of `exfat_e2e`, then 30 with 20 CPU hogs, 240 pipes: none had a reader, none missed.
So 1 in 18 was the one event seen and its rate is not explained; the cause rests on the matching
signature and on (3), which leaves the race the only way to it found.
(7) `>&-`, stdout closed outright, is another case, measured by hand: `sources` exits 0 with its list lost and
nothing said, `sync` loses its report with nothing said, and copies, with the right code. Rust's start-up
giving such a fd `/dev/null` would explain it, from an agent's recollection, not read. Whoever closes
stdout that way asked for no output, and it would look like `>/dev/null`, so nothing changed (Parked).
Why: S046: 1 failure in 18 gate runs; a flaky gate hurts every later step, and a retry could have hidden a
real miss by the binary.

## DEC-061  2026-10-31  the TUI's report wraps by itself and scrolls by rows; the result's box grows (S037)
Tags: tui, report
Decision: the agent's choice in S037, not yet confirmed by the user.
(1) `view::wrapped` makes the rows, not ratatui's `Wrap`. Scrolling must know how many rows there are, and
ratatui gives that count only behind its `unstable-rendered-line-info` feature; a `Paragraph`'s scroll is
also a `u16`, so a report past 65535 rows would lose its end. It measures with ratatui's own graphemes and
widths, so no new dependency, and a wide character never runs past the edge. It breaks after a space, or
inside a word too long for a row. Nothing is dropped; continuation rows have no indent.
(2) `App`'s scroll counts rows, not lines. Rejected: lines, drawing from a line on, which is smaller, but
a line taller than the screen (a few rows, on a small terminal) could never show its end.
(3) Only a drawing knows the terminal's size, so `tui::draw_frame` tells `App` the rows and how many show
after each drawing (`report_drawn`), and the keys stop by these. The view stays a reader of the model.
(4) The scroll stops when the end reaches the bottom row, not when the last line is on the top row. A page
key moves 10 rows, or the screen's rows if fewer, so it never jumps over rows not shown.
(5) The result's box is as tall as its wrapped result, no longer always three rows, so a short one leaves
the report more room. This ends the limit DEC-056's sentence was kept short for; that sentence stays as is.
(6) A result that would leave under three rows of report has no box: it comes first in what scrolls, in its
colour, then a blank row. Rejected: a capped box, which cuts it; the box alone, which shows no report.
Why: the prime rule: what the user must read is never cut, even on a small terminal.

## DEC-062  2026-10-31  the TUI's other screens wrap and scroll as the report does; the copying screen's gauge goes on top (S047)
Tags: tui
Decision: the agent's choice in S047, not yet confirmed by the user.
(1) Beyond what the step named, every sentence on these screens goes through `view::wrapped`: list paths, the
progress and per-card lines, the hint and the keys row. The goal is that nothing is cut, and each could be.
(2) What is too tall scrolls, on every screen, not only a refusal: the step's accepts ask a small terminal only
of a refusal, but the check names every card and the rest say what went wrong. One `App::scroll` serves them all,
and it goes back to 0 when the screen changes. `report_drawn` keeps its name, now for whatever scrolls.
(3) On the two lists the arrows move the cursor, so only `PgUp PgDn` scroll, and only the notes: the list keeps
the three rows it needs, and `ratatui`'s `List` follows the cursor by itself.
(4) The keys row offers scrolling only when something is off the screen (`App::can_scroll`), so `keys_help`
returns a `String`. Rejected: always offering it, for keys that would do nothing on most screens.
(5) The copying screen changes order: gauge on top, then the progress line, the cancelling sentence, each card,
then the scan lines, all under it in one column. Before, the scans came first. Rejected: keeping that order, as
scans that scroll would push the gauge and the progress off a small screen. Under 5 rows of body there is no gauge.
(6) What still can be cut, on purpose: the title row, on a terminal under about 50 columns (it names the step, and
the screen says the rest); the keys row and hint on a terminal so low that the body's 3 rows leave them no room
(about 6 rows); a sentence inside a box under 3 columns wide; the empty list's sentence, in a box with one row
on such a low terminal; a list row taller than the list itself. Hint and empty-list sentence do not scroll.
Why: the prime rule: what the user must read is never cut, and the step's goal covers all these screens.

## DEC-063  2026-10-31  the TUI keeps the report's wrapped rows in the view, keyed by the outcome's number and the body's area (S048)
Tags: tui, report
Decision: the agent's choice in S048, not yet confirmed by the user.
(1) `view::ReportCache` holds the report's rows and is owned by `run_loop`, not `App`: what a width makes of the text
belongs to a terminal, and a GUI replaces the view and keeps the model. Rejected: a `RefCell` in `App`, which puts rows
in the model and makes a reader mutate it.
(2) It is kept while `App::outcome_id` and the body's `Rect` are the same. Every outcome goes through `App::set_outcome`,
which changes the number. Rejected: comparing the text, which needs a second copy of the report and walks it on every
drawing; comparing the strings' addresses, as a new report can land where the old one was.
(3) The whole `Rect` is the key, so a hint that changes the body's height wraps again though the width did not. Height
also decides whether the result has a box, and a hint on this screen is rare.
(4) Only the report is kept. The other screens still wrap on each draw, and more than once: they are short, and a
count test holds them to what they cost before S048. `wrapped` calls were counted with a `#[cfg(test)]` counter per thread.
(5) Measured, not estimated, 5000 lines of 165 bytes (826000 bytes), 80x24, `TestBackend`, on this Mac, scratch test not
kept. A draw after the first, before and after: release 40.5 ms to 0.16 ms, debug 701 ms to 1.9 ms; `wrapped` calls 10006 to 3.
At 200x50, where most lines fit a row: release 8.8 ms to 0.27 ms. The review's estimate is not used. The first draw, or a
resize, still wraps all: release 21 ms, debug 241 ms. A real terminal's own cost is not in these numbers.
(6) `wrapped` measures with `Span::raw`, no `Line` per grapheme. Same width, as a test holds. Its gain on the whole report
in release is small, about 20 ms a wrap before and 21 ms after with the first draw's cold memory in it; the refused
screen, which has no cache, went from 448 µs to 293 µs a draw. The cache is what mattered.
Why: S048: at one key per draw, 40 ms a draw puts every key of a large report behind the wrapping; a held key's queue was reasoned, not run.

## DEC-064  2026-10-31  notes leave the list what it must show: the row under the cursor, or the empty list's sentence, boxless when too low (S049)
Tags: tui
Decision: the agent's choice in S049, not yet confirmed by the user. It narrows DEC-062 (3) and (6).
(1) `list_areas` reserves the box and the rows of the row under the cursor, not of the tallest row. The list follows
the cursor, so every row can be read whole, and the box may change height as the cursor moves. Rejected: the tallest
row, for a box that holds still: it wraps every path twice more per draw, over the bound S048 pinned.
(2) When the body cannot hold both, the notes keep one row, as they scroll and so are read to their end: a list row
may be cut on a body of 4 rows with a row of 2, and so on (DEC-062 (6)). Rejected: the list taking every row, which
leaves a warning that can never be read.
(3) An empty list too low for its box and one row of notes has no box: its sentence comes first in what scrolls, then
the notes, as a result too tall for its box does (DEC-061). So the empty-list exception of DEC-062 (6) is closed,
and it does scroll now, unlike what that entry says.
(4) One exception DEC-062 (6) did not name stays: with a body of exactly 3 rows (a terminal of about 6) and cards in
the list, the notes get no row, and so cannot be read there; the list takes all three. Named in the specs. Rejected:
dropping the list's box there, for a terminal this low.
(5) `App::cancel` goes back to the top of what scrolls, as only the top shows the sentence. On a body so low that the
progress line and the sentence do not fit, the start of the sentence shows and the rest scrolls.
(6) The progress line scrolls with the scans; only the gauge stays in sight (DEC-062 (5)). So S047's test comment
was wrong, not the code: it is corrected, and `scrolled_scans_leave_the_gauge_in_sight` holds what is true.
Why: the prime rule: what the user must read is never cut, and a warning must not cost them the sentence that says what to do.

## DEC-065  2026-10-31  the GUI is iced 0.14.0, software drawing only, its own font, one theme (S036)
Tags: gui, dependencies
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. Confirms DEC-011.
(1) iced 0.14.0, the newest on crates.io today, pinned `=0.14.0`, as the test crate of DEC-066 must match it. It needs Rust
1.88; this Mac has 1.99.1. Approved in DEC-006. egui, the other candidate of DEC-002, was not tried: nothing here showed
iced unfit. Tauri, Electron and web views stay rejected by the user.
(2) `default-features = false`, features `tiny-skia`, `crisp`, `web-colors`, `thread-pool`, `lazy`, `fira-sans`; on Linux also
`x11`, `wayland`, without which iced stops the build. Rejected: the default wgpu renderer: twice the size, more crates and
a compile warning, for a window of words and bars. The renderer the tests draw with is then the one that ships.
(3) Measured on this Mac (arm64, macOS 27.0.1, rustc 1.99.1), clean release builds of a small counter app, each with its own
target folder:
    default features: 28.9 s, 10922752 bytes, 383 crates in its lock, warning that `block v0.1.6` will be rejected by a future Rust.
    tiny-skia, thread-pool: 16.1 s, 5045168 bytes, 252 in its lock, no warning.
    tiny-skia, smol: 18.7 s, 5313584 bytes, 270 in its lock. No gain: only `iced::time::every` needs smol or tokio, and DEC-068 needs no timer.
A real GUI crate over this repo's shared layer, features as in (2): clean release of all of it 19.3 s, binary 6371136 bytes
(5365368 stripped). A clean debug build of the whole workspace's tests 14.1 s, target folder 1.3 GB. Repo's `Cargo.lock` 111 to 325 entries, all
platforms counted; on this Mac the crates a build of it uses, by `cargo tree`, 76 to 180. The binary links only system frameworks (`otool -L`).
(4) The font is the Fira Sans iced carries (`fira-sans`, `.default_font(Font::with_name("Fira Sans"))`), as the test harness
draws with it. With it the real window and the headless picture were the same: 0 of 1920000 pixels differ, 800x600 at
scale 2. With the system font they differ, and text wraps elsewhere. Fira has no Japanese: a test label `写真カード` drew from a system font, on this Mac.
(5) One fixed theme, `Theme::Dark`, so that a checked picture is what ships. The system's theme can come later: low stakes.
Why: the smallest iced that draws these screens, and what is tested is what ships.

## DEC-066  2026-10-31  how a GUI screen is checked with no window and nobody looking; iced_test is allowed by the user (S036)
Tags: gui, tests, dependencies
Decision: (1) is the user's decision. All the rest is the agent's choice under the user's standing instruction, not yet confirmed by the user. Amended by DEC-071 and DEC-072.
(1) THE USER'S DECISION, 2026-10-31: yes, `iced_test = "=0.14.0"` may be a dev-dependency of `photosync-gui`. Any other new direct dependency still needs asking. It is the iced project's own test
crate, not part of what DEC-006 approved. It buys: clicking, typing and finding words on a screen with no window, a picture of
it, and the real iced runtime with no window. It costs: 19 more lock entries, 18 more crates compiled, in test builds only, none in the
shipped binary (`png`, `sha2`, `nom`, `iced_selector`, ...). By hand: not possible through `iced` alone, which does not expose
the runtime's `UserInterface`; `iced_runtime` as a direct dependency would need the same asking and leave the harness to us.
(2) What works with no window, all run here. `iced_test::Simulator` over `view(&state)`: `find("exact words")`, `click`, `typewrite`,
`into_messages()`, then the test feeds those to `update`. A checkbox by its label (not used: a row is found by its words, DEC-072 (4)), a button by its label, a text field by its
placeholder or value. `find` matches a text's whole content, not a part. `Bounded::visible_bounds()` is `None` for text scrolled
out of a `scrollable` or off the window. `snapshot(&theme)` then `matches_image(path)` writes `<path>-tiny-skia.png` if none is there, and
compares if one is: 5.9 ms a picture. An agent reads the PNG with its Read tool.
(3) `iced_test::Emulator` runs the real program, tasks and subscriptions, real threads included, with no window. Measured: 200000
worker events through `Task::run` in 27 ms; and on this repo's shared layer, a real sync on two disk images, every screen by
its clicks, from the cards to the report, exit 0, photos on the image, source manifest unchanged, in 7.8 s with the mounting. The loop:
    let program = iced::application(boot, update, view).subscription(subscription);
    let (tx, mut rx) = mpsc::channel(1); // iced_test::futures::futures::channel::mpsc
    let mut emu = Emulator::new(tx, &program, Mode::Zen, Size::new(800.0, 600.0));
    loop { match block_on(rx.next()).unwrap() {
        Event::Action(a) => emu.perform(&program, a),
        Event::Failed(i) => panic!("{i}"),
        Event::Ready => match lines.next() {
            Some(line) => emu.run(&program, Instruction::parse(&line).unwrap()),
            None => break } } }
Lines read `click "Next"`, `type "Tirana"`, `type enter`, `expect "Report"`. `expect` passes for text out of sight, `click` does not (DEC-072 (5)). `emu.into_state()` then gives the state to check. In `Mode::Zen` the
next line waits for all tasks, so `click "Start copying"` returns when the sync has ended. A cancel in mid-sync calls
`emu.update(&program, msg)` between events instead (DEC-072 (6) holds the copy thread for it).
(4) What it cannot do. The Emulator never fires `window::close_requests`, and ignores exit, clipboard and system actions. So closing is tested from the
message on, and `exit_on_close_request(false)` and the subscription by the source check of (7); the real close button needs the user (S060). A
wrapped word: `text` with the default `Wrapping::Word` cuts a word longer than a row, and `bounds` and `visible_bounds` report nothing wrong
(a 309-byte path in a 400-wide window: width 400, height 41.6, its end gone in the picture). So no bounds check can prove a line is whole; (7) makes every
text wrap by `WordOrGlyph`, which showed it whole. Nothing runs on Linux from here.
(5) A SCREEN CHECK, as every GUI step's `accepts` names it: (a) a test through the Simulator, at 800x600 and at the window's least
size, 640x480, finds each sentence the screen must say by its whole words, with `visible_bounds` not `None`, or for text in a
`scrollable` found and no wider than the window, but never so for a warning, which is in the fixed part (DEC-071), and clicks each button the step adds; (b) the source check of (7) passes;
(c) the test writes `target/gui_screens/<step>_<screen>.png`, removing an old one first, as `matches_image` compares otherwise. The agent
opens every such picture with its Read tool, and the `done:` stamp names each and says what it saw. No picture is kept in the repo: a
stored picture would break on any change of look, and the words are what must hold. So no step can claim a screen unseen.
(6) A real window works from an agent's shell on this Mac, and from an `.app` by `open -n --stderr <file> --env K=V <app>`: `window::screenshot(id)`
gave real pixels, with no permission asked. It shows on the user's screen, so it is never in the gate: an `#[ignore]`d test in a test binary of its own (DEC-072 (1)), run by hand or by a
step. `osascript` to `System Events` to press the close button hung past 8 s here, with no accessibility permission: no agent can press it.
(7) `crates/photosync-gui/tests/gui_split.rs`, a source check as `ui_split.rs` is: no file under `crates/photosync/src` names iced;
no file of the GUI names ratatui, crossterm, `copy_plan`, `plan_sync`, `set_device_for_test` or `test-support`; `checking_library` is named in non-test code and `App::new` is not (DEC-050 (7));
`exit_on_close_request(false)` and `close_requests` are named; iced's `text` is called only by one helper that sets `Wrapping::WordOrGlyph`, and no other form draws words past it (DEC-072 (4)). It says what it found, and is not vacuous: it also finds the known files.
Why: the prime rule covers what the GUI says, and this is how to look at it without a person.

## DEC-067  2026-10-31  the GUI is its own crate; the shared layer gets a library target and moves nowhere (S036)
Tags: gui, architecture, tests
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. Amends DEC-030 (1).
(1) New crate `crates/photosync-gui`, binary `photosync-gui` over a library `photosync_gui` (DEC-072 (1)), the only package that depends on iced. CLI and TUI stay out of it.
(2) `crates/photosync` gains a library target beside its binary: `[lib] name = "photosync_front"`, `path = "src/lib.rs"`, `test = false`, `doctest = false`,
and `[[bin]] name = "photosync"`, `path = "src/main.rs"`. `src/lib.rs` holds only `#![allow(dead_code)]`, `#![allow(clippy::new_without_default)]` and
`pub mod app; pub mod args; pub mod flow; pub mod system; pub mod text; pub mod worker;`. `main.rs` keeps its own `mod` lines, so each of those files
compiles twice: in the binary, with its tests, as today; and in the library, with none. No file moves and no file but `Cargo.toml` changes.
(3) Proved on a scratch copy, with a GUI crate in it: `cargo test --workspace` gave all of today's tests, 372 passed, plus the scratch GUI's one;
`scripts/test_exfat.sh` exit 0; `diff` of `crates/photosync/tests`, `src` and the script: only `lib.rs` is new.
`tests/ui_split.rs` walks `lib.rs` too, and passes. Only clippy on the library has one more thing to say: `StderrWatch` has no `Default`, as it is public there. Hence the allow.
(4) Rejected: moving the files to a shared crate. `ui_split.rs` looks for `src/app.rs` and fails; the `app`, `text` and `tui` tests must run in one
test binary, as `ignored_image_tests_pass_on_real_mounts` lists them; `#[cfg(test)]` helpers such as `App::with_outcome_for_test` stop being
seen across crates. The suite could not stay unchanged, and it is the proof. Rejected: `main.rs` using the library in place of its `mod` lines,
for the same two reasons. Rejected: the GUI in the `photosync` binary, which DEC-030 (1) allowed: every CLI build carries iced, and a `.app`
has no way to pick it over the no-terminal usage error. Rejected: `#[path]` from the GUI crate into `crates/photosync/src`: what `ui_split.rs` forbids in spirit.
(5) What it costs. The shared files compile twice. The library hides what `#[cfg(test)]` adds, so GUI tests reach states by real messages, and a report with
every kind of failure is tested on a pure function of the view (S055). A later tidy could make `main.rs` use the library; the suite would have to change, so it is the user's call.
(6) The GUI's own tests are unit tests of its library (DEC-072 (1)), one test binary as the TUI's are, with `#[cfg(test)] #[path = "../../photosync/tests/common/mod.rs"] mod common;`. That compiled and ran here, with `libc` as a dev-dependency (approved, DEC-008).
Why: the least change that lets the existing suite stay byte for byte the proof.

## DEC-068  2026-10-31  the GUI drives the model by its keys, and hears the worker by waking and draining (S036)
Tags: gui, architecture, session, concurrency
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. Amended by DEC-071 and DEC-073.
(1) One `App::checking_library()` per run, so the library check stays (DEC-050 (7)), so one session (the Parked note on two sessions in one process), and a `Session` through `worker::spawn` (DEC-040 (9)), never `copy_plan`.
(2) No rule in the GUI. Each widget sends what the TUI's key does, and `app.rs` needs no new message:
    cards: tick a row = cursor to it, then `Key::Char(' ')`. Next = `Enter`. Look again = `Char('r')`. Quit = `Char('q')`. As in the TUI, Next with none ticked takes the row under the cursor, so that row is drawn highlighted.
    drives: click a row = cursor to it. Next = `Enter`. Back = `Esc`. Look again, Quit as above.
    library, name: `text_input` `on_input` = `Backspace` for each old char, then `Char` for each new one, so `edit` keeps deciding; `on_submit` and Next = `Enter`. Back = `Esc`.
    check: Start copying = `Enter`. Back = `Esc`. Quit = `Char('q')`.
    copying: Cancel = `Char('c')`. No Quit button.
    refused: Back, to fix it = `Enter`. Next card = `Char('n')`, shown only `in_session()`. Quit = `Char('q')`.
    report: Next card = `Char('n')`. Quit = `Char('q')`. Back on the cards is not shown in a session; it is shown nowhere on the cards before one, as the TUI's Esc there quits.
    Scrolling is iced's `scrollable`: `App`'s scroll and `report_drawn` are not used, and the arrow and page keys are not sent. So `Gui` sends each scrollable to its top on a change of screen or outcome (DEC-073 (1)). A row is a `button` (DEC-072 (4)).
(3) A hint in `App` names TUI keys: "(Esc)", "press r", "press q". Rather than change `app.rs`, the GUI's helper swaps those words for its buttons' names. A test makes each hint reachable by clicks and asserts no key name is left, so a changed hint fails.
(4) `Msg` is not `Clone`, and `button::on_press` needs it (the compiler says so), so the GUI has its own `Clone` message `M`, and worker messages never go in it:
    worker::spawn(job, move |msg| { let _ = tx.send(msg); if !pending.swap(true, AcqRel) { let _ = wake.unbounded_send(()); } });
    Task::run(woken, |()| M::Wake).chain(Task::done(M::Gone))
`tx` is `std::sync::mpsc`, `wake` is `iced::futures::channel::mpsc::unbounded`, `pending` an `Arc<AtomicBool>`. `M::Wake` clears `pending` first, then drains `try_recv` into `App::update`. The send never blocks, which it must not: it runs under the core's progress lock, on a copy thread (DEC-040 (3)). When the worker's thread ends the wake's sender drops and the stream ends, so `M::Gone` drains and sends `Msg::WorkerGone` on `Disconnected`, as the TUI's loop does, panic or not. Measured with 4 threads and 200000 events in a real window: 25 ms and 18 redraws. One `M` per event instead: 13.9 s and 3112 redraws. `Progress` events are whole snapshots, so keeping only what was last drained loses nothing, and the view draws `Progress::sources` and assumes no one file in flight (DEC-040 (9)).
(5) Discovery runs off the UI thread, as a stuck mount must not freeze the window: its own `std::thread`, its numbered `Volumes` through a channel like the worker's (`Volumes` is not `Clone`), and a looking state while the newest is out (DEC-073 (2)). The discover function is injected, as `tui::run_loop`'s is, so tests give literal folders. A hidden `--search-root` as the CLI's reaches it from the binary.
(6) The report: `lazy(app.outcome_id(), ..)` around a `scrollable` of one `text` per line, in `Font::MONOSPACE`, or the report's tables do not line up, as the picture showed. One `text` per line, kept by `lazy`, measured on a kept interface: 50000 lines, scroll and draw 8.4 ms; made again only when the report changes, 516 ms. One `text` holding the whole report: 10000 lines 208 ms, 50000 lines 1.03 s, a scroll, and with no `lazy` that layout comes again on every redraw (one `M` per event, no `lazy`, a 10000-line report in the window: a flood did not end in 60 s).
(7) One window, one screen at a time: the title, the buttons and the warnings of DEC-071 stay in sight, all else is in a `scrollable`. Default 900x650, least 640x480 (`window::Settings::min_size`). Words come from `text` and the model; words the TUI's view holds (`confirm_rows`, `field_help`, `keys_help`) are the GUI view's to say, with no key named.
(8) Left out, each a new dependency or more than the TUI does, for the user to ask for: a native folder picker (`rfd`), a Copy report button, keyboard shortcuts beyond Enter in a field, watching for mounts instead of Look again.
Why: every rule stays in one tested model, so the GUI cannot drift from what the TUI does.

## DEC-069  2026-10-31  what stands in for exit codes; closing, quitting and root in the GUI; its name (S036)
Tags: gui, exit-codes, session, safety, naming
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. Amended by DEC-071.
(1) Name: the window's title and the GUI's text name the app "phoosync pro 2", as DEC-041, the user's call for the TUI's text, does there. Treated the same: the name the user sees, and the `.app`'s name in Finder and the Dock, are "phoosync pro 2"; the binary, crate and docs stay `photosync`, `photosync-gui`. Sentences from `text` that say "photosync" stay, as in the TUI.
(2) A GUI user sees no exit code. What stands in is the screen: the report's result (`text::session_result`) stands above the report in a box, green only when `complete`, else red; a refusal or crash shows its sentence and, after several syncs, the session's result, as the TUI's. The words are the same strings, so "do not format the card" is word for word as strong. `App::exit()` still decides, and the process prints its report to stdout, its message to stderr, and returns its code, as the TUI's does, for a terminal and for tests. It is no promise to the user. If no `Exit` was recorded by the time `run()` returns, the code is 1 and the message says the window closed with no report.
(3) No way out hides a session not complete. A quit (button or close) with `App::exit().code != 0` from any screen but the report and the refused screen, which already say it, goes first to a "Before you quit" screen: the words of DEC-071 (2), with Quit anyway and Back. With code 0, or from those two screens, it quits at once. A close on "Before you quit" is Quit anyway. This is the GUI's only screen outside the model. Cmd-Q and the Dock's Quit are the one exception: the banner of DEC-071 (2) warns before them.
(4) Close during a sync: `exit_on_close_request(false)`, and `window::close_requests()` sends `Key::CtrlC`. The first close cancels: the window stays, the Cancelling sentence shows, and the report follows. Once cancelling, the sentence says: "Close the window again to quit at once: not every file will be known to be on the drive. Do not format the card." A second close then quits at once, as the CLI's second Ctrl-C (INV-7); the warning was on screen before it, in the fixed part (DEC-071 (1)). The message of `App::exit()` still goes to stderr.
(5) The copying screen always says "Do not format the card until the report says so.", in its fixed part (DEC-071 (1)). Why: (6).
(6) Cmd-Q and Quit in the Dock cannot be stopped. Measured, on an idle spike app: an Apple event `quit` to the bundle ended the process, with no close request. winit's Quit menu item is `terminate:`, with no hook. To stop it needs an AppKit delegate, so `objc2` as a direct dependency: not added. So those end like a kill: INV-7 keeps the drive safe, no report, nothing said to be safe. On the other screens the banner of DEC-071 (2) stands before it. For the user to take or leave.
(7) Root: before any window or discovery, `main` asks `system::refuses_user(system::effective_uid())`, through a function taking the euid, as a test reaches it (DEC-047). The CLI's sentence moves to a `pub const` in `system.rs`, with no change in what it says. It goes to stderr first, then a window shows only that sentence and Quit; exit code 2, also if no window can open. `default_search_roots` failing is the same with its own sentence. A window that cannot open says so on stderr: "Nothing was copied."
(8) A panic of the UI thread ends the process like a kill: same as (6). A panic of the worker is caught by `worker::spawn` and shown as the crash sentence, not complete, as in the TUI.
Why: each errs toward "not safe to format", and none says more than the TUI does.

## DEC-070  2026-10-31  packaging: an ad hoc signed .app and an AppImage, each its own step (S036)
Tags: gui, packaging
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user.
(1) Mac: a script makes `target/package/phoosync pro 2.app` and a `.zip` of it by `ditto -c -k --keepParent`, with only tools this Mac has: `plutil`, `codesign --sign -`, `ditto`. No new crate. Measured with a spike bundle: it runs from `open`, ad hoc signed by the linker, and draws. Such an app runs on this Mac. On another Mac, downloaded, Gatekeeper stops it until it is signed with a Developer ID and notarized: this Mac has 0 signing identities, so that waits for the user's Apple account and is left out.
(2) No icon: there is no art, and a generated one would be worth little. The user can give one.
(3) Linux: `cargo check` from this Mac on `aarch64-unknown-linux-gnu`, with a toolchain in a throwaway `RUSTUP_HOME`, so the user's stays as it is, passed for iced with these features in 14.5 s. So S059 can prove the GUI compiles for Linux here. Running it and the AppImage need a Linux machine (S061): `appimagetool` is a build tool there, not a crate.
Why: the app is usable on the user's own Mac now, and nothing needs an account or a machine before it must.

## DEC-071  2026-10-31  what must stay in sight in the GUI: the fixed part, the not-safe banner, boxes that cannot outgrow the window (S036, plan review)
Tags: gui, safety, exit-codes
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. From a review of the plan. Amends DEC-066 (5a), DEC-068 (7), DEC-069 (3), (5), (6).
(1) The copying screen's fixed part, outside the `scrollable`: the title, "Do not format the card until the report says so.", the Cancelling sentence and the second-close sentence of DEC-069 (4) once cancelling, the overall bar, Cancel. The per-card bars, the progress words and the scan lines scroll: the bars of 10 cards cannot fit 480 high. Why: found and no wider, as DEC-066 (5a) let pass, holds for a warning below the fold, and `App::cancel`'s `scroll = 0` reaches no iced scrollable (DEC-068 (2)). A fixed sentence is checked with `visible_bounds` not `None` at 640x480, 20 scan lines, 10 cards.
(2) A banner on cards, drive, library, name and check while `App::exit().code != 0`, in the fixed part, red: the line "Not every photo is known to be on the drive: do not format the card.", then a box with the words: `App::exit().message`, or, when that is `None`, the `result` of `App::outcome()`. A pure `banner(&Exit, Option<&Outcome>)` makes them; with code 0, none. The fallback is needed: `text::session_message` gives `None` for one sync with a report, failed or not, so the message alone would leave the commonest case, one failed sync then Next card, with nothing to show, here and on "Before you quit". The copying, report and refused screens say it themselves and have no banner. Below the line the reason is the session's own words. The line is new words, as a refusal's sentence may not say "do not format".
(3) The box of (2) is at most 104 high, five rows, and scrolls inside. The message names each sync not complete and can hold long paths, so it can outgrow the window. Rejected: the whole message in the fixed part as it is, which pushes the buttons off; the whole message in the body, which puts the reason below the rows on a long list.
(4) The result's box over the report is at most 240 high and scrolls inside; the report keeps the room under it. Both boxes are one helper: `container(scrollable(words)).max_height(h)`. Measured in a throwaway spike at 640x480, title over a box over a report over a button: text of 40 named syncs 499 high, visible 240, the first report line and the button in sight; a short text takes its own height; the wheel over the box reaches its end, and the report stays. Rejected: the TUI's rule (DEC-061 (6)), which has no box and puts a too-tall result first in what scrolls: to choose, the view must measure wrapped text before layout, which needs iced's `advanced` feature, and a wrong guess cuts or hides. Here nothing is measured, nothing is cut, and the report and buttons never leave. The refused sentence and "Before you quit" have no report under them, so their words are the body's `scrollable`, with the buttons fixed.
(5) Cmd-Q and the Dock's Quit are the one exception to DEC-069 (3). They cannot go to "Before you quit" (DEC-069 (6)). So the banner is what warns: it is on screen before such a quit, on every screen that does not say it itself. S060 checks it by hand.
Why: the prime rule is only kept by a warning the user can see, on the screens where a quit gives no second chance.

## DEC-072  2026-10-31  how the GUI's tests stay honest: the window test apart, rows the source check can read, a cancel that cannot miss (S036, plan review)
Tags: gui, tests
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. From a review of the plan. Amends DEC-066 (2), (3), (6), (7), DEC-067 (1), (6). No new dependency.
(1) The window test lives in its own test binary: `crates/photosync-gui/tests/smoke_window.rs`, `#[ignore]`d test `the_real_window_draws_what_the_headless_picture_draws`. Two things run ignored tests. `scripts/test_exfat.sh` runs `--include-ignored`, so it needs that exact name in its skip list once it adds `-p photosync-gui` (S057). And `run_ignored_tests` in `tests/common/images.rs` runs `<test binary> --ignored` with no filter and wants all it lists to pass, so an ignored window test beside the image tests would open a window in plain `cargo test`, which a skip list does not stop. A test file also gets `CARGO_BIN_EXE_photosync-gui`, which a unit test does not. It needs the view, and a binary shares none, so the crate is a library `photosync_gui` (`src/lib.rs`, all unit tests, `mod tests` at its root) and a thin binary `photosync-gui` (`src/main.rs`, `test = false`). `gui_split.rs` holds that every `#[ignore` under `src` has the images' reason, and, from S057, that the script's skip list and the test file both name that test.
(2) The smoke run gets `--search-root` with an empty folder wherever it runs, S058's `open` included: real `/Volumes` with a card mounted would differ from the headless picture, and no removable volume is touched, so no macOS prompt can cover the window. It takes its picture after the first discovery result (DEC-073 (2)). It ends by the same message as the Quit button, so one real run proves that road. `PHOTOSYNC_GUI_SMOKE_PNG=<file>` makes the test compare that file and run nothing, for S058.
(3) `print_report` is private to `crates/photosync/src/main.rs`, which the library `photosync_front` does not hold. So the GUI has its own pure `ending(&Exit, out, say) -> u8`, over `system::write_all`, with its own `a_report_that_cannot_be_written_keeps_the_exit_code`, as specs' closed-stdout rule covers it too. Moving `print_report` would change `main.rs` for nothing.
(4) `Checkbox::label` draws with its own wrapping, `Wrapping::Word`, past the one `text` helper, and a word longer than a row is cut with nothing wrong in bounds (DEC-066 (4)). So a list row is one `button` holding a bare `checkbox(picked)`, with no `on_toggle`, and the helper's words. Measured: a click on the words sends the button's message, as a checkbox with no `on_toggle` captures nothing; a `button` with no `on_press` sends none. A test finds a row by its words. Style makes the bare checkbox look enabled. The source check then rejects, in the GUI's non-test code: `.label(`, `text!`, `rich_text`, `Text::new`, `Text::from`, `span(`, `value(`, `".into()`, a string literal as the first thing in `button(`, `container(`, `center(`, `scrollable(`, `mouse_area(`, `column![`, `row![` or `.push(`, and the widgets that draw their own words: `radio`, `toggler`, `pick_list`, `combo_box`, `text_editor`, `tooltip`, `markdown`. iced's `text`, `button` and `checkbox` are called only in the one helpers' file. `text_input` stays: what is typed stays on one line, as in the TUI. What it cannot see: a `&str` variable given straight to a container, which `From<&str> for Element` allows. Only the picture catches that.
(5) `expect` in an Emulator line passes for text scrolled out of sight; only `click` fails on it, by `Event::Failed`. Measured. So a `play` that must prove words in sight clicks them. `emu.update` runs its task unwaited when nothing is pending, so a test performs the one action it sends before its next line. The result's or a box's end is proved by `find(selector::id(..))`: `Target::Scrollable`'s `translation.y + bounds.height >= content_bounds.height`.
(6) A cancel that cannot miss: `Gui` has a `#[cfg(test)]` `before_send: Option<Arc<dyn Fn(&Msg) + Send + Sync>>`, called on the copy thread before the real send, so a test can hold that thread. S057 (g) holds it on the first `FileStarted` in `Phase::Copying`, sets a flag first, calls `emu.update` with Cancel's message from its event loop once the flag is set, then lets it go. The copy cannot end before the cancel, so `cancelled >= 1` holds on a fast disk too. Measured with a thread standing in for the copy, in `Mode::Zen`: no deadlock, the update lands before the release. Rejected: big files and a race, which can end before the cancel; the job's `Cancel` set straight by the test, which skips the GUI's message.
Why: a check that can pass with the fault in place is worse than none, and these were the ways it could.

## DEC-073  2026-10-31  the GUI's scroll starts at the top of each screen; looking for volumes is a state of its own (S036, plan review)
Tags: gui, concurrency, session
Decision: the agent's choice under the user's standing instruction, not yet confirmed by the user. From a review of the plan. Amends DEC-068 (2), (5).
(1) `Tree::diff` keeps a widget's state when its tag matches, and a scrollable's offset is in that state. With one layout on each screen, a report scrolled down would leave the next screen or report opened scrolled, its top lines unseen. The TUI resets at each change. Rejected: `.id()` per screen and per `outcome_id`, as the review proposed: `Scrollable::diff` never looks at its id, and measured in a spike, a second screen with another id opened at the first's end. Rejected: one `keyed_column` child, as its diff with equal lengths keeps the old tree too (read, not run). Chosen: `Gui::update` compares `(app.screen(), app.outcome_id())` before and after each message, and on a change chains `operation::snap_to(id, RelativeOffset::START)` for the body, report, result and banner scrollables, each with one fixed `.id()`. Measured: with it the next screen opens at its top, through `lazy` too, 5 runs of 5. A fresh Simulator cannot see this, so an Emulator test does: a `#[cfg(test)]` `M::ScrollEndForTest`; before leaving, a click on the first words gives `Event::Failed`, which proves it scrolled; after the change, a click on the new screen's first words passes. The first words must be a `text` of their own, as a long one stays partly in sight.
(2) `Gui` counts its looks. Each `Effect::Discover` takes the next number, and its thread sends `(number, Volumes)`. A result with an older number is dropped, unread, so two Look agains arriving out of order keep the newest. While the newest is out, the list screens show only "Looking for cards and drives. A stuck card can hold this up: take it out, or press Look again.": no rows and no notes, as `next_card` keeps old `sources` and `Next` would take a stale row; no "No camera card found", which nothing has said yet; Next is held, by no `on_press`. Look again stays, as a stuck mount never answers, and starts a newer look; Back and Quit stay. No look can be out past the list screens, as only they have Look again and Next is held. A thread stuck on a mount stays stuck until the process ends: it holds nothing the app needs. `app.rs` does not change: the state is `Gui`'s, and the model gets `Msg::Volumes` only for the newest.
Why: the screen says only what is known, and a stale row never starts a sync.

## DEC-074  2026-10-31  small choices of the GUI's first step: screens to come, arguments, smoke timing, harness picture (S051)
Tags: gui, tests
Decision: the agent's choice, not yet confirmed by the user.
(1) A screen with no step done yet is a bare title with no button, as S052's `accepts` expects of Library. So after Next on the cards only a close of the window leaves. Rejected: a Quit there: `Quit` is `Char('q')`, which the Library and Name fields would type. `Effect::Start` panics (`unreachable!`) until S053: no widget reaches it, and dropping the job would leave the screen saying a copy runs.
(2) `photosync-gui` reads its arguments by hand, as clap is not among S051's named crates. Any other argument, or a flag with no value, is refused on stderr with exit 2. Rejected: ignoring them: `--smoke-png` spelt wrong would leave a window open with nobody to close it.
(3) `--smoke-png` waits for the first discovery result, then two `window::frames()`, then asks for `window::screenshot`, which gives the renderer's last drawing, not a new one: taken sooner it could be the looking screen. Frames are subscribed to only then, as each one's message makes the next. Measured: 0 pixels off, 3 runs of 3 here, at scale 2, 1.6 s a run. A mouse resting on a button changes the pixels, so that test can fail with nothing wrong: it says so. It is why the test is by hand or by a step that means to.
(4) `screen_check` checks words at 800x600 and 640x480 as DEC-066 (5a) says, and draws its picture at the window's 900x650, so the picture is what first opens. `picture_at` draws one at another size. `in_sight` is the stricter check for what may never scroll: `screen_check` alone lets text pass when it is in a `scrollable`.
(5) `update` holds Next and a row while looking, as well as the view: a click drawn before the look began can still arrive. A row is sent by its path, not its place, so a list that changed cannot tick another card. `Gui::quit` records the model's `Effect::Quit`, as a test cannot see iced's exit; S056's shared slot may replace it.
(6) The looking sentence of DEC-073 (2) says "press Look again": that names a button, not a key, so S052's check for `press` leaves that sentence out (S052 amended).
(7) Measured: clean release of `photosync-gui` 19.25 s, 5981152 bytes, 4712216 stripped, beside DEC-065 (3)'s 19.3 s, 6371136 and 5365368. `Cargo.lock` 111 to 387 entries, all platforms counted; the spike's lock with iced_test had 344: the rest come only with `x11` and `wayland` on Linux, which the spike's lock left out, read from the difference, not traced crate by crate. Not one version of the 111 changed.
Why: each is the least that keeps the screen honest, and a later step would ask why.

## DEC-075  2026-10-31  small choices of the cards and drive screens: stale clicks, scroll reset, the drive's mark, a box's end (S052)
Tags: gui, tests
Decision: the agent's choice, not yet confirmed by the user. Amends DEC-072 (5), DEC-073 (1).
(1) `update` drops a click left from the screen before. A row, Next, Look again, Quit and Back do nothing off the list screens, and Back only on the drive screen. Events of one frame go to one view, so a second message can land on the next screen: there a key would type in a field, and Esc on the cards would quit. Each later step opens this for its own screen. Rejected: a screen number in each message, as a human's second click comes frames later and only these harmful landings need a stop. So two Nexts in one frame would still take the first drive; nothing is copied before the check, which names the library.
(2) DEC-072 (5)'s test of a box's end, `translation.y + bounds.height >= content_bounds.height`, fails at the end: measured 790 + 104 of 894.4, as iced rounds a translation to a whole pixel. So it compares with `content_bounds.height.floor()`.
(3) DEC-073 (1) chains the scroll reset; here it is batched with the message's task, as a look's task can wait on a stuck card for ever, and a chained reset would wait with it.
(4) DEC-073 (1) compares screen and outcome; here looking too. The looking sentence has no room to scroll, and the rows after it would open wherever the old ones were left.
(5) The chosen drive (`app.drive()`) has the ticked box, the row under the cursor the grey bar, as the TUI's `(*)` and `>`. A click on a row only moves the bar: Next chooses, as Enter does there (DEC-068 (2)). A radio is left out, as it draws its own words (DEC-072 (4)).
(6) `hint_words` swaps only what these screens reach: "press r" for "click Look again", never "press Look again", so the check for `press` stays whole. S053 adds "(Esc)". The hint stands in the fixed part, over the buttons, in the theme's warning colour.
(7) In a session the cards' heading is the TUI's "Sync N: pick the next card".
(8) With a code not 0 and no words at all, the banner is its line alone, with no box.
(9) What the tests do not prove. The banner's own reset is sent but no test sees it: the Emulator has no wheel and no way to a session here. A chosen drive below the fold after Back, past about 25 drives, leaves its bar out of sight, as the body opens at its top. Neither is reached by a usual run.
Why: each keeps a click or a scroll from landing where the user did not mean it, and a later step would ask why.
