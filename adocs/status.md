# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-31, by Claude (S066 done)

- Last done: S066  double close, long paths, Cmd-Q code (625 passed, 39 ignored; script 636)
- In progress: none
- Next: S058  GUI: Mac .app package (signing for other Macs needs the user, left out)
- Waiting for the user: S060 (Mac by hand), S061 (Linux), S025 (Linux), S017 (Mac hardware)
- Blocked: none
- Parked:
  - Decisions the agents picked that the user has not confirmed yet:
    DEC-005 (folder date = earliest new file), DEC-026 to DEC-028,
    DEC-030 to DEC-032, DEC-034, DEC-035 (1)-(3), DEC-036 to DEC-040,
    DEC-042, DEC-043, DEC-044, DEC-045, DEC-046, DEC-047, DEC-048, DEC-049,
    DEC-050, DEC-051, DEC-052, DEC-053, DEC-054, DEC-055, DEC-056, DEC-057,
    DEC-058, DEC-059, DEC-060, DEC-061, DEC-062, DEC-063, DEC-064, DEC-065,
    DEC-066 (all but (1), the user's yes to iced_test), DEC-067, DEC-068, DEC-069, DEC-070,
    DEC-071, DEC-072, DEC-073, DEC-074, DEC-075, DEC-076, DEC-077, DEC-078, DEC-079, DEC-080, DEC-081, DEC-082, DEC-083, DEC-084, DEC-085.
    Each says "not yet confirmed by the user" in decisions.md.
  - The GUI's banner and refused screen give the shared refusal's CLI words, "`photosync drives` lists them", "run the sync again" (S062; S053 read them on the library, name, check and refused pictures and left them, as `text.rs` is outside its step and S055 wants the words byte for byte). For the user to take or leave.
  - A marked drive can stand below the fold when the drive screen opens with Next live, past 13 drives at 900x650 and 9 at 640x480, down to 9 and 4 under the banner (S053 measured it; DEC-075 (9) said about 25). Harm is bounded: Next takes the drive the user clicked, and the check names the library before any copy. For the user to take or leave.
  - Real syncs through the GUI are in the gate, on disk images and with no window (S057). Not for cards that cannot be copied again until a real window on a real card has been run (S060); the README says so.
  - `the_banner_holds_on_the_library_name_and_check_screens` depends on how long `$TMPDIR` is: a much longer one fails it, in plain `cargo test` too, as a line of the check's body opens below the fold at 640x480 under a banner. S066 found no banner or button moves, at 300 bytes either, so no screen changed, the test is left as it is, and the script's temp folder stays in `/tmp` for that reason (DEC-084 (12), DEC-085 (3), (4)). For the user to take or leave.
  - At 640x480 under a banner, a 300-byte card path leaves "Into the library:" cut at the fold with Start copying live, and a scrollbar beside it (S066's picture); many cards do the same at any length. A body line, so by S066's `accepts` no defect (DEC-085 (5)). For the user to take or leave.
  - Each close is timed when handled, as winit gives it no time: if the window stands still past 500 ms between two clicks, as the 3.5 s of a 50000-line report, the second can still quit (DEC-085 (2)). Not reproduced. For the user to take or leave.
  - What Cmd-Q returns in a copy may be 0, from an agent's memory; S060 measures it, and tries a real double click on close (DEC-085 (5)).
  - A window that cannot open says iced's words only, "the application window could not be created", as its `Error` prints no cause (DEC-083 (6)). For the user to take or leave.
  - A report line that is not all ASCII is in Fira Sans, not the fixed font, as the fixed font's fallback draws kanji as nothing (DEC-082 (3), departing from S055's `accepts`). Tables stay fixed. For the user to take or leave.
  - The window stands still about 3.5 s when a report of 50000 lines arrives (release, measured; 0.35 s at 5000), 7 times DEC-068 (6)'s number, cause not looked for (DEC-082 (6)). For the user to take or leave.
  - The GUI no longer waits for S025 (user, 2026-10-30: do all that needs no
    input or hardware). The audit re-runs after the GUI.
  - Planned GUI: Cmd-Q and the Dock's Quit cannot be stopped without `objc2` as a new direct dependency,
    so they end a copy like a kill, with no sentence at the time, and no "Before you quit" on any screen: a fixed warning stands before them (DEC-069 (6), DEC-071 (5)). Left out, each for the user to ask for:
    a folder picker (`rfd`), Copy report, more shortcuts, watching for mounts (DEC-068 (8)); Developer ID signing
    and an icon (DEC-070). Spike's scratch not kept: its numbers are in those entries.
  - The camera's PTP mode (USB without card-reader mode) is unsupported
    (DEC-004). Revisit if card-reader mode proves unreliable.
  - The permission-based tests assume a non-root user: run as root (e.g.
    Docker CI), most fail loudly, and `only_name_and_size_candidates_are_hashed`
    can pass falsely.
  - Linux exFAT: `canonicalize` keeps the case it is given, so two library
    spellings that differ only in case get two fallback locks (S013 fast
    check). Fold the case or key by inode there; check in S025. DEC-055's `flock`
    is on the folder itself, so it may cover this already: an agent's guess, not run.
  - Linux exFAT: what DEC-044 says of it comes from an agent's recollection of the kernel's fs/exfat (source not read), not
    a run. Check in S025: a folder a Mac made and an accented name typed on
    Linux are expected to give two folders.
  - Linux exFAT: DEC-053's refused names rest on an agent's recollection of the
    kernel, not a run. Check in S025 which names it refuses or changes, and
    that the lookup behind the length check answers there as on a Mac.
  - A freshly formatted card without `DCIM` looks like any USB drive and is
    offered as a destination (S007 fast check). Telling them apart needs
    DiskArbitration (Mac) or udev (Linux).
  - Linux exFAT: DEC-055's `flock` on a folder was measured on macOS only. Check
    in S025 that two processes exclude each other there. On a Linux drive with
    neither a no-replace rename nor that lock (NFS, from an agent's recollection,
    not a run), every copy fails and is reported.
  - Two sessions on one library in one process at once would still take each
    other's temps: that copy fails and is reported (DEC-054 (2)). No front-end
    does this; a GUI must not, or needs a list of temps in flight. On exFAT, one's
    cancel could also wait up to a minute behind the other's lock wait (DEC-057 (2)).
  - `copy_plan` (public, one plan) still copies on one thread; only
    `Session` copies cards at once (DEC-040 (9)). A GUI must use `Session`.
  - CLI progress measures width in chars, not columns: a wide-character card
    label (`写真カード`) wraps the line and `\r` leaves rows behind (S035 fast
    check, read only). A fix needs a width table or unicode-width (DEPS: ask).
  - `photosync sources >&-` (stdout closed outright, not a closed pipe) exits 0 with its list
    lost and nothing said; `sync` loses its report the same way, with the right code (S046, by
    hand). It looks like `>/dev/null`, so nothing changed (DEC-060 (7)). For the user to take or leave.
  - S046's window applies to every pipe a test makes on macOS, read only, not run: the images'
    watchdog hears its parent die by EOF on stdin, so a child holding that write end would delay
    it; `system.rs`'s test `pipe()` sets no close-on-exec at all, so its read can wait on a child.
    Only delays seen in the reading, no wrong result. Harness only.
  - Test watchdog: a process stuck in the kernel (state U) ignores SIGKILL, so
    `wait` in tests/common/images.rs and scripts/test_exfat.sh would block with
    nothing logged (S041 fast check, read only, not reproduced). Harness only.
  - macOS's disk-image service hung once on a test image (2026-10-28); the
    script now also detaches images attached but not mounted. Since S029
    every hdiutil call and test run has a time limit, in the script and in
    the images plain `cargo test` mounts (DEC-049 (4)), and since S041 in their
    watchdog too (DEC-051).
  - 16 unit tests of `app.rs` and `tui/view.rs` drive `App::new` on made-up
    libraries, so the TUI's library check is on only in `App::checking_library`
    (DEC-050 (7)). Moving them to real temp folders would let `App::new` check
    always. For the user to take or leave.
  - The TESTS rule still names only `cargo test --workspace`: DEC-049 (8)
    proposes wording that adds the script. For the user to take or change.
