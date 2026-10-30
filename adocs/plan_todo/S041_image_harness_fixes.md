id:         S041
goal:       the image test harness keeps its promises: the watchdog's hdiutil calls have a time limit, the holder's tell file is atomic, the README's Linux commands work
accepts:    crates/photosync-core/tests/common/images.rs: the watchdog's hdiutil calls are limited, tested with a fake hdiutil on PATH that hangs (the watchdog ends and says so), matching what README and DEC-049 (4) claim; crates/photosync/tests/test_images.rs: the tell file is written to a temp name then renamed, and the holder is killed on the failure path too; README's Linux image block removes lost+found after mount (mkfs.ext4 leaves it root-only and the test manifest walk would panic), with the same line in S025 if it repeats it, stated as not yet run on Linux; with `mount` and `hdiutil` missing from PATH the message says that, not "FAILED to detach", and leaves no empty temp folder; suite and scripts/test_exfat.sh pass; 0 images attached after
touches:    crates/photosync-core/tests/common/images.rs, crates/photosync/tests/test_images.rs, README, S025 file
excludes:   product code
closes:
paused_by:
author:
done:
