id:         S076
goal:       tests kill the mutants N38 and N34
accepts:    on an archive copy: N38 (a folder or symlink named like a stale temp is removed) and N34 (lowercase dcim) each fail the gate, by a real folder on a real image for N34; if a real lowercase folder shows a defect, a new step; no behaviour change otherwise
touches:    photosync-core copy.rs tests, source.rs tests
excludes:   behaviour changes
closes:     2026-10-31_adversarial-F08
paused_by:
author:
done:
