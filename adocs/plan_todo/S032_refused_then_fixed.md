id:         S032
goal:       a refused sync whose cards all later complete in the session no longer ends the session NOT COMPLETE; and the TUI checks the library before it makes the session
accepts:    failing tests first: refused then complete on the same cards gives exit 0 and a complete result, in CLI and TUI; a refused sync whose card was not later completed still counts as worst, naming it (the rule only clears a sync that wrote nothing and whose cards completed later); no refused-then-different-card path hides anything; the TUI on a fresh drive with no <drive>/Photos gets its refusal or offer before the session starts; DEC-034 (2) is amended by a new decision, written as the agent's choice under the user's standing instruction and not yet confirmed; specs and README updated; suite and script pass
touches:    photosync-core session.rs verdict, crates/photosync app.rs, text.rs, specs, README, decisions
excludes:   creating the library folder without the user asking
closes:     2026-10-30_adversarial-F07
paused_by:
author:
done:
