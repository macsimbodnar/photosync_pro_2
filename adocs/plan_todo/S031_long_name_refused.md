id:         S031
goal:       a name that cannot be a folder name is refused before any card is read, including one too long
accepts:    failing test first: '<date> - <name>' over 255 bytes is refused, exit 2 in the CLI, fixable in the TUI before the session starts; the characters refused per platform are decided and recorded (':' on macOS and FAT-family characters on exFAT, checked on a real image); the 'Folder ...' report line never names a folder that does not exist in this path; suite and script pass
touches:    photosync-core copy.rs check_name, crates/photosync app.rs, text.rs, specs, decisions
excludes:   renaming or sanitising a name silently
closes:     2026-10-30_adversarial-F06
paused_by:
author:
done:
