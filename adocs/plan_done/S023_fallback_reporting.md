id:         S023
goal:       twin-fallback reporting fixes from the S021 fast check
accepts:    in a session of several syncs, a sync whose only problem is a read error copied from a twin is never shown as plain "complete": its per-sync state and the session result line warn about the unread card (CLI and TUI), tested on a multi-sync session; twins are only files on another source (never the same card), as INV-3 says, tested with the same photo in two folders of one card; a twin copy that is in place but whose folder flush failed is reported as "in place, flush not confirmed" (counted in not_flushed), not hidden, tested by fault injection; each item gets a failing test first; scripts/test_exfat.sh passes
touches:    photosync-core plan/copy/report, crates/photosync text.rs and app.rs
excludes:   parallel copying (S019); GUI
closes:
paused_by:
author:     Claude agent (2026-10-28)
done:       2026-10-28, Claude agent. Each accepts item had a failing test first.
            (1) multi-sync session state and result name the unread card, CLI
            text::tests::a_session_names_the_card_a_file_was_copied_from_its_twin_for,
            TUI app::tests::the_result_box_names_the_card_a_file_was_copied_from_its_twin_for
            (real syncs on the exFAT drive image; plain cargo test skips them),
            plus text::tests::the_unread_warning_names_each_card_and_counts_the_files;
            (2) twins only on another source,
            copy::tests::s023_the_same_photo_in_two_folders_of_one_card_is_no_twin;
            (3) a twin's copy in place unflushed is reported as not flushed,
            copy::tests::s023_a_twin_copy_in_place_but_not_flushed_is_reported_as_not_flushed
            (new CopyOutcome::unflushed_fallbacks). cargo test --workspace green,
            clippy clean, scripts/test_exfat.sh exit 0 with no image left
            mounted. Choices in DEC-037; DEC-036 (1) and (3) amended.
