id:         S036
goal:       plan the native GUI: confirm iced, decide the crate layout, write the GUI's steps
accepts:    a short feasibility spike on scratch only proves the chosen iced version builds on this Mac and can render headlessly or be driven in tests; a decision confirms iced (DEC-011) as the agent's choice under the user's standing instruction, not yet confirmed; the plan says how the reusable app/worker/flow/text layer reaches a GUI binary (no core change beyond new API, INV-8, tests/ui_split.rs); GUI uses Session (DEC-040 (9)); specs' GUI wording and Open items are updated; small ordered steps exist in plan_todo with testable accepts, entered in plan.md's Open before S025, covering every screen of the TUI's walk and its rules; the UI name follows DEC-041; packaging (Mac .app, Linux AppImage) gets its own steps, with what needs the user marked
touches:    adocs, nothing in the product
excludes:   writing the GUI
closes:     
paused_by:
author:
done:
