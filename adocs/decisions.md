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
