id:         S043
goal:       on a filesystem without no-replace rename, two processes can never replace each other's file under one final name
accepts:    first show the race, or say honestly why it could not be shown: on an exFAT image two photosync processes copy different photos of one name into one folder at once, or a core test forces the check-then-rename window; then the fallback's check and rename hold a lock that other processes honour (DEC-018's lock is per process); preferred: flock(2) on a lock file or the library folder, because the kernel drops it when a process dies, so a crash leaves nothing stuck; measure on exFAT, HFS+ and FAT32 images whether flock works; where it does not, refuse or fail the copy rather than go unguarded, or record why the risk stays; a lock file left behind is never counted as a photo, never reported and cleaned like a temp; the no-replace path on APFS and ext4 is unchanged; the outcome is exactly one final file of that name and the other copy reported as a name collision; DEC-018 amended by a new decision, the agent's choice, not yet confirmed; specs, README if they name the limit; suite and scripts/test_exfat.sh pass; mutant: lock removed fails the test if the race could be forced
touches:    photosync-core copy.rs, tests, scripts/test_exfat.sh, specs, decisions
excludes:   a lock across the whole sync; two sessions in one process
closes:
paused_by:
author:
done:
