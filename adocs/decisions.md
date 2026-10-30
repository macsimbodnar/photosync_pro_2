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
`crates/photosync`).
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
API for a typo; the refused sync would still count).
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
re-run. Linux exFAT, from the kernel's fs/exfat, NOT measured (check in S025):
lookups fold case with the upcase table and do not normalise, and names are
stored as given. So one form reuses its folder, as listed bytes match; a folder
a Mac made decomposed and a name typed composed on Linux are two names and two
folders. No photo is lost or copied twice: already-synced detection goes by file
name, size and hash, in any folder.
Why: only the filesystem knows which names it treats as one folder.
