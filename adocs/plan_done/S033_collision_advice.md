id:         S033
goal:       after a name collision, the result names the way out: sync again under another name
accepts:    failing test first: a result with a collision among the failures says to sync again under another name (DEC-013), and a result without one keeps today's sentence; the TUI's wording fits its own rule that the name stays for the session; suite passes
touches:    crates/photosync text.rs, tests
excludes:   changing collision handling
closes:     2026-10-30_adversarial-F08
paused_by:
author:     Claude (subagent, session 01RTrRzTxLkBQpdFLoEc5hDW)
done:       2026-10-30. Each new test failed first. cargo test --workspace: 322 passed, 0 failed, 28 ignored. scripts/test_exfat.sh: exit 0, 325 passed, 0 failed, no image left attached. A result with a collision says to start photosync again and sync under another name (DEC-056), the TUI's box shows it at 80 columns, and on images `--name` another copies it, exit 0; without one the sentence stays. exfat_session's exact sentence for a session holding a collision changed to the new wording: the old one was F08. Also fixed, each with its own failing test: (a) a temp already gone on removal is neither removed nor failed; (b) a path over the system's limit is refused with its own sentence, not "at most 242 bytes", and DEC-053 (4) corrected. Measured by hand: a library that takes the folder but not a file under it, or a sync with no name, still fails each file, exit 1.
