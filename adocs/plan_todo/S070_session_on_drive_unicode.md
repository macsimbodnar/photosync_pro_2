id:         S070
goal:       a later card's duplicate is named as a duplicate with its card, whatever the filesystem's Unicode form
accepts:    failing test first on exFAT and HFS+ images: session with an accented name, then a later card with the same photo: 'duplicate in session' names the card; no new dependency; suite and script pass
touches:    photosync-core session.rs, plan.rs, tests
excludes:   normalisation tables or crates
closes:     2026-10-31_adversarial-F02
paused_by:
author:
done:
