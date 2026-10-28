# Research: existing tools

2026-10-28. What exists for camera → drive photo ingest, measured against
specs.md. Web search plus project pages, not hands-on tests. Claims marked
(unverified) come from search summaries only.

| Tool | Platforms | License | RAW/JPEG split | Already-synced skip | Multi-card | Camera USB | Notes |
|---|---|---|---|---|---|---|---|
| Rapid Photo Downloader | Linux only | GPL | no (syncs sequence numbers only) | yes, remembers downloads | yes, parallel | yes, gphoto2 | Closest match. No Mac. A 2020 Mac port lacked device detection. |
| CamHauler (RPD fork) | Linux; Mac/Win planned | GPLv3 | not described | filter "downloaded before" | no auto-detect since 2026-07 | no (needs card reader or mount) | Alpha, v3.0.0-alpha.1. |
| Photo Mechanic | Mac, Win | commercial | via file-type filter + 2 ingests | not confirmed | yes, several cards at once | — | Pro tool, expensive, much more than needed. |
| Lightroom Classic import | Mac, Win | subscription | no | "don't import suspected duplicates" | one at a time | yes | Tied to a catalog. |
| Photocopy (mpetersen) | Mac | Apache-2.0 | no | yes, via `photocopy.json` written **on the card** | no | — | Java, tiny project. Writes to the source. |
| Imagin Raw | Mac | MIT | (unverified) | near-duplicate bursts, not import dedup | no | — | Date folders, backup to second destination. |
| ShutterFlow | Mac (App Store) | commercial | (unverified) | name + size (unverified) | (unverified) | — | |
| Card Ingest | Mac (App Store) | commercial | type filter, date folders (unverified) | (unverified) | (unverified) | — | |
| cameraimport, exisort, neatcli | CLI | various | cameraimport, neatcli: yes | exisort: yes | no | no | No GUI. |
| macOS Image Capture | Mac | built-in | no | no | no | yes | The X-T5 shows up here in default (PTP) mode. |

## Conclusion

No tool covers all of: Mac first + Linux, RAW and JPEG in separate folders,
skip anything already anywhere in the library, several cards at once or one
after another, named event folders, ETA, and an end report. Rapid Photo
Downloader comes closest but is Linux only. The gap justifies building
photosync.

## X-T5 over USB

- In its default mode the camera does not mount on a Mac. It uses PTP (works
  with Image Capture).
- Menu → Connection setting → Connection mode → "USB card reader" makes it
  mount as a drive (user reports). Then it is just another volume.
- The bundled USB-C cable may be charge-only. Some Mac ports work and
  others do not (user reports).

## Sources

- https://damonlynch.net/rapid/features.html
- https://gitea.com/t00mietum/camhauler
- https://github.com/mpetersen/photocopy
- https://discuss.pixls.us/t/any-interest-in-a-macos-version/15626
- https://alternativeto.net/software/rapid-photo-downloader
- https://alternativeto.net/software/imagin-raw/about/
- https://apps.apple.com/mt/app/shutterflow/id6760598632
- https://apps.apple.com/be/app/lifter-import-sort/id6596760364
- https://home.camerabits.com/tour/
- https://www.dpreview.com/forums/threads/backup-restore-to-mac-wont-work-with-x-t5.4814270
- https://www.fredmiranda.com/forum/topic/1883509/
