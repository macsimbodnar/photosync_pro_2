id:         S015
goal:       strict INV-10 (DEC-025): refuse a library on any source's device, or on a volume that is a card
accepts:    a plan is refused when the library's st_dev equals any source's st_dev, mount point or not (the review's case: source `/Volumes/CARD/backup`, library `/Volumes/CARD/Photos`); a plan is refused when the library's volume root (walking up to its mount point) holds a `DCIM` folder, even if that card is not a source; each refusal has a clear PlanError variant and a failing test first; tests that need source and library on one temp device simulate "another device" explicitly and visibly (no silent bypass in a normal build), and the device rules have dedicated tests; INV-10 wording in specs updated; scripts/test_exfat.sh still passes
touches:    photosync-core plan/source/volume, adocs/specs.md, tests
excludes:   CLI (S008)
closes:
paused_by:
author:
done:
