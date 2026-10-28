id:         S020
goal:       the CLI and the TUI run a core Session: one card after another, one combined report, the worst outcome
accepts:    tests: the TUI's report screen offers the next card (same library, name and folder) and its final report merges every sync, naming the library and the folder; the CLI takes a next card in the same session (design to confirm with the user); across a session the exit code and final message keep the worst outcome of every sync in it, and the message names each not-complete run, so a later refused retry never hides an earlier failed or crashed copy (S018 note); a worker crash is recorded in the session (`Session::record_crash`); the final TUI report merges every sync's report (S006 fast check, front-end part); README and specs updated; split from S010 (DEC-032). The user ruled (DEC-033): `app::tests::a_later_sync_replaces_the_kept_failure` is rewritten to the session rule: exit 1, and the message names the crashed run
touches:    crates/photosync (app, worker, flow, text, tui, main), README.md, adocs/specs.md
excludes:   GUI
closes:
paused_by:
author:
done:
