id:         S020
goal:       the CLI and the TUI run a core Session: one card after another, one combined report, the worst outcome
accepts:    tests: the TUI's report screen offers the next card (same library, name and folder) and its final report merges every sync, naming the library and the folder; the CLI takes a next card in the same session (design to confirm with the user); across a session the exit code and final message keep the worst outcome of every sync in it, and the message names each not-complete run, so a later refused retry never hides an earlier failed or crashed copy (S018 note); a worker crash is recorded in the session (`Session::record_crash`); README and specs updated; split from S010 (DEC-032). Needs the user first: `app::tests::a_later_sync_replaces_the_kept_failure` asserts DEC-031 (1), "the last sync wins", which the S018 note reverses; the user drops or changes it
touches:    crates/photosync (app, worker, flow, text, tui, main), README.md, adocs/specs.md
excludes:   GUI
closes:
paused_by:
author:
done:
