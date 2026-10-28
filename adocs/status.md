# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-28, by Claude (S021 fast check)

- Last done: S021  twin fallback on a read error, F07 (DEC-036)
- In progress: none
- Next: S023  twin-fallback reporting fixes (S021 fast check)
- Blocked: S017 waits for the user's hardware (DEC-029)
- Parked:
  - S021's fallback choices are the agent's pick, not yet confirmed by
    the user (DEC-036): read errors only fall back (not a changed file),
    a file copied from its twin is skipped and the sync complete (exit 0,
    "check that card"), its progress stays on the planned file's card.
  - S022's seam, prompt poll and lookup-cancel choices are the agent's
    pick, not yet confirmed by the user (DEC-035 (1)-(3)).
  - S020's CLI and TUI session design is the agent's pick, not yet
    confirmed by the user (DEC-034): `--one-by-one` prompt, `n` next card,
    library and name fixed for the TUI session.
  - The camera's PTP mode (USB without card-reader mode) is unsupported
    (DEC-004). Revisit if card-reader mode proves unreliable.
  - The folder date is the earliest new-file date: the agent's pick, not
    yet confirmed by the user (DEC-005).
  - The permission-based tests assume a non-root user: run as root (e.g.
    Docker CI), most fail loudly, and `only_name_and_size_candidates_are_hashed`
    can pass falsely.
  - Linux exFAT (later Linux verification): `canonicalize` keeps the case it
    is given, so two library spellings that differ only in case get two
    fallback locks (S013 fast check). Fold the case or key by inode there.
  - A freshly formatted card without `DCIM` looks like any USB drive and is
    offered as a destination (S007 fast check). Telling them apart needs
    DiskArbitration (Mac) or udev (Linux).
  - 2026-10-28: macOS's disk-image service once hung attaching the third
    test image (also seen by the S020 agent). The helper was force-killed
    with the user's OK; a dead `PSLIB.dmg` entry stays in `hdiutil info`
    until reboot. New images attach and detach fine.
