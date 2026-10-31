id:         S073
goal:       decide and record whether a copy keeps the camera's modification time, then do it
accepts:    first ask the user, as it changes what is on the drive; then a decision; if kept: failing test first, on exFAT and HFS+ images, set before the rename, a failure to set it reported and never a false copied; suite and script pass
touches:    photosync-core copy.rs, specs, decisions
excludes:   changing the folder's date rule
closes:     2026-10-31_adversarial-F05
paused_by:
author:
done:
