# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-31, by Claude (S036 done)

- Last done: S036  plan the GUI: iced 0.14.0 confirmed by a spike, its own crate, steps S050 to S061 (DEC-065 to DEC-070)
- In progress: none
- Next: S050  shared layer gets a library target, no behaviour change
- Waiting for the user: a yes or no to iced_test as a dev-dependency (DEC-066 (1)): S051 and every GUI step after it wait for it, S050 does not. S060 (Mac by hand), S061 (Linux), S025 (Linux), S017 (Mac hardware)
- Blocked: none
- Parked:
  - Decisions the agents picked that the user has not confirmed yet:
    DEC-005 (folder date = earliest new file), DEC-026 to DEC-028,
    DEC-030 to DEC-032, DEC-034, DEC-035 (1)-(3), DEC-036 to DEC-040,
    DEC-042, DEC-043, DEC-044, DEC-045, DEC-046, DEC-047, DEC-048, DEC-049,
    DEC-050, DEC-051, DEC-052, DEC-053, DEC-054, DEC-055, DEC-056, DEC-057,
    DEC-058, DEC-059, DEC-060, DEC-061, DEC-062, DEC-063, DEC-064, DEC-065,
    DEC-066, DEC-067, DEC-068, DEC-069, DEC-070.
    Each says "not yet confirmed by the user" in decisions.md.
  - The GUI no longer waits for S025 (user, 2026-10-30: do all that needs no
    input or hardware). The audit re-runs after the GUI.
  - Planned GUI: Cmd-Q and the Dock's Quit cannot be stopped without `objc2` as a new direct dependency,
    so they end a copy like a kill, with no sentence (DEC-069 (6)). Left out, each for the user to ask for:
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
