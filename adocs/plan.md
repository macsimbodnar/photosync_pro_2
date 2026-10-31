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

1. S051  GUI crate, window, test harness, cards list (waits for the user's yes to iced_test, DEC-066 (1))
2. S052  GUI: cards and drive screens
3. S053  GUI: library, name, check, refusals before a session
4. S054  GUI: copying: worker, progress per card and overall, ETA, cancel
5. S055  GUI: report, refused, scrolling, next card in one session
6. S056  GUI: closing, quitting, root, what the process returns
7. S057  GUI: real syncs by its clicks on disk images, in the gate
8. S058  GUI: Mac .app package (signing for other Macs needs the user, left out)
9. S059  GUI: compile check for Linux from this Mac
10. S060  GUI: hand checks on Mac (waits for the user and real hardware)
11. S061  GUI: run on Linux, AppImage (waits for the user and a Linux machine)
12. S025  first run on Linux: build, tests, hand checks (waits for the user)
13. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after the GUI, covering
it too; the audit loop stops when a run reports no high and no medium.

## Done recently

Last five; the full record is `plan_done/`.

- S050  shared layer gets a library target, no behaviour change
- S036  plan the GUI: iced confirmed, crate layout, steps S050 to S061
- S049  list sentence and Cancelling stay in sight (S047 fast check)
- S048  a large report stays quick to scroll (S037 fast check)
- S047  TUI's other screens cut lines (found in S037)
