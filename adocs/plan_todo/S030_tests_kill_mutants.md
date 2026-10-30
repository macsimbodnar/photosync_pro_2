id:         S030
goal:       tests kill the surviving mutants M16, M08, M02, M03; M13 is reachable or stays a recorded hand check
accepts:    on a scratch copy (never the tree), each mutant fails the gate: M16 threads per source not per device (Session test, two sources on one device, one file in flight per device); M08 session re-check before cleanup removed (Session test, library moved before cleanup; session.rs:308); M02 and M03 flush removed (a seam that records real flush calls on file and folder, core only, keeping INV-8 and no_test_seam); M13: factor main's guard so a test reaches it, or record why it stays a hand check; the comment at copy.rs:934-936 says only what is true; the mutation outcomes are in the stamp
touches:    photosync-core copy.rs, session.rs, threads_tests.rs, crates/photosync main.rs, system.rs
excludes:   behaviour changes
closes:     2026-10-30_adversarial-F05
paused_by:
author:
done:
