id:         S072
goal:       the shipped GUI's picture seam cannot replace a user's file, and its flags match the specs
accepts:    failing test first: the smoke picture never replaces an existing file (create_new, refused with a sentence); --search-root and --smoke-png are either stated in specs or hidden behind what specs allow, and the specs' words 'narrow' match what the root does; suite and scripts/test_package_mac.sh pass
touches:    crates/photosync-gui src/args.rs, png.rs, gui.rs; specs; DEC-090
excludes:   removing the package test's seam without a replacement
closes:     2026-10-31_adversarial-F04
paused_by:
author:
done:
