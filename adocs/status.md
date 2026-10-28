# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-28, by Claude agent (S022 in progress)

- Last done: S020  one card after another in the CLI and TUI (DEC-034)
- In progress: S022  session CLI/TUI hardening: prompt poll, cancel during
  lookup and the restored refused-report coverage done (DEC-035)
- Next: S022, once the user rules on its first accepts item
- Blocked:
  - S017 waits for the user's hardware (DEC-029)
  - S022's first item ("never waits when stdin is not a terminal") breaks
    three tests that feed cards through a pipe: `cli.rs`
    `one_by_one_takes_cards_until_q_and_sums_up_the_session` and
    `ctrl_c_at_the_prompt_ends_the_session_with_its_summary`, and
    `exfat_session.rs`. The user to choose: move them to a pseudo-terminal
    (libc, assertions kept), or a bounded wait on a pipe. The doc comment
    item follows that ruling.
- Parked:
  - S022's seam and prompt choices are the agent's pick, not yet
    confirmed by the user (DEC-035).
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
