# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-28, by Claude (S002 fast check)

- Last done: S002  scan source DCIM, classify by type
- In progress: none
- Next: S011  scan: per-file errors instead of whole-source failure
- Blocked: none
- Parked:
  - The camera's PTP mode (USB without card-reader mode) is unsupported
    (DEC-004). Revisit if card-reader mode proves unreliable.
  - The folder date is the earliest new-file date: the agent's pick, not
    yet confirmed by the user (DEC-005).
