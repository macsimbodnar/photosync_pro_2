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

1. S026  short read fails, never copied (F01)
2. S028  report shows duplicates and the card (F03)
3. S027  name reuse across Unicode forms (F02)
4. S030  tests kill the surviving mutants (F05)
5. S029  the gate runs a real sync (F04)
6. S032  refused then fixed is not NOT COMPLETE (F07)
7. S031  too long a name is refused (F06)
8. S034  cleanup leaves live temps (F09)
9. S033  collision result names the way out (F08)
10. S035  CLI bytes per source (F10)
11. S036  plan the GUI: confirm iced, layout, steps
12. S025  first run on Linux: build, tests, hand checks (waits for the user)
13. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after the GUI, covering
it too; the audit loop stops when a run reports no high and no medium.

## Done recently

Last five; the full record is `plan_done/`.

- S019  copy several cards at once, one thread per card (split from S010)
- S024  twins on another device; one-sync result names unread cards (S023 fast check)
- S023  twin-fallback reporting fixes (S021 fast check)
- S021  twin fallback on a read error, F07 (split from S010)
- S022  session CLI/TUI hardening (S020 fast check)
