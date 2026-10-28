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
- INV-8 Core is UI-free. The core library depends on no terminal or GUI
  crate and never writes to stdout or stderr. All output goes through its
  API: return values and progress events.
- INV-9 No state outside the library. A sync reads and writes nothing on
  the drive except the photo files under the library folder. It keeps no
  index or catalog file anywhere.

## Behaviour

Photosync is a desktop app that copies photos from a Fujifilm X-T5 (its SD
cards, or the camera itself over USB) to an external hard drive the user
picks. It is simple to use.

### Platforms and interfaces

- Mac first, then Linux, Windows maybe later.
- Plain Rust, no web view. Ships as one self-contained binary or app; the
  user installs nothing else.
- One core library holds all sync logic. Front-ends are thin layers over it:
  - CLI: scriptable commands with plain-text output.
  - TUI: a terminal interface, the first interactive front-end.
  - GUI: a native Rust GUI, later. Adding it must need no change to the core
    beyond new API, which keeps INV-8 true.

### Sources

- The app lists the mounted volumes that hold a `DCIM` folder, and the user
  picks one or more of them.
- This covers SD cards in card readers, and the X-T5 on a USB cable in its
  "USB card reader" connection mode. The default PTP mode does not mount as
  a volume and is not supported for now.
- Several sources at once, when several readers are connected. Each source
  shows as its own entry with its own progress.
- One source after another, with a single reader. The session stays open
  between cards, and the report covers all of them.
- Any slot setup works: RAW on one card and JPEG on the other, backup
  (identical cards), or overflow. Each file goes to the folder for its type,
  whatever card it came from. A file found on two sources in one session is
  copied once.

### Destination

- The user picks the external drive, and the library folder on it.
- The app may remember the last choice in its own config on the computer,
  never on the drive (INV-9).

### Layout

`<library>/<date>[ - <name>]/<TYPE>/<camera file name>`

- `<date>` is `YYYY-MM-DD`. Which date: see Open items.
- `<name>` is the optional name the user gives the sync, e.g. "Tirana":
  `2026-10-28 - Tirana/RAW/DSCF1234.RAF`. With no name: `2026-10-28/RAW/...`.
- `<TYPE>` and the extensions that map to it (case-insensitive):
  - `RAW`: `.RAF`
  - `JPEG`: `.JPG`, `.JPEG`
  - `HEIF`: `.HIF`, `.HEIF`
  - `VIDEO`: `.MOV`, `.MP4`
- Other files on the card are ignored and counted in the report.
- File names are the camera's own. If the target folder already holds a
  different file with the same name (the counter wraps at `DSCF9999`), the
  new file gets a suffix, `DSCF1234_1.RAF`, and the report says so.

### Already-synced detection

- Before copying, the app scans the whole library on the drive into memory.
  Nothing is persisted (INV-9).
- A source file counts as already synced if any library file anywhere has
  the same name and size and the same content (hash). Name + size picks
  the candidates, and the hash confirms them. So a photo synced earlier into
  a different folder is still skipped.

### Progress

- Shows files and bytes done out of the total, per source and overall.
- Shows an estimate of the time remaining.
- The user can cancel. Cancelling is the same as an interruption (INV-7).

### Report

- At the end, shows what was done: copied, skipped (already on drive),
  renamed on collision, ignored types, and failed with a reason. Totals per
  source and per type.

## Non-goals

- Editing, viewing, or culling photos.
- Deleting or formatting the card. The camera does this.
- Two-way sync. Copying goes one way only, camera to drive.
- Cloud upload.

## Open items

- Which date goes in the folder name: each photo's capture date (a 5-day
  trip gives 5 folders), the earliest capture date in the sync (one folder),
  or the sync date?
- Where the capture date comes from: EXIF (needs a parser crate that reads
  RAF, HIF and MOV), or the file's modification time as the camera wrote it
  (no dependency, but FAT stores local time with 2 s resolution).
- How copies are verified: a hash crate (fast, one extra read of the
  destination), or a byte compare with no dependency (also re-reads the
  source).
- Native GUI framework, for later: see DEC-002.
