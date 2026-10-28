id:         S003
goal:       core scans the library into memory and decides which source files are already synced
accepts:    tests: same name + size + blake3 hash anywhere in the library → skipped; same name and size, different content → not skipped; a file present on two sources in one session is planned once; no file is written anywhere during planning (INV-9)
touches:    photosync-core
excludes:   copying; persistence of any index
closes:
paused_by:
author:
done:
