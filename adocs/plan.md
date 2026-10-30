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

1. S033  collision result names the way out (F08)
2. S035  CLI bytes per source (F10)
3. S037  TUI report cuts long lines (found in S028)
4. S036  plan the GUI: confirm iced, layout, steps
5. S025  first run on Linux: build, tests, hand checks (waits for the user)
6. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after the GUI, covering
it too; the audit loop stops when a run reports no high and no medium.

## Done recently

Last five; the full record is `plan_done/`.

- S034  cleanup leaves live temps (F09)
- S031  too long a name is refused (F06)
- S042  refused screen offers only keys that work (S032 fast check)
- S041  image harness keeps its promises (S029 fast check)
- S032  refused then fixed is not NOT COMPLETE (F07)
