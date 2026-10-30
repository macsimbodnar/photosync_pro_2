id:         S042
goal:       the TUI's refused screen offers only keys that work
accepts:    failing test first: a refusal before the session starts (fresh drive, bad name) shows no "n next card" in the footer, and the footer's keys all act; inside a session the footer still offers it; suite passes
touches:    crates/photosync tui/view.rs, tests
excludes:   the rule of what `n` does
closes:
paused_by:
author:     Claude
done:       2026-10-30. Footer offers `n` only in a session. New tests: fresh drive and bad name before the session failed before the fix and pass after; in a session all three keys act (passed before and after, it guards the offer). `cargo test --workspace` rc 0: 302 passed, 0 failed, 23 ignored. `scripts/test_exfat.sh` rc 0: 298 passed, 0 failed. `hdiutil info` shows 0 images after. README and specs wording updated.
