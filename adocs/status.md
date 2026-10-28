# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-28, by Claude agent (S008)

- Last done: S016  enforce INV-10: e2e card test, read-only device, re-check before copy
- In progress: S008  CLI commands: code, tests and docs done (DEC-028, not
  yet confirmed); waiting for the user's hand checks on Mac (real X-T5 card
  in a reader, X-T5 over USB in card reader mode, root refusal)
- Next: S009  TUI
- Blocked: none
- Parked:
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
