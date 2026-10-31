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

Work stopped here on the user's word, 2026-10-31. See `handover_2026-10-31.md`.

1. S069  ignored files on a complete card (F01, Medium; asks the user first)
2. S074  a DCIM symlink cannot pass INV-10 (F06)
3. S071  accented file names are not copied twice (F03)
4. S070  session duplicates across Unicode forms (F02)
5. S072  GUI picture seam and flags (F04)
6. S073  keep the camera's modification time (F05; asks the user first)
7. S075  result counts and long temp names (F07)
8. S076  tests kill N38 and N34 (F08)
9. S060  GUI: hand checks on Mac (waits for the user and real hardware)
10. S061  GUI: run on Linux, AppImage (waits for the user and a Linux machine)
11. S025  first run on Linux: build, tests, hand checks (waits for the user)
12. S017  hardware hand checks on Mac (waits for the user)

Later, not yet steps: re-run the adversarial audit after S069 to S076. The
loop stops when a run reports no high and no medium; 2026-10-31 still has
one medium (F01).

## Done recently

Last five; the full record is `plan_done/`.

- S068  a launch with no arguments opens the window; what Finder cannot see is said
- S059  GUI: compile check for Linux from this Mac (script; nothing runs there)
- S058  GUI: Mac .app package, ad hoc signed
- S067  tests cannot delete outside images; GUI not-safe roads proved
- S066  double close, long paths, Cmd-Q code
