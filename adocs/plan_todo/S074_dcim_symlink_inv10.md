id:         S074
goal:       a DCIM symlink cannot get a library past INV-10
accepts:    failing test first: a plain-folder source whose DCIM is a symlink, library on the source's drive or the source itself: refused, exit 2; a real card's check unchanged; suite and script pass
touches:    photosync-core plan.rs, volume.rs, tests; specs
excludes:   following symlinks during the scan
closes:     2026-10-31_adversarial-F06
paused_by:
author:
done:
