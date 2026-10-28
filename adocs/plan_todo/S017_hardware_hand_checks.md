id:         S017
goal:       hand checks on real hardware (Mac): X-T5 SD card in a reader, X-T5 over USB in card-reader mode, sudo refusal, Ctrl-C
accepts:    with a real external drive as library: an X-T5 SD card in a card reader is listed by `photosync sources` (not by `drives`), `photosync sync --source <card> --dest <drive>/Photos --name HandCheck` exits 0 and the files land under `<date> - HandCheck/<TYPE>/`, and the same sync again copies 0; the scan line's device equals `stat -f %d <card>` and differs from `/` and the drive; the X-T5 over a USB-C data cable in PC CONNECTION MODE = USB CARD READER is listed and syncs the same way; `sudo photosync drives` refuses with exit 2; the same card synced through the TUI (`photosync` with no arguments) reaches the report; optional: Ctrl-C during a large sync gives NOT COMPLETE, exit 1, and a re-run finishes; camera firmware, card and macOS versions plus the outputs are recorded in the stamp
touches:    nothing in code unless a check fails (then a new step)
excludes:   Linux
closes:     2026-10-28_adversarial-F09
paused_by:
author:
done:
