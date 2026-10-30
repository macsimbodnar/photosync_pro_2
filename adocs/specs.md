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
  after that check, by a rename that fails if the target exists. A file
  that planning hashed must also read as that hash when copied: one that
  changed since fails, and so do its duplicates. Any file that reads as
  more or fewer bytes than the scan listed fails the same way, before the
  rename, hashed or not (DEC-042). One that cannot be read
  is copied from its first twin (same name, size and planning hash, on
  another device than its source, so never the same card, even given as
  two sources) that reads as that hash; a twin
  that cannot be read or reads otherwise is passed for the next. A failure on the drive's side
  is never retried on a twin (DEC-036).
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
  original's bucket, with a reason naming the original. A file that could
  not be read while copying, whose twin was copied instead (INV-3), is a
  duplicate in session of that twin, with its read error; it is also
  counted apart ("could not be read"), so a front-end shows that its card
  had a read error. Its other twins are duplicates in session of the
  copied twin too, except one passed because it read otherwise than
  planned: that one fails. Failed holds read,
  write and verify errors and name collisions, whether found while planning
  or copying, and "not flushed": a copy in place whose folder flush failed.
  Not flushed stays in failed and is also counted apart, so it can be shown
  as "in place, flush not confirmed". A twin's copy in place unflushed is
  that too: the file it stood in for fails with its own read error, and
  its other twins are failed duplicates of the twin's copy. Cancelled holds every file a cancel
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
  library is under a source root, inside any `DCIM` tree, contains a
  source, is on the same device (`st_dev`) as any source root (mount point
  or plain folder), or is on a card: a volume with a `DCIM` folder at its
  top, a source or not. The library's volume is found by walking up from
  the library to where the device changes; the boot disk (`/`) is never a
  card. So no second path to a card, and no folder on it, gets past the
  check (DEC-025, DEC-026). Copying checks again before it writes or
  deletes anything: if the library is now on another device or volume
  than when the sync was planned, or on a source's device, it refuses
  (DEC-027). It checks again before it makes each folder, in each card's
  copy thread, and once such a check has failed in any of them, before
  every file: those files fail with the reason.

## Behaviour

Photosync is a desktop app that copies photos from a Fujifilm X-T5 (its SD
cards, or the camera itself over USB) to an external hard drive the user
picks. It is simple to use.

### Platforms and interfaces

- Mac first, then Linux, Windows maybe later.
- Plain Rust, no web view. Ships as one self-contained binary or app; the
  user installs nothing else.
- One core library holds all sync logic. Front-ends are thin layers over it:
  - CLI: scriptable commands with plain-text output: `sources`, `drives`,
    `sync --source <path>... --dest <library> [--name <name>]
    [--one-by-one]`. Exit code 0 when every supported file is copied or
    already on the drive, 1 when some failed or were cancelled or part of
    a card could not be read, 2 for a usage error or a sync refused before
    copying (DEC-028). With `--one-by-one`, after each sync it asks on
    stderr for the next card and reads a line on stdin: Enter alone takes
    the mounted cards, a line names one card, `q`, Ctrl-C or the end of
    the input finishes; with no `--source` the first card is asked for,
    and none given exits 2. It asks only when stdin is a terminal:
    otherwise (`</dev/null`, a pipe, open or not) it syncs the `--source`
    cards and ends, and with none prints "No card was synced." and exits
    2, so a script never hangs. A Ctrl-C at the prompt is always seen,
    and one while the cards are looked for runs no further sync. All syncs are one session; after several, a
    summary ends the report, and the exit code and the result are the
    session's worst, naming each sync that was not complete (DEC-034).
    A sync with a file copied from its twin is never plain "complete":
    its state and the session's result name each card that could not be
    read (DEC-037).
    It refuses to run as root (euid 0): root can write to system volumes.
  - TUI: a terminal interface, the first interactive front-end. `photosync`
    with no command opens it when stdin and stdout are a terminal; otherwise
    it is a usage error (exit 2). It walks through picking the sources, the
    destination drive, the library folder on it (default `<drive>/Photos`),
    an optional name, then shows progress and the report, with the CLI's
    sentences and exit codes. Cancel is a key, and Ctrl-C. One session
    lasts the whole run: after a report or a refusal, the next card goes
    back to the cards in the same session (same library, name and folder),
    or the user quits. The report shows every sync of the session and the
    totals of all, naming the library and the folder. The exit code and
    the message are the session's worst, naming each sync that was not
    complete, so neither going back nor a later sync ever hides an earlier
    one that was not complete. The result names each card a file could
    not be read on and was copied from its twin, as the CLI's does. A name that cannot be a folder name is
    refused before the session starts and can be fixed; after that, the
    library and the name stay. A sync with no source picked does not
    start. A panic in the sync's thread leaves the screen as it is, and
    the sync is recorded in the session as not complete (DEC-031,
    DEC-034).
  - A closed stdout never stops a sync nor panics: the exit code stays
    within 0/1/2.
  - GUI: a native Rust GUI, later. Adding it must need no change to the core
    beyond new API, which keeps INV-8 true.

### Sources

- The app lists the mounted volumes that hold a `DCIM` folder, and the user
  picks one or more of them. It looks under `/Volumes` on macOS, and under
  `/media/$USER` and `/run/media/$USER` on Linux. Only a folder there that
  is a mount point is a volume: the macOS boot volume (a symlink to `/` in
  `/Volumes`), a folder left by an unclean eject, and entries whose name
  starts with `.` are never listed (DEC-024).
- This covers SD cards in card readers, and the X-T5 on a USB cable in its
  "USB card reader" connection mode. The default PTP mode does not mount as
  a volume and is not supported for now.
- Several sources at once, when several readers are connected. Each source
  shows as its own entry with its own progress. Each card (each device) is
  copied on its own thread, at the same time as the others; two sources on
  one card are read one file at a time. Renames into one folder still go
  one at a time (DEC-008), so two cards' different photos of the same name
  give one copy and one name collision, never an overwrite. A photo on two
  cards is copied once, by the thread of the card it was planned from; its
  twin on the other card is read only by that thread (DEC-040).
- One source after another, with a single reader. The session stays open
  between cards, and the report covers all of them. A photo an earlier card
  of the session put on the drive, or found there, is a "duplicate in
  session" on a later card, naming the card whose file is on the drive now.
  The same card synced again is "already on drive", not a duplicate of
  itself. The session's verdict is the worst of its syncs, so a later sync
  never hides an earlier one that was not complete.
- An unreadable file or folder under `DCIM` does not stop the scan: it is
  recorded as a failure with its path and error, and the rest of the card
  is still scanned. Only an unreadable source or `DCIM` fails the source.
- Any slot setup works: RAW on one card and JPEG on the other, backup
  (identical cards), or overflow. Each file goes to the folder for its type,
  whatever card it came from. A file found on two sources in one session is
  copied once. If reading it from one source fails, the copy on the other
  source is tried, and must read as what planning read (INV-3). The report
  lists the extra copies as "duplicate in session", names the card whose
  file was copied, and shows the read error of the card that failed.

### Destination

- The app lists the other mounted volumes in the same places that the user
  can write to. The user picks the external drive, and the library folder
  on it.
- The library is never on a card, nor on the same drive as a source, even a
  source that is a plain folder (INV-10).
- The library folder must exist; the app never creates it (DEC-028).
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
  after an interruption lands in the same folder. The filesystem decides
  what is the same name: a dated folder is reused if `<its date> - <name>`,
  as the user spelt it, resolves to it. So another Unicode form of the name
  reuses the folder where the filesystem does not tell the forms apart
  (macOS), and so does another case on a case-insensitive drive. Where the
  filesystem tells them apart, it is another name (DEC-044).
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
  (DEC-016). A session deletes photosync's own stale temp files once,
  before its first copy, before any card's copy thread starts, never
  while one of its copies runs.
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
- With several cards at once, several files are in flight, at most one per
  card. Each progress event is a whole snapshot: the cards' counts add up
  to the totals, and nothing goes back.
- The user can cancel. Cancelling is the same as an interruption (INV-7).
  A cancel stops every card's copy at its next check. A crash in one
  card's copy stops the others the same way; the sync is then not
  complete (DEC-040).
- Planning, which hashes the already-synced candidates, shows progress too
  and can be cancelled. A cancelled plan has written nothing.

### Report

- At the end, shows what was done: copied, skipped (already on drive, or
  duplicate in session, with the read error of a file copied from its
  twin, and each card a file could not be read on named in the result,
  complete or not),
  failed with a reason (name collisions and copies in place not flushed
  included),
  cancelled, ignored types, and the entries the scan could not read. Totals
  per source and per type (INV-6). A source with duplicates counts its
  skipped by reason, and lists each duplicate in session, with the card it
  is on the drive from (DEC-043). Duplicates never bury a failure: when a
  report has a file or entry that went wrong on any card, or a line about
  the library, or a sync that was refused or crashed, every duplicate comes
  after all of those and the tables, under the name of its source's card;
  with none of those, each stays in its card's section (DEC-046). The CLI
  prints a report as each sync ends, so this holds within each report it
  prints; the TUI's report holds it over the whole session. When problem
  lines (a file or entry that went wrong, a library entry or temp file that
  could not be read or removed, in the TUI's report a refused or crashed
  sync) sit above duplicates, the report says how many just above its result:
  "2 problem lines above the duplicates: read them." After several syncs the
  CLI's summary says how many the reports above hold, as a library problem
  leaves its sync "complete". A report with no problem has no such line
  (DEC-048). Files already on
  the drive are counted, not listed. The CLI's report names the library and
  the session folder. After several syncs in a session, the report ends
  with the totals of all of them and the session's result.
- The report is never built from inputs that would make it wrong: a source
  given twice (in any spelling), or a plan or copy that is not of these
  scans (a file not listed, listed with another size or mtime, left out, a
  duplicate whose original is not reported or is itself a duplicate of an
  earlier sync's file, or a file read in place of a
  planned one that the plan does not list as its twin) is refused with the
  reason.

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
