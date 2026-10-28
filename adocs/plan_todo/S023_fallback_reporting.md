id:         S023
goal:       twin-fallback reporting fixes from the S021 fast check
accepts:    in a session of several syncs, a sync whose only problem is a read error copied from a twin is never shown as plain "complete": its per-sync state and the session result line warn about the unread card (CLI and TUI), tested on a multi-sync session; twins are only files on another source (never the same card), as INV-3 says, tested with the same photo in two folders of one card; a twin copy that is in place but whose folder flush failed is reported as "in place, flush not confirmed" (counted in not_flushed), not hidden, tested by fault injection; each item gets a failing test first; scripts/test_exfat.sh passes
touches:    photosync-core plan/copy/report, crates/photosync text.rs and app.rs
excludes:   parallel copying (S019); GUI
closes:
paused_by:
author:
done:
