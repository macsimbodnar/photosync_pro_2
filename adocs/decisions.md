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
folder or symlink of that name is never touched. A second photosync running
on the same library at that moment would lose its temp file, so that copy
fails; no photo is lost.
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
