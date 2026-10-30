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

1. S037  TUI report cuts long lines (found in S028)
2. S045  watchdog test fails now and then: a race (found in S035)
3. S036  plan the GUI: confirm iced, layout, steps
4. S025  first run on Linux: build, tests, hand checks (waits for the user)
5. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after the GUI, covering
it too; the audit loop stops when a run reports no high and no medium.

## Done recently

Last five; the full record is `plan_done/`.

- S035  CLI bytes per source (F10)
- S044  cancel is heard during the lock wait (S043 fast check)
- S033  collision result names the way out (F08)
- S043  no-replace fallback holds across processes (found in S034)
- S034  cleanup leaves live temps (F09)
