id:         S071
goal:       an accented file name from an NFC source is not copied twice nor a false collision on exFAT or HFS+
accepts:    failing test first on images: second run copies 0, exit 0, and another --name does not copy it again (INV-4); no new dependency, or ask; suite and script pass
touches:    photosync-core library.rs, plan.rs, copy.rs, tests
excludes:   renaming a user's files
closes:     2026-10-31_adversarial-F03
paused_by:
author:
done:
