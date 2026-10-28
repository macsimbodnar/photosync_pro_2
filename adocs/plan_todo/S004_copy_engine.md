id:         S004
goal:       core executes a sync plan: target folder naming, temp copy, blake3 verify, atomic rename, collision suffix
accepts:    tests for INV-2, INV-3, INV-4, INV-5 and INV-7 (simulated interruption leaves no file under a final name; re-run completes); folder is `<earliest new-file date>[ - <name>]/<TYPE>/`; a same-name different-content file gets `_1`, `_2`, ...; running the same sync twice copies zero files the second time; rename fails if the target exists (libc renamex_np/renameat2, DEC-008), and a test races two renames onto one name and loses no file; file and folder fsync'ed before verify; temp files named `.photosync-tmp-*`, stale ones deleted by the next sync and nothing else ever deleted (INV-2); the folder is fixed at session start and an existing `<any date> - <name>` is reused (DEC-009); a test for INV-1 (full path + hash + mtime manifest) around a copy
touches:    photosync-core
excludes:   progress events; UI
closes:     2026-10-28_adversarial-F02, 2026-10-28_adversarial-F04, 2026-10-28_adversarial-F05, 2026-10-28_adversarial-F06, 2026-10-28_adversarial-F08
paused_by:
author:
done:
