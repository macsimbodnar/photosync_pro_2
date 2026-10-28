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

- INV-1 Source is read-only. A manifest of the source (every path, with
  its hash and mtime) taken before a sync equals one taken after it, so
  added or removed files are caught too. This holds for scan, copy, cancel
  and multi-source sessions.
- INV-2 No destination overwrite. Every file present on the destination
  before a sync is still there afterwards with the same bytes. The one
  exception is photosync's own stale temp files (`.photosync-tmp-*`),
  which a later sync may delete.
- INV-3 Verified copy. A file counts as copied only after its destination
  copy is flushed to disk (file and folder) and its hash equals the source
  hash. Copies are written under a temporary name and moved into place only
  after that check, by a rename that fails if the target exists.
- INV-4 Idempotent. Syncing the same source to the same destination twice
  copies zero files the second time, whatever target folder is picked.
- INV-5 Type separation. Every RAW file lands in a RAW folder and every JPEG
  in a JPEG folder; neither ever lands in the other.
- INV-6 Complete report. Every file a source scan lists appears in the
  report exactly once, with its reason. Per source: copied + skipped +
  failed + cancelled = the supported files the scan listed. Skipped holds
  "already on drive" and "duplicate in session", each with its own reason.
  A duplicate in session is skipped only if its original is copied or
  already on drive; one whose original failed or was cancelled takes the
  original's bucket, with a reason naming the original. Failed holds read,
  write and verify errors and name collisions, whether found while planning
  or copying, and "not flushed": a copy in place whose folder flush failed.
  Not flushed stays in failed and is also counted apart, so it can be shown
  as "in place, flush not confirmed". Cancelled holds every file a cancel
  left uncopied; a cancelled plan reports all supported files as cancelled.
  Ignored files are listed and counted apart, outside the sum. An entry the
  scan could not read is listed with its error, outside the sum; a failed
  folder is "contents unknown", and the files in it are counted nowhere.
- INV-7 Interruption is safe. Killing the app or unplugging a device during a
  sync leaves no partial file under a final name. Re-running the sync
  finishes the job.
- INV-8 Core is UI-free. The core library depends on no terminal, GUI or
  logging crate and never writes to stdout or stderr. All output goes through its
  API: return values and progress events.
- INV-9 No state outside the library. A sync reads and writes nothing on
  the drive except the photo files under the library folder. It keeps no
  index or catalog file anywhere.
- INV-10 Destination is never a source. A sync refuses to start if the
  library is on a source volume, inside any `DCIM` tree, or contains a
  source.

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
- An unreadable file or folder under `DCIM` does not stop the scan: it is
  recorded as a failure with its path and error, and the rest of the card
  is still scanned. Only an unreadable source or `DCIM` fails the source.
- Any slot setup works: RAW on one card and JPEG on the other, backup
  (identical cards), or overflow. Each file goes to the folder for its type,
  whatever card it came from. A file found on two sources in one session is
  copied once. If reading it from one source fails, the copy on the other
  source is tried. The report lists the extra copies as "duplicate in
  session".

### Destination

- The user picks the external drive, and the library folder on it.
- The app may remember the last choice in its own config on the computer,
  never on the drive (INV-9).

### Layout

`<library>/<date>[ - <name>]/<TYPE>/<camera file name>`

- One sync session puts all its new files in one folder, `<date>[ - <name>]`.
  Files skipped as already synced stay where they already are.
- The folder is fixed once, when the session's first copy plan is made.
  Later cards in the same session reuse it.
- `<date>` is `YYYY-MM-DD`, the earliest capture date among the new files
  of that first plan.
- With a name: if a folder `<any date> - <name>` already exists in the
  library, it is reused (the earliest, if several; DEC-017), so a re-run
  after an interruption lands in the same folder.
- The capture date is the file's modification time as the camera wrote it,
  read as local time.
- `<name>` is the optional name the user gives the sync, e.g. "Tirana":
  `2026-10-28 - Tirana/RAW/DSCF1234.RAF`. With no name: `2026-10-28/RAW/...`.
- If that folder already exists, new files are added to it (name collisions
  as below).
- A session or `<TYPE>` folder that is a symlink is never followed: every
  file that would go there fails with the reason (INV-9, INV-10; DEC-020).
- `<TYPE>` and the extensions that map to it (case-insensitive):
  - `RAW`: `.RAF`
  - `JPEG`: `.JPG`, `.JPEG`
  - `HEIF`: `.HIF`, `.HEIF`
  - `VIDEO`: `.MOV`, `.MP4`
- Other files on the card are ignored and counted in the report.
- Any file or folder whose name starts with `.` is ignored, whatever its
  extension (e.g. macOS `._DSCF0001.JPG` AppleDouble files). A dot folder
  is not walked; it counts as one ignored entry (DEC-014).
- File names are the camera's own and are never changed. If the target
  folder already holds a different file with the same name (counter wrap or
  reset), the new file is not copied. It is reported as failed with reason
  "name collision". The user can re-sync it under another name (DEC-013).

### Already-synced detection

- Before copying, the app scans the library folder (not the rest of the
  drive) into memory. Nothing is persisted (INV-9).
- Entries whose name starts with `.` (trash such as `.Trashes` and `.Trash`,
  OS metadata, photosync's own `.photosync-tmp-*` files), as well as
  `$RECYCLE.BIN` and `System Volume Information`, are not library files.
  The scan skips them, so a photo found only there is copied again
  (DEC-016). The sync deletes its own stale temp files.
- A source file counts as already synced if any library file anywhere has
  the same name and size and the same content (hash). Name + size picks
  the candidates, and the hash confirms them. So a photo synced earlier into
  a different folder is still skipped.
- An unreadable library file or folder does not stop the sync: it is
  reported with its path and error. A source file that matches no readable
  candidate is copied (DEC-015).

### Progress

- Shows files and bytes done out of the total, per source and overall.
- Shows an estimate of the time remaining.
- The user can cancel. Cancelling is the same as an interruption (INV-7).
- Planning, which hashes the already-synced candidates, shows progress too
  and can be cancelled. A cancelled plan has written nothing.

### Report

- At the end, shows what was done: copied, skipped (already on drive, or
  duplicate in session), failed with a reason (name collisions included),
  cancelled, ignored types, and the entries the scan could not read. Totals
  per source and per type (INV-6).
- The report is never built from inputs that would make it wrong: a source
  given twice (in any spelling), or a plan or copy that is not of these
  scans (a file not listed, listed with another size or mtime, left out, or
  a duplicate whose original is not reported) is refused with the reason.

## Non-goals

- Editing, viewing, or culling photos.
- Deleting or formatting the card. The camera does this.
- Two-way sync. Copying goes one way only, camera to drive.
- Cloud upload.

## Open items

- Naming a selection of photos, instead of the whole sync, is a possible
  future feature.
- Capture date from EXIF instead of the file timestamp: maybe later.
- Native GUI framework: iced is approved as a dependency and is the
  tentative choice, to be confirmed when the GUI is planned (DEC-011).
