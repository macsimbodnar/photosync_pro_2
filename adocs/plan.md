# Plan

Photosync is a simple desktop app, Mac first and then Linux, that copies
photos from a Fujifilm X-T5 (SD cards or the camera over USB) to an external
drive the user picks. RAW and JPEG go to separate folders, optionally under a
named folder such as "Tirana trip". Photos already in the library are
skipped. The app shows progress with an ETA and a report at the end. No
existing tool covers this (`research.md`). Detail in `specs.md`. Plain Rust: a UI-free core
library with CLI and TUI front-ends first, then a native GUI in iced (DEC-002, DEC-065).

Order lives here and nowhere else. Step detail lives in the step files under
`plan_todo/`, `plan_current/`, and `plan_done/`. Ids are allocated in creation
order and never renumbered, so reordering is a one-line edit to this list.
Every open step file appears as an entry under Open, and every Open entry has
a step file — keep both true by hand. An id named in prose is prose: it does
not change the order.

## Open

1. S065  Start stays live, late keys stay in their field, Esc goes back (S053 fast check)
2. S055  GUI: report, refused, scrolling, next card in one session
3. S056  GUI: closing, quitting, root, what the process returns
4. S057  GUI: real syncs by its clicks on disk images, in the gate
5. S058  GUI: Mac .app package (signing for other Macs needs the user, left out)
6. S059  GUI: compile check for Linux from this Mac
7. S060  GUI: hand checks on Mac (waits for the user and real hardware)
8. S061  GUI: run on Linux, AppImage (waits for the user and a Linux machine)
9. S025  first run on Linux: build, tests, hand checks (waits for the user)
10. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after the GUI, covering
it too; the audit loop stops when a run reports no high and no medium.

## Done recently

Last five; the full record is `plan_done/`.

- S054  GUI: copying: worker, progress per card and overall, ETA, cancel
- S053  GUI: library, name, check, refusals before a session
- S064  harness tells fixed from scrolling
- S063  drive screen marks what Next will take
- S062  GUI harness checks fail when a thing is not seen
