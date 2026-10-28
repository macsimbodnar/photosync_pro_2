//! Plain-text rendering for the CLI: progress lines, the report, and one
//! sentence per refusal. Strings only, apart from `StderrWatch`, so the
//! rules for what is shown are tested without a terminal.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

use photosync_core::{
    CopyError, Counts, Event, EventKind, FailReason, FileOutcome, FileReport, FolderError,
    MediaType, Phase, PlanError, Progress, Report, Run, RunEnd, ScanError, Session, SourceReport,
    SourceScan, SyncFolder, Tally, TempCleanup, Verdict,
};

use crate::flow::{SyncError, Watch};
use crate::system;

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
    /// The terminal's width in columns, read when a line is due.
    width: Option<fn() -> Option<usize>>,
}

impl ProgressText {
    pub fn new(in_place: bool) -> ProgressText {
        // A log gets fewer lines than a screen.
        let interval = Duration::from_millis(if in_place { 250 } else { 1000 });
        ProgressText {
            in_place,
            interval,
            last: None,
            width: None,
        }
    }

    /// Cuts each in-place line to the width `width` gives. A line that
    /// wraps cannot be rewritten in place: `\r` returns to its last row
    /// only, so each update would leave a row behind (S008 fast check).
    pub fn cut_to(mut self, width: fn() -> Option<usize>) -> ProgressText {
        self.width = Some(width);
        self
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
        if let (true, Some(width)) = (self.in_place, self.width.and_then(|w| w())) {
            // One column spare: a line that fills the last column leaves
            // some terminals' cursor on the next row.
            line = line.chars().take(width.saturating_sub(1)).collect();
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
            progress: ProgressText::new(io::stderr().is_terminal()).cut_to(system::stderr_width),
        }
    }
}

impl Watch for StderrWatch {
    fn scanned(&mut self, scan: &SourceScan) {
        system::say(&scanned(scan));
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
/// The rule is the core's, so the front-ends and the session agree.
pub fn complete(totals: &Tally, sources: &[SourceReport]) -> bool {
    photosync_core::is_complete(totals, sources)
}

/// The whole report: a section per source, overall totals, and a result
/// line saying whether the cards are safe to format, with the session's
/// library and folder under its title, so the report says where the
/// photos went.
pub fn report_in(report: &Report, library: &Path, folder: Option<&SyncFolder>) -> String {
    let folder = folder.map_or_else(
        || "none (nothing to copy)".to_string(),
        |f| f.path().display().to_string(),
    );
    report_with(
        report,
        vec![
            format!("Library {}", library.display()),
            format!("Folder  {folder}"),
        ],
    )
}

fn report_with(report: &Report, header: Vec<String>) -> String {
    let mut out = Vec::new();
    out.push("Report".to_string());
    out.extend(header);
    out.extend(report_body(report));
    out.push(String::new());
    out.push(result_line(&report.totals(), report.sources()));
    out.join("\n") + "\n"
}

/// A report's sections, without its title and result line.
fn report_body(report: &Report) -> Vec<String> {
    let mut out = Vec::new();
    for source in report.sources() {
        out.push(String::new());
        out.extend(source_section(source));
    }
    if report.sources().len() > 1 {
        out.push(String::new());
        out.push("All sources".to_string());
        out.extend(table(&report.by_type(), &report.totals()));
    }
    out.extend(library_section(report.library_failures(), report.temps()));
    out
}

fn library_and_folder(session: &Session) -> Vec<String> {
    let folder = session.folder().map_or_else(
        || "none (nothing to copy)".to_string(),
        |f| f.path().display().to_string(),
    );
    vec![
        format!("Library {}", session.library().display()),
        format!("Folder  {folder}"),
    ]
}

/// "sync 2 (/Volumes/A, /Volumes/B)": a run, by its place in the session
/// and its cards, so the user knows which cards are not safe.
fn run_name(index: usize, run: &Run) -> String {
    format!("sync {} ({})", index + 1, cards(run))
}

/// What a run left undone, in a few words, or "complete"; either way
/// naming each card a file could not be read on and was copied from its
/// twin instead (S023).
fn run_state(run: &Run) -> String {
    match &run.end {
        RunEnd::Report(report) => {
            let totals = report.totals();
            let mut state = if report.is_complete() {
                "complete".to_string()
            } else {
                let unreadable: usize = report.sources().iter().map(|s| s.unreadable.len()).sum();
                format!(
                    "{} failed, {} cancelled, {} unreadable",
                    totals.failed.files, totals.cancelled.files, unreadable
                )
            };
            if let Some(warning) =
                unread_warning(totals.unread.files, &unread_cards(report.sources()))
            {
                state.push_str(", but ");
                state.push_str(&warning);
            }
            state
        }
        RunEnd::Refused(SyncError::Report(_)) => "copied, but its report does not add up".into(),
        RunEnd::Refused(_) => "refused, nothing copied".to_string(),
        RunEnd::Crashed(what) => format!("stopped with an internal error ({what})"),
    }
}

/// The cards of `sources` with a file that could not be read and was
/// copied from its twin, in order, each once.
fn unread_cards<'r>(sources: impl IntoIterator<Item = &'r SourceReport>) -> Vec<&'r Path> {
    let mut cards: Vec<&Path> = Vec::new();
    for source in sources.into_iter().filter(|s| s.totals().unread.files > 0) {
        if !cards.contains(&source.root.as_path()) {
            cards.push(&source.root);
        }
    }
    cards
}

/// "1 could not be read on /Volumes/A and was copied from another card:
/// check that card", or `None` when every file was read. A complete sync
/// still says it, as it is the only word that a card had trouble.
fn unread_warning(files: u64, cards: &[&Path]) -> Option<String> {
    if files == 0 {
        return None;
    }
    let named: Vec<String> = cards.iter().map(|c| c.display().to_string()).collect();
    let (was, check) = match (files, cards.len()) {
        (1, _) => ("was", "that card"),
        (_, 1) => ("were", "that card"),
        _ => ("were", "those cards"),
    };
    Some(format!(
        "{files} could not be read on {} and {was} copied from another card: check {check}",
        named.join(", ")
    ))
}

/// Each run that was not complete, named, with what it left undone; `None`
/// when every run was complete.
pub fn not_complete(session: &Session) -> Option<String> {
    let each: Vec<String> = session
        .not_complete()
        .map(|(i, run)| format!("{}: {}", run_name(i, run), run_state(run)))
        .collect();
    (!each.is_empty()).then(|| each.join("; "))
}

/// The session's result line: the worst of its syncs. A session of one
/// sync with a report says what that sync's report says.
pub fn session_result(session: &Session) -> String {
    if let [
        Run {
            end: RunEnd::Report(report),
            ..
        },
    ] = session.runs()
    {
        return result_line(&report.totals(), report.sources());
    }
    let syncs = session.runs().len() as u64;
    let totals = session.totals();
    let mut line = match not_complete(session) {
        None => format!(
            "Result: all {} on the drive ({} copied, {} already there), over {}.",
            plural(totals.supported().files, MEDIA.0, MEDIA.1),
            totals.copied.files,
            totals.skipped.files,
            plural(syncs, "sync", "syncs")
        ),
        Some(each) => format!(
            "Result: NOT COMPLETE: {} of {} not complete: {each}. Do not format the cards \
             of those syncs. Fix the cause and sync them again: files already on the \
             drive are skipped.",
            session.not_complete().count(),
            plural(syncs, "sync", "syncs")
        ),
    };
    let reports = session.runs().iter().filter_map(|run| match &run.end {
        RunEnd::Report(report) => Some(report),
        _ => None,
    });
    if let Some(warning) = unread_warning(
        totals.unread.files,
        &unread_cards(reports.flat_map(Report::sources)),
    ) {
        line.push(' ');
        line.push_str(&warning);
        line.push('.');
    }
    line
}

/// The sentence for a sync that stopped with no result.
pub fn crash(what: &str) -> String {
    format!(
        "The copy stopped with an internal error ({what}), so nothing says which files are \
         on the drive. Do not format the card. Run the same sync again: files already on \
         the drive are skipped."
    )
}

/// The whole session's report: each sync's report, refusal or crash in
/// order, the totals of all of them and the session's result. A session of
/// one sync gives that sync's report, word for word as the CLI prints it;
/// `None` when that one sync has no report (its sentence says why).
pub fn session_report(session: &Session) -> Option<String> {
    match session.runs() {
        []
        | [
            Run {
                end: RunEnd::Refused(_) | RunEnd::Crashed(_),
                ..
            },
        ] => None,
        [
            Run {
                end: RunEnd::Report(report),
                ..
            },
        ] => Some(report_in(report, session.library(), session.folder())),
        runs => {
            let mut out = vec!["Report".to_string()];
            out.extend(library_and_folder(session));
            for (i, run) in runs.iter().enumerate() {
                out.push(String::new());
                out.push(format!("Sync {} of {}: {}", i + 1, runs.len(), cards(run)));
                match &run.end {
                    RunEnd::Report(report) => {
                        out.extend(report_body(report));
                        out.push(format!(
                            "  {}",
                            result_line(&report.totals(), report.sources())
                        ));
                    }
                    RunEnd::Refused(e) => out.push(format!("  {}", refusal(e))),
                    RunEnd::Crashed(what) => out.push(format!("  {}", crash(what))),
                }
            }
            out.extend(all_syncs(session));
            Some(out.join("\n") + "\n")
        }
    }
}

/// The CLI's last words after several syncs, each already reported: one
/// line per sync, the totals of all of them, and the session's result.
pub fn session_summary(session: &Session) -> String {
    let runs = session.runs();
    let mut out = vec![format!(
        "Session of {}",
        plural(runs.len() as u64, "sync", "syncs")
    )];
    out.extend(library_and_folder(session));
    for (i, run) in runs.iter().enumerate() {
        out.push(format!("  {}: {}", run_name(i, run), run_state(run)));
    }
    out.extend(all_syncs(session));
    out.join("\n") + "\n"
}

fn all_syncs(session: &Session) -> Vec<String> {
    let mut out = vec![String::new(), "All syncs".to_string()];
    out.extend(table(&session.by_type(), &session.totals()));
    out.push(String::new());
    out.push(session_result(session));
    out
}

fn cards(run: &Run) -> String {
    let cards: Vec<String> = run
        .sources
        .iter()
        .map(|s| s.display().to_string())
        .collect();
    cards.join(", ")
}

/// What to say on stderr when the session is over, beside its report: the
/// refusal or the crash of a session of one sync, or, after several, the
/// result naming each sync that was not complete; `None` when nothing is
/// left to say.
pub fn session_message(session: &Session) -> Option<String> {
    match session.runs() {
        [] => None,
        [run] => match &run.end {
            RunEnd::Report(_) => None,
            RunEnd::Refused(e) => Some(refusal(e)),
            RunEnd::Crashed(what) => Some(crash(what)),
        },
        _ if session.verdict() == Verdict::Complete => None,
        _ => Some(session_result(session)),
    }
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
    if totals.unread.files > 0 {
        out.push(format!(
            "  of the skipped, {} could not be read and {} copied from another card",
            totals.unread.files,
            if totals.unread.files == 1 { "was" } else { "were" }
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

/// One line for a file that is not copied or skipped, with its reason, and
/// for one skipped because its card could not be read (its twin was
/// copied); `None` for one that is OK.
pub fn problem_line(entry: &FileReport) -> Option<String> {
    let path = entry.file.path.display();
    Some(match &entry.outcome {
        FileOutcome::Copied { .. }
        | FileOutcome::AlreadySynced { .. }
        | FileOutcome::DuplicateInSession { .. }
        | FileOutcome::Ignored => return None,
        FileOutcome::DuplicateUnread { of, error } => format!(
            "  read error  {path}: {error}; copied from its twin {}",
            of.display()
        ),
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
        let mut line = format!(
            "Result: all {} on the drive ({} copied, {} already there).",
            plural(supported.files, MEDIA.0, MEDIA.1),
            totals.copied.files,
            totals.skipped.files
        );
        // Every photo is safe, but a card had a read error.
        line.push_str(&unread_sentence(totals, sources));
        return line;
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
    // Named here too, so what failed does not hide a card that only had a
    // read error (S024).
    line.push_str(&unread_sentence(totals, sources));
    line.push_str(
        " Do not format the card. Fix the cause and run the same sync again: \
         files already on the drive are skipped.",
    );
    line
}

/// " 1 could not be read on /Volumes/A and was copied from another card:
/// check that card.", or "" when every file was read.
fn unread_sentence(totals: &Tally, sources: &[SourceReport]) -> String {
    unread_warning(totals.unread.files, &unread_cards(sources))
        .map_or_else(String::new, |warning| format!(" {warning}."))
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
        // The session turns a cancelled plan into its report.
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
        // Only a plan and folder from two libraries; a session never mixes them.
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

    /// S019: two cards copy at once, so their files' events interleave.
    /// Each line is drawn from its event's snapshot alone, so it shows
    /// each card's own count, whatever file the event is of.
    #[test]
    fn progress_of_two_cards_at_once_shows_each_card() {
        let mut text = ProgressText::new(false);
        let ev = |kind, (a, b): (u64, u64), ms| {
            let mut p = progress(Phase::Copying, (a + b, 0), (4, 40), ms);
            p.sources[0].done = Counts { files: a, bytes: 0 };
            p.sources[0].total = Counts {
                files: 2,
                bytes: 20,
            };
            p.sources.push(SourceProgress {
                root: PathBuf::from("/Volumes/CARD2"),
                done: Counts { files: b, bytes: 0 },
                total: Counts {
                    files: 2,
                    bytes: 20,
                },
            });
            Event { kind, progress: p }
        };
        let lines: Vec<String> = [
            ev(EventKind::Started, (0, 0), 0),
            ev(
                EventKind::FileStarted(PathBuf::from("/Volumes/CARD/a")),
                (0, 0),
                1,
            ),
            ev(
                EventKind::FileStarted(PathBuf::from("/Volumes/CARD2/b")),
                (0, 0),
                2,
            ),
            ev(
                EventKind::FileDone(PathBuf::from("/Volumes/CARD2/b")),
                (0, 1),
                3000,
            ),
            ev(
                EventKind::FileDone(PathBuf::from("/Volumes/CARD/a")),
                (1, 1),
                6000,
            ),
            ev(EventKind::Finished, (2, 2), 7000),
        ]
        .iter()
        .filter_map(|e| text.event(e))
        .collect();
        let each: Vec<&str> = lines.iter().map(|l| &l[l.find('[').unwrap()..]).collect();
        assert_eq!(
            each,
            [
                "[CARD 0/2, CARD2 0/2]\n",
                "[CARD 0/2, CARD2 1/2]\n",
                "[CARD 1/2, CARD2 1/2]\n",
                "[CARD 2/2, CARD2 2/2]\n"
            ]
        );
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

    /// A line wider than the terminal would wrap, and `\r` could not
    /// rewrite it: each update would leave a row behind.
    #[test]
    fn progress_in_place_is_cut_to_the_terminal_width() {
        let ev = Event {
            kind: EventKind::Started,
            progress: progress(Phase::Copying, (1, 25_000_000), (4, 100_000_000), 0),
        };
        let full = progress_line(&ev.progress);
        assert!(full.len() > 20);

        let mut narrow = ProgressText::new(true).cut_to(|| Some(20));
        assert_eq!(
            narrow.event(&ev).unwrap(),
            format!("\r\x1b[K{}", &full[..19])
        );
        // Unknown width, or not a terminal: the whole line.
        let mut unknown = ProgressText::new(true).cut_to(|| None);
        assert_eq!(unknown.event(&ev).unwrap(), format!("\r\x1b[K{full}"));
        let mut log = ProgressText::new(false).cut_to(|| Some(20));
        assert_eq!(log.event(&ev).unwrap(), format!("{full}\n"));
        // The end of a phase is cut too, and still ends its line.
        let mut end = ProgressText::new(true).cut_to(|| Some(10));
        let done = Event {
            kind: EventKind::Finished,
            ..ev
        };
        assert_eq!(
            end.event(&done).unwrap(),
            format!("\r\x1b[K{}\n", &full[..9])
        );
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

    /// F07: a file read from its twin on another card is skipped, as the
    /// photo is on the drive, but its card's read error is shown: on its
    /// line, under the totals and in the result, which stays complete.
    #[test]
    fn a_file_copied_from_its_twin_shows_its_read_error() {
        let unread = FileOutcome::DuplicateUnread {
            of: PathBuf::from("/d/DCIM/D.RAF"),
            error: eio(),
        };
        let source = SourceReport {
            root: PathBuf::from("/c"),
            files: vec![entry("/c/DCIM/D.RAF", unread)],
            unreadable: Vec::new(),
        };

        let text = source_section(&source).join("\n");
        let line = format!(
            "  read error  /c/DCIM/D.RAF: {}; copied from its twin /d/DCIM/D.RAF",
            eio()
        );
        assert!(text.contains(&line), "{text}");
        assert!(
            text.contains("  of the skipped, 1 could not be read and was copied from another card"),
            "{text}"
        );
        let result = result_line(&source.totals(), std::slice::from_ref(&source));
        assert_eq!(
            result,
            "Result: all 1 photo or video on the drive (0 copied, 1 already there). \
             1 could not be read on /c and was copied from another card: check that card."
        );
    }

    /// S024: the one-sync result names each card a file could not be read
    /// on, by name, whether the sync is complete or not: a not-complete
    /// sync must not hide that another card had a read error too.
    #[test]
    fn the_one_sync_result_names_every_unread_card_complete_or_not() {
        let unread = |root: &str| {
            entry(
                &format!("{root}/DCIM/D.RAF"),
                FileOutcome::DuplicateUnread {
                    of: PathBuf::from("/V/T/DCIM/D.RAF"),
                    error: eio(),
                },
            )
        };
        let failed = || {
            entry(
                "/V/C/DCIM/E.RAF",
                FileOutcome::Failed(FailReason::Read(eio())),
            )
        };
        let source = |root: &str, files| SourceReport {
            root: PathBuf::from(root),
            files,
            unreadable: Vec::new(),
        };
        // `Tally` has no public sum: the totals are of one source with
        // every file.
        let result = |sources: &[SourceReport], all: Vec<FileReport>| {
            result_line(&source("/all", all).totals(), sources)
        };

        let both = result(
            &[
                source("/V/A", vec![unread("/V/A")]),
                source("/V/B", vec![unread("/V/B")]),
            ],
            vec![unread("/V/A"), unread("/V/B")],
        );
        assert!(both.starts_with("Result: all 2 "), "{both}");
        assert!(
            both.ends_with(
                " 2 could not be read on /V/A, /V/B and were copied from another card: \
                 check those cards."
            ),
            "{both}"
        );

        let not = result(
            &[
                source("/V/A", vec![unread("/V/A")]),
                source("/V/C", vec![failed()]),
            ],
            vec![unread("/V/A"), failed()],
        );
        assert!(
            not.starts_with(
                "Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable. \
                 1 could not be read on /V/A and was copied from another card: check that card. \
                 Do not format the card."
            ),
            "{not}"
        );
    }

    /// S023: in a session of several syncs, a sync whose only problem is
    /// a file copied from its twin is not plain "complete": its state and
    /// the session's result name the card that could not be read, whether
    /// the session is complete or not. Real syncs, so on the drive image.
    #[test]
    fn a_session_names_the_card_a_file_was_copied_from_its_twin_for() {
        use crate::common::{LoseOnCopy, OnDrive, TempDir, at, test_drive, twin_cards, write_file};
        use photosync_core::{Cancel, Session};

        struct Quiet;
        impl Watch for Quiet {}

        let Some(drive) =
            test_drive("a_session_names_the_card_a_file_was_copied_from_its_twin_for")
        else {
            return;
        };
        let Some((card_a, card_b)) =
            twin_cards("a_session_names_the_card_a_file_was_copied_from_its_twin_for")
        else {
            return;
        };
        let library = OnDrive::new(&drive, "S023-cli");
        let dir = TempDir::new();
        // The twins on two images: on one device they would be no twins
        // (DEC-038).
        let (on_a, on_b) = (
            OnDrive::new(&card_a, "S023-cli"),
            OnDrive::new(&card_b, "S023-cli"),
        );
        let (a, b, c) = (on_a.0.clone(), on_b.0.clone(), dir.0.join("c"));
        let lost = write_file(&a, "DCIM/100_FUJI/DSCF0001.RAF", b"twin", at(1_700_000_000));
        write_file(&b, "DCIM/100_FUJI/DSCF0001.RAF", b"twin", at(1_700_000_000));
        write_file(
            &c,
            "DCIM/100_FUJI/DSCF0002.RAF",
            b"other",
            at(1_700_000_000),
        );
        let mut session = Session::new(&library.0, None).unwrap();

        let mut quiet = Quiet;
        let mut lose = LoseOnCopy {
            path: lost,
            inner: &mut quiet,
        };
        session.sync(&[a.clone(), b.clone()], &Cancel::new(), &mut lose);
        session.sync(std::slice::from_ref(&c), &Cancel::new(), &mut Quiet);

        let warning = format!(
            "1 could not be read on {} and was copied from another card: check that card",
            a.display()
        );
        let summary = session_summary(&session);
        let state = format!(
            "  sync 1 ({}, {}): complete, but {warning}\n",
            a.display(),
            b.display()
        );
        assert!(summary.contains(&state), "{summary}");
        assert!(
            summary.contains(&format!("  sync 2 ({}): complete\n", c.display())),
            "{summary}"
        );
        let result = session_result(&session);
        assert!(result.starts_with("Result: all 3 "), "{result}");
        assert!(
            result.ends_with(&format!(" over 2 syncs. {warning}.")),
            "{result}"
        );

        session.sync(&[dir.0.join("gone")], &Cancel::new(), &mut Quiet);
        let result = session_result(&session);
        assert!(
            result.starts_with("Result: NOT COMPLETE: 1 of 3 syncs"),
            "{result}"
        );
        assert!(result.ends_with(&format!(" {warning}.")), "{result}");
    }

    #[test]
    fn the_unread_warning_names_each_card_and_counts_the_files() {
        let (a, b) = (Path::new("/V/A"), Path::new("/V/B"));
        assert_eq!(unread_warning(0, &[]), None);
        assert_eq!(
            unread_warning(1, &[a]).unwrap(),
            "1 could not be read on /V/A and was copied from another card: check that card"
        );
        assert_eq!(
            unread_warning(2, &[a]).unwrap(),
            "2 could not be read on /V/A and were copied from another card: check that card"
        );
        assert_eq!(
            unread_warning(2, &[a, b]).unwrap(),
            "2 could not be read on /V/A, /V/B and were copied from another card: check those \
             cards"
        );
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
