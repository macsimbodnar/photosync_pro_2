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

1. S049  list sentence and Cancelling stay in sight (S047 fast check)
2. S036  plan the GUI: confirm iced, layout, steps
3. S025  first run on Linux: build, tests, hand checks (waits for the user)
4. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after the GUI, covering
it too; the audit loop stops when a run reports no high and no medium.

## Done recently

Last five; the full record is `plan_done/`.

- S048  a large report stays quick to scroll (S037 fast check)
- S047  TUI's other screens cut lines (found in S037)
- S037  TUI report cuts long lines (found in S028)
- S046  closed-stdout test fails now and then (found in S045)
- S045  a killed hdiutil never counts as ended well (found in S035)
