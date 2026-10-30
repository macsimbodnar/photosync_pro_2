# Status

Convenience view, rewritten by hand at the end of any turn that changed plan
state. The filesystem beats this file: on disagreement, `plan_current/` wins
and this file is rewritten to match it.

Updated: 2026-10-30, by Claude (S038 done)

- Last done: S038  duplicate line names the real card (S028 fast check)
- In progress: none
- Next: S039  duplicate lists never bury failures; then the rest of plan.md's Open list
- Waiting for the user: S025 (Linux), S017 (Mac hardware)
- Blocked: none
- Parked:
  - Decisions the agents picked that the user has not confirmed yet:
    DEC-005 (folder date = earliest new file), DEC-026 to DEC-028,
    DEC-030 to DEC-032, DEC-034, DEC-035 (1)-(3), DEC-036 to DEC-040,
    DEC-042, DEC-043, DEC-044, DEC-045.
    Each says "not yet confirmed by the user" in decisions.md.
  - The GUI no longer waits for S025 (user, 2026-10-30: do all that needs no
    input or hardware). The audit re-runs after the GUI.
  - The camera's PTP mode (USB without card-reader mode) is unsupported
    (DEC-004). Revisit if card-reader mode proves unreliable.
  - The permission-based tests assume a non-root user: run as root (e.g.
    Docker CI), most fail loudly, and `only_name_and_size_candidates_are_hashed`
    can pass falsely.
  - Linux exFAT: `canonicalize` keeps the case it is given, so two library
    spellings that differ only in case get two fallback locks (S013 fast
    check). Fold the case or key by inode there; check in S025.
  - Linux exFAT: what DEC-044 says of it comes from an agent's recollection of the kernel's fs/exfat (source not read), not
    a run. Check in S025: a folder a Mac made and an accented name typed on
    Linux are expected to give two folders.
  - A freshly formatted card without `DCIM` looks like any USB drive and is
    offered as a destination (S007 fast check). Telling them apart needs
    DiskArbitration (Mac) or udev (Linux).
  - `copy_plan` (public, one plan) still copies on one thread; only
    `Session` copies cards at once (DEC-040 (9)). A GUI must use `Session`.
  - macOS's disk-image service hung once on a test image (2026-10-28); the
    script now also detaches images attached but not mounted.
