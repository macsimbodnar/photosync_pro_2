# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-28, by Claude agent (S010 first part, split proposed)

- Last done: S018  TUI hardening (S009 fast check)
- In progress: S010  multi-source and one-by-one session. First part
  committed: core `Session` (one card after another, memory of earlier
  cards, temp cleanup once, worst verdict), planned files with source,
  hash and twins, copy checked against the planning hash, INV-10 re-check
  per folder, renames under the folder lock, CLI report names library and
  folder. Split proposed (DEC-032): S019, S020, S021. Waits for the user:
  confirm the split; the two tests S020 and S021 name.
- Next: S019  copy several cards at once
- Blocked: S017 waits for the user's hardware (DEC-029)
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
