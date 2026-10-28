//! Plain-text rendering for the CLI: progress lines, the report, and one
//! sentence per refusal. Strings only, apart from `StderrWatch`, so the
//! rules for what is shown are tested without a terminal.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

use photosync_core::{
    CopyError, Counts, Event, EventKind, FailReason, FileOutcome, FileReport, FolderError,
    MediaType, Phase, PlanError, Progress, Report, ScanError, SourceReport, SourceScan, Tally,
    TempCleanup,
};

use crate::flow::{SyncError, Watch};

/// Bytes in decimal units, as Finder shows them.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub fn duration(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, s % 3600 / 60)
    } else if s >= 60 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

fn plural(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

const MEDIA: (&str, &str) = ("photo or video", "photos or videos");

fn name_of(root: &Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |n| n.to_string_lossy().into(),
    )
}

/// One line: files and bytes done out of the total, overall and, with
/// several sources, per source, and the ETA.
pub fn progress_line(p: &Progress) -> String {
    let label = match p.phase {
        Phase::Planning => "comparing with library",
        Phase::Copying => "copying",
    };
    let percent = match p.total.bytes {
        0 => 100,
        total => u128::from(p.done.bytes) * 100 / u128::from(total),
    };
    let eta = p.eta.map_or_else(|| "--".to_string(), duration);
    let mut line = format!(
        "{label} {}/{} files, {} of {} ({percent}%), ETA {eta}",
        p.done.files,
        p.total.files,
        size(p.done.bytes),
        size(p.total.bytes)
    );
    if p.sources.len() > 1 {
        let each: Vec<String> = p
            .sources
            .iter()
            .map(|s| format!("{} {}/{}", name_of(&s.root), s.done.files, s.total.files))
            .collect();
        line.push_str(&format!(" [{}]", each.join(", ")));
    }
    line
}

/// Turns progress events into text, throttled by the events' own clock:
/// a phase's start and end always show, anything between at most once per
/// interval. The core sends an event per MiB (DEC-021 (4)); a terminal
/// does not need them all.
pub struct ProgressText {
    /// Rewrite one line in place (a terminal), or print one line each time.
    in_place: bool,
    interval: Duration,
    last: Option<Duration>,
}

impl ProgressText {
    pub fn new(in_place: bool) -> ProgressText {
        // A log gets fewer lines than a screen.
        let interval = Duration::from_millis(if in_place { 250 } else { 1000 });
        ProgressText {
            in_place,
            interval,
            last: None,
        }
    }

    /// The text to write for `event`, if it is due.
    pub fn event(&mut self, event: &Event) -> Option<String> {
        let p = &event.progress;
        // Planning often has nothing to hash: a 0/0 line says nothing.
        if p.total.files == 0 {
            return None;
        }
        let end = matches!(event.kind, EventKind::Finished | EventKind::Cancelled);
        let due = match event.kind {
            EventKind::Started => true,
            _ if end => true,
            _ => self
                .last
                .is_none_or(|t| p.elapsed.saturating_sub(t) >= self.interval),
        };
        if !due {
            return None;
        }
        self.last = Some(p.elapsed);
        let mut line = progress_line(p);
        if event.kind == EventKind::Cancelled {
            line.push_str(", cancelled");
        }
        Some(match (self.in_place, end) {
            (true, false) => format!("\r\x1b[K{line}"),
            (true, true) => format!("\r\x1b[K{line}\n"),
            (false, _) => format!("{line}\n"),
        })
    }
}

/// Scan summaries and progress on stderr, so stdout holds only the report.
pub struct StderrWatch {
    progress: ProgressText,
}

impl StderrWatch {
    pub fn new() -> StderrWatch {
        StderrWatch {
            progress: ProgressText::new(io::stderr().is_terminal()),
        }
    }
}

impl Watch for StderrWatch {
    fn scanned(&mut self, scan: &SourceScan) {
        eprintln!("{}", scanned(scan));
    }

    fn event(&mut self, event: Event) {
        if let Some(text) = self.progress.event(&event) {
            let mut err = io::stderr().lock();
            // Progress only: a closed stderr must not stop a copy halfway,
            // and the report on stdout still says what happened.
            let _ = err.write_all(text.as_bytes()).and_then(|()| err.flush());
        }
    }
}

/// What a scan found, with the device it is on: the hand check compares it
/// with `stat -f %d <card>` to see the card's own mount was scanned.
pub fn scanned(scan: &SourceScan) -> String {
    let (mut photos, mut bytes) = (0, 0);
    for f in scan.supported() {
        photos += 1;
        bytes += f.size;
    }
    format!(
        "scanned {}: {} ({}), {} ignored, {} unreadable, device {}",
        scan.root.display(),
        plural(photos, MEDIA.0, MEDIA.1),
        size(bytes),
        scan.ignored().count(),
        scan.failures.len(),
        scan.device()
    )
}

/// Whether every supported file is on the drive and every entry of every
/// card was read: the only state in which a card is safe to format.
pub fn complete(totals: &Tally, sources: &[SourceReport]) -> bool {
    totals.failed.files == 0
        && totals.cancelled.files == 0
        && sources.iter().all(|s| s.unreadable.is_empty())
}

/// The whole report: a section per source, overall totals, and a result
/// line saying whether the cards are safe to format.
pub fn report(report: &Report) -> String {
    let mut out = Vec::new();
    out.push("Report".to_string());
    for source in report.sources() {
        out.push(String::new());
        out.extend(source_section(source));
    }
    let totals = report.totals();
    if report.sources().len() > 1 {
        out.push(String::new());
        out.push("All sources".to_string());
        out.extend(table(&report.by_type(), &totals));
    }
    out.extend(library_section(report.library_failures(), report.temps()));
    out.push(String::new());
    out.push(result_line(&totals, report.sources()));
    out.join("\n") + "\n"
}

/// The totals per type of one source, then every file that is not OK.
pub fn source_section(source: &SourceReport) -> Vec<String> {
    let mut out = vec![format!("Source {}", source.root.display())];
    out.extend(table(&source.by_type(), &source.totals()));
    if let Some(line) = ignored_line(source) {
        out.push(line);
    }
    out.extend(source.files.iter().filter_map(problem_line));
    for u in &source.unreadable {
        let unknown = if u.contents_unknown {
            " (contents unknown)"
        } else {
            ""
        };
        out.push(format!(
            "  unreadable  {}: {}{unknown}",
            u.error.path.display(),
            u.error.source
        ));
    }
    out
}

fn table(by_type: &BTreeMap<MediaType, Tally>, totals: &Tally) -> Vec<String> {
    let row = |label: &str, cells: [String; 4]| {
        format!(
            "  {label:<6}{:>9}{:>10}{:>9}{:>12}",
            cells[0], cells[1], cells[2], cells[3]
        )
    };
    let files =
        |t: &Tally| [t.copied, t.skipped, t.failed, t.cancelled].map(|c| c.files.to_string());
    let mut out = vec![row(
        "",
        ["copied", "skipped", "failed", "cancelled"].map(String::from),
    )];
    for (media, tally) in by_type {
        out.push(row(media.dir_name(), files(tally)));
    }
    out.push(row("all", files(totals)));
    out.push(row(
        "size",
        [
            totals.copied,
            totals.skipped,
            totals.failed,
            totals.cancelled,
        ]
        .map(|c| size(c.bytes)),
    ));
    if totals.not_flushed.files > 0 {
        out.push(format!(
            "  of the failed, {} in place, flush not confirmed",
            totals.not_flushed.files
        ));
    }
    out
}

/// Ignored files are counted by kind, not listed: they are not photos.
fn ignored_line(source: &SourceReport) -> Option<String> {
    let mut kinds: BTreeMap<String, u64> = BTreeMap::new();
    for entry in source
        .files
        .iter()
        .filter(|e| matches!(e.outcome, FileOutcome::Ignored))
    {
        let path = &entry.file.path;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let kind = if name.starts_with('.') {
            "hidden".to_string()
        } else {
            path.extension().map_or_else(
                || "no extension".to_string(),
                |e| format!(".{}", e.to_string_lossy()),
            )
        };
        *kinds.entry(kind).or_default() += 1;
    }
    let total: u64 = kinds.values().sum();
    (total > 0).then(|| {
        let each: Vec<String> = kinds.iter().map(|(k, n)| format!("{k} {n}")).collect();
        format!(
            "  ignored {} ({})",
            plural(total, "file", "files"),
            each.join(", ")
        )
    })
}

/// One line for a file that is not copied or skipped, with its reason;
/// `None` for one that is OK.
pub fn problem_line(entry: &FileReport) -> Option<String> {
    let path = entry.file.path.display();
    Some(match &entry.outcome {
        FileOutcome::Copied { .. }
        | FileOutcome::AlreadySynced { .. }
        | FileOutcome::DuplicateInSession { .. }
        | FileOutcome::Ignored => return None,
        FileOutcome::Failed(FailReason::NotFlushed { to, error }) => format!(
            "  in place, flush not confirmed  {path} -> {}: {error}",
            to.display()
        ),
        FileOutcome::Failed(reason) => format!("  failed      {path}: {reason}"),
        FileOutcome::DuplicateOfFailed { of } => {
            format!(
                "  failed      {path}: same as {}, which failed",
                of.display()
            )
        }
        FileOutcome::Cancelled => format!("  cancelled   {path}"),
        FileOutcome::DuplicateOfCancelled { of } => {
            format!(
                "  cancelled   {path}: same as {}, which was cancelled",
                of.display()
            )
        }
    })
}

fn library_section(failures: &[ScanError], temps: &TempCleanup) -> Vec<String> {
    let mut out = Vec::new();
    if !failures.is_empty() {
        out.push(String::new());
        out.push(
            "Library entries that could not be read (a photo matching only these was copied):"
                .to_string(),
        );
        out.extend(
            failures
                .iter()
                .map(|e| format!("  {}: {}", e.path.display(), e.source)),
        );
    }
    if !temps.removed.is_empty() {
        out.push(String::new());
        out.push(format!(
            "Removed {} left by an interrupted sync.",
            plural(temps.removed.len() as u64, "temp file", "temp files")
        ));
    }
    if !temps.failures.is_empty() {
        out.push(String::new());
        out.push("Temp files that could not be removed (the next sync tries again):".to_string());
        out.extend(
            temps
                .failures
                .iter()
                .map(|e| format!("  {}: {}", e.path.display(), e.source)),
        );
    }
    out
}

pub fn result_line(totals: &Tally, sources: &[SourceReport]) -> String {
    let supported: Counts = totals.supported();
    if complete(totals, sources) {
        if supported.files == 0 {
            return "Result: no photos or videos found.".to_string();
        }
        return format!(
            "Result: all {} on the drive ({} copied, {} already there).",
            plural(supported.files, MEDIA.0, MEDIA.1),
            totals.copied.files,
            totals.skipped.files
        );
    }
    let unreadable: usize = sources.iter().map(|s| s.unreadable.len()).sum();
    let mut line = format!(
        "Result: NOT COMPLETE: {} failed, {} cancelled, {} unreadable.",
        totals.failed.files, totals.cancelled.files, unreadable
    );
    if totals.not_flushed.files > 0 {
        line.push_str(&format!(
            " {} of the failed in place, flush not confirmed.",
            totals.not_flushed.files
        ));
    }
    line.push_str(
        " Do not format the card. Fix the cause and run the same sync again: \
         files already on the drive are skipped.",
    );
    line
}

/// One plain sentence per refusal: what is wrong, and what to do.
pub fn refusal(err: &SyncError) -> String {
    let p = |path: &Path| path.display().to_string();
    match err {
        SyncError::Scan(e) => format!(
            "Cannot read {}: {}. A source must be a camera card with a DCIM folder \
             (`photosync sources` lists them); check it is still plugged in.",
            p(&e.path),
            e.source
        ),
        SyncError::SourceTwice { first, again } => format!(
            "Source {} is the same as source {}. Give each card once.",
            p(again),
            p(first)
        ),
        SyncError::Plan(e) => plan_refusal(e),
        SyncError::Folder(FolderError::InvalidName(name)) => format!(
            "The name {name:?} cannot be a folder name. Use a name that is not empty \
             and has no \"/\". Nothing was copied."
        ),
        SyncError::Folder(FolderError::Io(e)) => format!(
            "Cannot read {}: {}. Nothing was copied. Check the drive, then run the sync again.",
            p(&e.path),
            e.source
        ),
        SyncError::Copy(e) => copy_refusal(e),
        SyncError::Report(e) => format!(
            "The copy ran, but its report does not add up ({e}), so it is not shown. \
             Do not format the card. Run the same sync again: files already on the \
             drive are skipped, and its report says what is left."
        ),
    }
}

fn plan_refusal(err: &PlanError) -> String {
    let p = |path: &Path| path.display().to_string();
    match err {
        PlanError::LibraryOnCard { library, volume } => format!(
            "The library {} is on {}, which is a camera card (it has a DCIM folder). \
             Pick a library folder on your external drive (`photosync drives` lists them).",
            p(library),
            p(volume)
        ),
        PlanError::LibraryOnSourceDevice {
            library,
            source_root,
        } => format!(
            "The library {} is on the same drive as source {}. Pick a library folder \
             on another drive (`photosync drives` lists them).",
            p(library),
            p(source_root)
        ),
        PlanError::LibraryOnSource {
            library,
            source_root,
        } => format!(
            "The library {} is inside source {}. Pick a library folder on your external \
             drive (`photosync drives` lists them).",
            p(library),
            p(source_root)
        ),
        PlanError::LibraryInDcim { library } => format!(
            "The library {} is inside a DCIM folder, where cameras keep their photos. \
             Pick a library folder on your external drive (`photosync drives` lists them).",
            p(library)
        ),
        PlanError::LibraryContainsSource {
            library,
            source_root,
        } => format!(
            "Source {} is inside the library {}. Pick a library folder that holds no source.",
            p(source_root),
            p(library)
        ),
        PlanError::Library(e) if e.source.kind() == io::ErrorKind::NotFound => format!(
            "The library folder {} does not exist. Create it on your external drive \
             first (`photosync drives` lists them), then run the sync again.",
            p(&e.path)
        ),
        PlanError::Library(e) => format!(
            "Cannot read the library {}: {}. Check the drive is plugged in and the \
             folder is readable, then run the sync again.",
            p(&e.path),
            e.source
        ),
        PlanError::Source(e) => format!(
            "Cannot read source {}: {}. Check the card is still plugged in, then run \
             the sync again.",
            p(&e.path),
            e.source
        ),
        // `flow::sync` turns a cancelled plan into its report.
        PlanError::Cancelled => "Cancelled before anything was copied.".to_string(),
    }
}

fn copy_refusal(err: &CopyError) -> String {
    let p = |path: &Path| path.display().to_string();
    match err {
        CopyError::LibraryDeviceChanged { library, .. }
        | CopyError::LibraryVolumeChanged { library, .. } => format!(
            "The library {} is now on another drive than when it was checked ({err}). \
             Nothing was copied. Check which drive is plugged in, then run the sync again.",
            p(library)
        ),
        CopyError::LibraryOnSourceDevice {
            library,
            source_root,
        } => format!(
            "The library {} is now on the same drive as source {}. Nothing was copied. \
             Check which drives are plugged in, then run the sync again.",
            p(library),
            p(source_root)
        ),
        CopyError::Library(e) => format!(
            "The library {} cannot be read any more: {}. Nothing was copied. Plug the \
             drive back in, then run the sync again.",
            p(&e.path),
            e.source
        ),
        // Only a plan and folder from two libraries; `flow::sync` never mixes them.
        CopyError::LibraryMismatch { .. } => {
            format!("Internal error: {err}. Nothing was copied.")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use photosync_core::{SourceFile, SourceProgress, Unreadable};

    use super::*;

    fn file(path: &str, media_type: Option<MediaType>, size: u64) -> SourceFile {
        SourceFile {
            path: PathBuf::from(path),
            media_type,
            size,
            modified: UNIX_EPOCH,
        }
    }

    fn entry(path: &str, outcome: FileOutcome) -> FileReport {
        let media = MediaType::from_path(Path::new(path));
        FileReport {
            file: file(path, media, 10),
            outcome,
        }
    }

    fn eio() -> io::Error {
        io::Error::from_raw_os_error(libc::EIO)
    }

    #[test]
    fn sizes_and_durations_read_as_people_write_them() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(1500), "1.5 kB");
        assert_eq!(size(2_340_000_000), "2.3 GB");
        assert_eq!(duration(Duration::from_secs(45)), "45s");
        assert_eq!(duration(Duration::from_secs(190)), "3m 10s");
        assert_eq!(duration(Duration::from_secs(3720)), "1h 02m");
    }

    fn progress(phase: Phase, done: (u64, u64), total: (u64, u64), elapsed_ms: u64) -> Progress {
        let c = |(files, bytes)| Counts { files, bytes };
        Progress {
            phase,
            done: c(done),
            total: c(total),
            sources: vec![SourceProgress {
                root: PathBuf::from("/Volumes/CARD"),
                done: c(done),
                total: c(total),
            }],
            elapsed: Duration::from_millis(elapsed_ms),
            eta: Some(Duration::from_secs(30)),
        }
    }

    #[test]
    fn progress_line_shows_files_bytes_percent_eta_and_each_source() {
        let mut p = progress(Phase::Copying, (1, 25_000_000), (4, 100_000_000), 10_000);
        assert_eq!(
            progress_line(&p),
            "copying 1/4 files, 25.0 MB of 100.0 MB (25%), ETA 30s"
        );

        p.sources.push(SourceProgress {
            root: PathBuf::from("/Volumes/CARD2"),
            done: Counts::default(),
            total: Counts { files: 2, bytes: 5 },
        });
        p.eta = None;
        assert!(
            progress_line(&p).ends_with("ETA -- [CARD 1/4, CARD2 0/2]"),
            "{}",
            progress_line(&p)
        );
    }

    /// The core sends an event per MiB; the text shows a phase's start and
    /// end, and in between at most one line per interval of event time.
    #[test]
    fn progress_is_throttled_by_event_time() {
        let mut text = ProgressText::new(false);
        let ev = |kind, ms| Event {
            kind,
            progress: progress(Phase::Copying, (0, 0), (2, 10), ms),
        };
        let shown: Vec<bool> = [
            ev(EventKind::Started, 0),
            ev(EventKind::Bytes, 10),
            ev(EventKind::Bytes, 999),
            ev(EventKind::Bytes, 1000),
            ev(EventKind::FileDone(PathBuf::from("/a")), 1500),
            ev(EventKind::Bytes, 2000),
            ev(EventKind::Finished, 2001),
        ]
        .iter()
        .map(|e| text.event(e).is_some())
        .collect();
        assert_eq!(shown, [true, false, false, true, false, true, true]);
    }

    #[test]
    fn progress_in_place_rewrites_one_line_and_ends_it() {
        let mut text = ProgressText::new(true);
        let ev = |kind| Event {
            kind,
            progress: progress(Phase::Planning, (0, 0), (2, 10), 0),
        };
        let start = text.event(&ev(EventKind::Started)).unwrap();
        let end = text.event(&ev(EventKind::Cancelled)).unwrap();
        assert!(start.starts_with("\r\x1b[Kcomparing with library 0/2") && !start.ends_with('\n'));
        assert!(end.ends_with(", cancelled\n"), "{end:?}");
    }

    #[test]
    fn a_phase_with_nothing_to_do_shows_nothing() {
        let mut text = ProgressText::new(false);
        let ev = Event {
            kind: EventKind::Started,
            progress: progress(Phase::Planning, (0, 0), (0, 0), 0),
        };
        assert_eq!(text.event(&ev), None);
    }

    #[test]
    fn every_file_that_is_not_ok_has_a_line_with_its_reason() {
        let ok = [
            entry(
                "/c/DCIM/A.RAF",
                FileOutcome::Copied {
                    to: PathBuf::from("/l/A.RAF"),
                },
            ),
            entry(
                "/c/DCIM/B.RAF",
                FileOutcome::AlreadySynced {
                    library_file: PathBuf::from("/l/B.RAF"),
                },
            ),
            entry(
                "/c/DCIM/C.RAF",
                FileOutcome::DuplicateInSession {
                    of: PathBuf::from("/d/C.RAF"),
                },
            ),
            entry("/c/DCIM/x.txt", FileOutcome::Ignored),
        ];
        for e in &ok {
            assert_eq!(problem_line(e), None, "{e:?}");
        }
        let line = |outcome| problem_line(&entry("/c/DCIM/D.RAF", outcome)).unwrap();
        assert_eq!(
            line(FileOutcome::Failed(FailReason::NameCollision {
                existing: PathBuf::from("/l/D.RAF")
            })),
            "  failed      /c/DCIM/D.RAF: name collision with /l/D.RAF"
        );
        assert!(
            line(FileOutcome::Failed(FailReason::Read(eio())))
                .starts_with("  failed      /c/DCIM/D.RAF: read error: ")
        );
        assert_eq!(
            line(FileOutcome::DuplicateOfFailed {
                of: PathBuf::from("/d/D.RAF")
            }),
            "  failed      /c/DCIM/D.RAF: same as /d/D.RAF, which failed"
        );
        assert_eq!(line(FileOutcome::Cancelled), "  cancelled   /c/DCIM/D.RAF");
        assert_eq!(
            line(FileOutcome::DuplicateOfCancelled {
                of: PathBuf::from("/d/D.RAF")
            }),
            "  cancelled   /c/DCIM/D.RAF: same as /d/D.RAF, which was cancelled"
        );
    }

    /// INV-6 keeps "not flushed" in failed and counts it apart; the text
    /// names it apart too, in the file's line, the totals and the result.
    #[test]
    fn not_flushed_is_shown_as_in_place_flush_not_confirmed() {
        let reason = FailReason::NotFlushed {
            to: PathBuf::from("/l/D.RAF"),
            error: eio(),
        };
        let source = SourceReport {
            root: PathBuf::from("/c"),
            files: vec![entry("/c/DCIM/D.RAF", FileOutcome::Failed(reason))],
            unreadable: Vec::new(),
        };

        let text = source_section(&source).join("\n");
        assert!(
            text.contains("  in place, flush not confirmed  /c/DCIM/D.RAF -> /l/D.RAF: "),
            "{text}"
        );
        assert!(
            text.contains("  of the failed, 1 in place, flush not confirmed"),
            "{text}"
        );
        assert!(!text.contains("  failed      /c/DCIM/D.RAF"), "{text}");
        let result = result_line(&source.totals(), std::slice::from_ref(&source));
        assert!(result.starts_with("Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable. 1 of the failed in place, flush not confirmed."), "{result}");
    }

    #[test]
    fn source_section_has_totals_per_type_ignored_kinds_and_unreadable_entries() {
        let source = SourceReport {
            root: PathBuf::from("/c"),
            files: vec![
                entry(
                    "/c/DCIM/A.RAF",
                    FileOutcome::Copied {
                        to: PathBuf::from("/l/A.RAF"),
                    },
                ),
                entry(
                    "/c/DCIM/A.JPG",
                    FileOutcome::AlreadySynced {
                        library_file: PathBuf::from("/l/A.JPG"),
                    },
                ),
                entry("/c/DCIM/B.JPG", FileOutcome::Cancelled),
                entry("/c/DCIM/._A.JPG", FileOutcome::Ignored),
                entry("/c/DCIM/x.txt", FileOutcome::Ignored),
                entry("/c/DCIM/y.txt", FileOutcome::Ignored),
            ],
            unreadable: vec![Unreadable {
                error: ScanError {
                    path: PathBuf::from("/c/DCIM/101"),
                    source: eio(),
                },
                contents_unknown: true,
            }],
        };

        let lines = source_section(&source);
        let expected = [
            "Source /c",
            "           copied   skipped   failed   cancelled",
            "  RAW           1         0        0           0",
            "  JPEG          0         1        0           1",
            "  all           1         1        0           1",
            "  size       10 B      10 B      0 B        10 B",
            "  ignored 3 files (.txt 2, hidden 1)",
            "  cancelled   /c/DCIM/B.JPG",
        ];
        assert_eq!(lines[..expected.len()], expected);
        assert_eq!(lines.len(), expected.len() + 1);
        assert!(lines[expected.len()].starts_with("  unreadable  /c/DCIM/101: "));
        assert!(lines[expected.len()].ends_with(" (contents unknown)"));
    }

    #[test]
    fn only_a_sync_with_nothing_failed_cancelled_or_unreadable_is_complete() {
        let with = |files, unreadable| SourceReport {
            root: PathBuf::from("/c"),
            files,
            unreadable,
        };
        let ok = with(
            vec![entry(
                "/c/DCIM/A.RAF",
                FileOutcome::Copied {
                    to: PathBuf::from("/l/A.RAF"),
                },
            )],
            vec![],
        );
        let cancelled = with(vec![entry("/c/DCIM/A.RAF", FileOutcome::Cancelled)], vec![]);
        let unreadable = with(
            vec![],
            vec![Unreadable {
                error: ScanError {
                    path: PathBuf::from("/c/DCIM/x"),
                    source: eio(),
                },
                contents_unknown: false,
            }],
        );
        let done = |s: &SourceReport| complete(&s.totals(), std::slice::from_ref(s));

        assert!(done(&ok));
        assert!(done(&with(vec![], vec![])));
        assert!(!done(&cancelled));
        assert!(!done(&unreadable));
        assert_eq!(
            result_line(&ok.totals(), std::slice::from_ref(&ok)),
            "Result: all 1 photo or video on the drive (1 copied, 0 already there)."
        );
        assert_eq!(
            result_line(&Tally::default(), &[]),
            "Result: no photos or videos found."
        );
        let two = with(
            vec![
                entry(
                    "/c/DCIM/A.RAF",
                    FileOutcome::Copied {
                        to: PathBuf::from("/l/A.RAF"),
                    },
                ),
                entry(
                    "/c/DCIM/B.RAF",
                    FileOutcome::AlreadySynced {
                        library_file: PathBuf::from("/l/B.RAF"),
                    },
                ),
            ],
            vec![],
        );
        assert_eq!(
            result_line(&two.totals(), std::slice::from_ref(&two)),
            "Result: all 2 photos or videos on the drive (1 copied, 1 already there)."
        );
    }

    /// Every refusal names the path and says what to do next.
    #[test]
    fn each_inv10_refusal_is_one_sentence_naming_the_paths() {
        let lib = || PathBuf::from("/Volumes/SSD/Photos");
        let src = || PathBuf::from("/Volumes/CARD");
        let cases = [
            (
                SyncError::Plan(PlanError::LibraryOnCard {
                    library: lib(),
                    volume: src(),
                }),
                "is on /Volumes/CARD, which is a camera card",
            ),
            (
                SyncError::Plan(PlanError::LibraryOnSourceDevice {
                    library: lib(),
                    source_root: src(),
                }),
                "is on the same drive as source /Volumes/CARD",
            ),
            (
                SyncError::Plan(PlanError::LibraryOnSource {
                    library: lib(),
                    source_root: src(),
                }),
                "is inside source /Volumes/CARD",
            ),
            (
                SyncError::Copy(CopyError::LibraryDeviceChanged {
                    library: lib(),
                    planned: 1,
                    now: 2,
                }),
                "is now on another drive than when it was checked",
            ),
            (
                SyncError::Copy(CopyError::LibraryVolumeChanged {
                    library: lib(),
                    planned: None,
                    now: Some(src()),
                }),
                "is now on another drive than when it was checked",
            ),
            (
                SyncError::Copy(CopyError::LibraryOnSourceDevice {
                    library: lib(),
                    source_root: src(),
                }),
                "is now on the same drive as source /Volumes/CARD",
            ),
            (
                SyncError::Copy(CopyError::Library(ScanError {
                    path: lib(),
                    source: eio(),
                })),
                "cannot be read any more",
            ),
            (
                SyncError::SourceTwice {
                    first: src(),
                    again: PathBuf::from("/Volumes/CARD/."),
                },
                "Source /Volumes/CARD/. is the same as source /Volumes/CARD. Give each card once.",
            ),
        ];
        for (err, part) in cases {
            let text = refusal(&err);
            assert!(text.contains(part), "{text}");
            assert!(!text.contains('\n'), "{text}");
        }
    }
}
