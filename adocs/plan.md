# Plan

Photosync is a simple desktop app, Mac first and then Linux, that copies
photos from a Fujifilm X-T5 (SD cards or the camera over USB) to an external
drive the user picks. RAW and JPEG go to separate folders, optionally under a
named folder such as "Tirana trip". Photos already in the library are
skipped. The app shows progress with an ETA and a report at the end. No
existing tool covers this (`research.md`). Detail in `specs.md`. Plain Rust: a UI-free core
library with CLI and TUI front-ends first, a native GUI later (DEC-002).

Order lives here and nowhere else. Step detail lives in the step files under
`plan_todo/`, `plan_current/`, and `plan_done/`. Ids are allocated in creation
order and never renumbered, so reordering is a one-line edit to this list.
Every open step file appears as an entry under Open, and every Open entry has
a step file — keep both true by hand. An id named in prose is prose: it does
not change the order.

## Open

1. S007  volume discovery (Mac, then Linux)
2. S008  CLI commands
3. S009  TUI
4. S010  multi-source and one-by-one session

Later, not yet steps: GUI (iced), Mac .app and Linux AppImage packaging,
Linux verification.

## Done recently

Last five; the full record is `plan_done/`.

- S014  report hardening (S006 fast check)
- S006  sync report
- S005  progress events, ETA, cancel
- S013  copy engine hardening (S004 fast check)
- S012  exFAT: locked check + rename fallback
