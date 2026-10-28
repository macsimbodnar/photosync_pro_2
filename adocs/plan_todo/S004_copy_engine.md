id:         S004
goal:       core executes a sync plan: target folder naming, temp copy, blake3 verify, no-replace rename, name collision = failed
accepts:    tests for INV-2, INV-3, INV-4, INV-5 and INV-7 (simulated interruption leaves no file under a final name; re-run completes); folder is `<earliest new-file date>[ - <name>]/<TYPE>/`; a same-name different-content file in the target folder is not copied, is reported as failed "name collision", and the existing file is untouched; running the same sync twice copies zero files the second time; rename fails if the target exists (libc renamex_np/renameat2, DEC-008), and a test races two renames onto one name and loses no file; file and folder fsync'ed before verify; temp files named `.photosync-tmp-*`, stale ones deleted by the next sync and nothing else ever deleted (INV-2); the folder is fixed at session start and an existing `<any date> - <name>` is reused (DEC-009); a test for INV-1 (full path + hash + mtime manifest) around a copy; a read error on a source file while hashing or copying fails that file only (path + error) and the sync goes on (the scan reads metadata only, see S011)
touches:    photosync-core
excludes:   progress events; UI
closes:     2026-10-28_adversarial-F01, 2026-10-28_adversarial-F02, 2026-10-28_adversarial-F04, 2026-10-28_adversarial-F05, 2026-10-28_adversarial-F06, 2026-10-28_adversarial-F08
paused_by:
author:
done:
