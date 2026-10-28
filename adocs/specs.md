# Specs

What the software must do — current state only. Precedence: specs beat plan
beat status. Code that disagrees with this file is a bug or an unrecorded
decision. The narrative of how a rule got its wording lives in commits and
`plan_done/`, not here.

## Prime directive

Never lose a photo: the source is never modified, and no file already on the
destination drive is ever overwritten or deleted.

## Invariants

Numbered, testable properties, referenced by number from code comments, test
names, and commit messages. An invariant nothing could check is a wish —
reword it until it is checkable. Numbers are never reused; a retired one
stays listed as retired.

- INV-1 Source is read-only. A hash and mtime manifest of the source taken
  before a sync equals one taken after it.
- INV-2 No destination overwrite. Every file present on the destination
  before a sync is still there afterwards with the same bytes.
- INV-3 Verified copy. A file counts as copied only after the destination
  copy's hash equals the source hash. Copies are written under a temporary
  name and renamed into place only after that check.
- INV-4 Idempotent. Syncing the same source to the same destination twice
  copies zero files the second time, whatever target folder is picked.
- INV-5 Type separation. Every RAW file lands in a RAW folder and every JPEG
  in a JPEG folder; neither ever lands in the other.
- INV-6 Complete report. Every supported source file appears in the report
  exactly once, as copied, skipped (already on drive) or failed (with a
  reason). copied + skipped + failed = supported files found.
- INV-7 Interruption is safe. Killing the app or unplugging a device during a
  sync leaves no partial file under a final name. Re-running the sync
  finishes the job.

## Behaviour

Photosync is a desktop app that copies photos from a Fujifilm X-T5 (its SD
cards, or the camera itself over USB) to an external hard drive the user
picks. It is simple to use.

### Platforms

Mac first, then Linux, Windows maybe later. Ships as one self-contained app:
the user installs nothing else.

### Sources

- An SD card in a card reader.
- Several cards at once, when several readers are connected. Each card
  shows as its own source with its own progress.
- One card after another, with a single reader. The session stays open
  between cards and the report covers all of them.
- The X-T5 connected by USB cable.

### Destination

- The user picks which external drive to sync to, and the folder on it that
  holds the photo library.
- The app remembers the last drive and library and offers them next time.

### Layout

- RAW files (`.RAF`) and JPEG files (`.JPG`) go to separate folders.
- The user can give the sync a name, e.g. "Tirana trip". Photos then go into
  that folder, with its RAW and JPEG sub-folders.
- Exact folder structure: see Open items.

### Already-synced detection

- Before copying, the app scans the library on the drive and skips every
  photo already there, even if it was synced earlier into a different
  folder.

### Progress

- Shows files and bytes done out of the total, per source and overall.
- Shows an estimate of the time remaining.

### Report

- At the end, shows what was done: copied, skipped (already on drive), and
  failed with a reason. Totals per source and per type (RAW / JPEG).

## Non-goals

- Editing, viewing, or culling photos.
- Deleting or formatting the card. The camera does this.
- Two-way sync. Copying goes one way only, camera to drive.
- Cloud upload.

## Open items

- Folder layout. Proposal: `<library>/<name or YYYY-MM-DD>/RAW/` and
  `<library>/<name or YYYY-MM-DD>/JPEG/`. Alternative:
  `<library>/RAW/<name>/` and `<library>/JPEG/<name>/`. With no name given,
  group by capture date?
- File identity, i.e. what counts as "already synced". The X-T5 file counter
  wraps at `DSCF9999`, so the name alone is not enough. Proposal: original
  name + size + EXIF capture time, with a content hash on a match. A catalog
  file on the drive would avoid rescanning the whole library on every sync.
- Other file types. The X-T5 also shoots HEIF (`.HIF`) and video
  (`.MOV`/`.MP4`). Copy them, ignore them, or give them their own folders?
- Camera over USB. In "USB card reader" connection mode the X-T5 mounts as a
  normal drive (user reports, not verified). Supporting only that mode avoids
  the PTP protocol entirely. Is that acceptable?
- Renaming. Keep the camera's original file names, or rename (e.g. by date)?
- Two cards in one camera. The X-T5 has two slots, often set to RAW on one
  card and JPEG on the other, or to backup. Does dedup across cards in one
  session need special handling?
