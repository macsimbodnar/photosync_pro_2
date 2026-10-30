id:         S029
goal:       'cargo test --workspace' runs at least one real successful sync through the binary, and a skipped test is not reported as passed silently
accepts:    on macOS the gate proves a successful sync, the worst-outcome exit code (mutant M29: exit code from the last sync) and the closed-stdout rule, without the user setting anything: e.g. the tests mount their own images through scripts or a helper when hdiutil exists, or a fixture the binary already accepts; no test seam in the binary (tests/no_test_seam.rs stays true and unweakened); where the fixture is impossible (Linux without root) the skip is visible as #[ignore] or an explicit count, and Linux gets a documented equivalent in the README and in S025; rerun M29 on a scratch copy and show it killed; propose the TESTS rule wording as a decision, not yet confirmed by the user, and do not edit the rule itself
touches:    crates/photosync tests, tests/common, scripts, README, S025 file, decisions
excludes:   new dependencies
closes:     2026-10-30_adversarial-F04
paused_by:
author:
done:
