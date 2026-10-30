id:         S044
goal:       a cancel is heard while waiting for the fallback's folder lock, and one lock timeout does not repeat for every remaining photo
accepts:    failing tests first: a process holding the folder lock (the existing cross-process helper) and a cancel in the other: the waiting copy stops within a second or two, its temp removed, and it counts as cancelled, not failed; with no cancel, after one timeout on a folder the remaining photos of that folder fail at once with the same reason rather than read, verify and wait 60 s each; no photo is ever counted as copied on this path; the per-folder mutex is not held across a wait that a cancel cannot end; README's Linux image-test command passes `--skip copy::tests::fallback_writer`, and the helper keeps failing without its env; DEC-055 (5) stays true or gets a new decision; suite and scripts/test_exfat.sh pass
touches:    photosync-core copy.rs, tests, README
excludes:   the no-replace path; waiting forever
closes:
paused_by:
author:
done:
