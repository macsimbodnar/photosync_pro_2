id:         S027
goal:       a sync name reuses its folder whatever the filesystem's Unicode form
accepts:    failing test first on the exFAT image and HFS+ if the script can mount it: 'Tiranë' in NFC, second run with a later date lands in the first folder; no new dependency (DEPS); preferred approach: let the filesystem decide, by looking up '<date> - <name>' as the user spelt it for each candidate date and comparing identity with the listed entry, since normalisation-insensitive filesystems resolve it and byte-exact ones rightly do not; if that fails on a real image, stop and record why; DEC records what Linux exFAT does; suite and script pass
touches:    photosync-core copy.rs, scripts/test_exfat.sh, specs Layout, decisions
excludes:   normalisation tables or crates
closes:     2026-10-30_adversarial-F02
paused_by:
author:
done:
