id:         S018
goal:       TUI hardening from the S009 fast check: no lost failure exit, worker panic handled, no empty sync, stricter ui split check
accepts:    the last non-OK outcome (failed, cancelled, refused report, worker gone) and its exit code are kept until a later sync finishes, so navigating back and quitting never exits 0 or drops the message, tested for WorkerGone and a report error; a panic in the worker thread does not restore or break the terminal while the TUI runs: it is caught (e.g. catch_unwind in worker::spawn) and reported as a not-complete outcome, or the restore hook acts on the main thread only, tested; Confirm refuses to start with no source picked, tested; ui_split.rs checks every .rs file under the reusable modules (incl. new submodules) and rejects re-exports from the terminal modules, and it fails on a planted violation (prove it, then restore); each item gets a failing test first; scripts/test_exfat.sh passes
touches:    crates/photosync (app.rs, worker.rs, tui/, tests/ui_split.rs)
excludes:   session across cards (S010); GUI
closes:
paused_by:
author:
done:
