id:         S022
goal:       session CLI/TUI hardening from the S020 fast check, and restore test coverage weakened in S020
accepts:    `sync --one-by-one` never waits for the next card when stdin is not a terminal (it ends after the given cards, as with end of input), tested with an open, never-closed pipe and a timeout; a Ctrl-C at the prompt is never missed (e.g. poll stdin with a short timeout and re-check the cancel), tested; a cancel during card discovery runs no sync and records none, tested; the coverage S020 dropped is restored: a refused report (report does not add up) after copying is NotComplete in the core session verdict (core test, may use test-support), and in the app it exits 1 with its run_state text and is kept after going back and quitting, tested without the core seam in the binary (via an injected sync in worker::spawn_with or an equivalent seam the binary owns); each item gets a failing test first; the doc comment about stdin without a terminal matches the code; scripts/test_exfat.sh passes
touches:    crates/photosync (main.rs, system.rs, app.rs, worker.rs, tests), photosync-core session tests
excludes:   twin fallback (S021); parallel copying (S019); GUI
closes:
paused_by:
author:
done:
