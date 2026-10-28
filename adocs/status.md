# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-28, by Claude (S013 fast check)

- Last done: S013  copy engine hardening (S004 fast check)
- In progress: none
- Next: S005  progress events, ETA, cancel
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
