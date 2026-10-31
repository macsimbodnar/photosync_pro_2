//! Draws an `App` with ratatui. Reads the model only: every rule is in
//! `app`, every sentence about the sync in `text`.

use std::path::{Path, PathBuf};

use photosync_core::Progress;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Gauge, List, ListItem, ListState, Paragraph};

use crate::app::{App, Outcome, Screen};
use crate::text;

/// Rows the report keeps under the result's box: with fewer, little of it
/// could be read at a time, so the result scrolls with it instead.
const MIN_REPORT_ROWS: usize = 3;

/// Rows of a gauge in its box; with too little room, the words come first.
const GAUGE_ROWS: u16 = 3;
const MIN_ROWS_UNDER_GAUGE: u16 = 2;
/// A list's borders and one row of it.
const MIN_LIST_ROWS: u16 = 3;

fn screen_areas(app: &App, area: Rect) -> [Rect; 4] {
    let hint = app.hint().map_or(1, |h| wrapped(h, area.width).len());
    let keys = wrapped(&keys_help(app), area.width).len();
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(hint as u16),
        Constraint::Length(keys as u16),
    ])
    .areas(area)
}

pub fn render(app: &App, report: &mut ReportCache, frame: &mut Frame) {
    let [title, body, hint, keys] = screen_areas(app, frame.area());

    frame.render_widget(
        Line::from(vec![
            Span::from(" phoosync pro 2 ").bold().reversed(),
            Span::from(format!("  {}", heading(app))),
        ]),
        title,
    );
    match app.screen() {
        Screen::Sources => list_screen(app, frame, body, "Camera cards"),
        Screen::Drive => list_screen(app, frame, body, "Drives"),
        Screen::Library => field(frame, body, "Library folder", app.library()),
        Screen::Name => field(frame, body, "Name (optional)", app.name()),
        Screen::Confirm => frame.render_widget(Block::bordered(), body),
        Screen::Running => running(app, frame, body),
        Screen::Refused | Screen::Report => outcome(app, report, frame, body),
    }
    if let Some(column) = column(app, body) {
        draw_column(app, frame, column);
    }
    if let Some(h) = app.hint() {
        let rows = styled_rows(h, hint.width, Style::new().yellow());
        frame.render_widget(Paragraph::new(rows), hint);
    }
    let rows = styled_rows(&keys_help(app), keys.width, Style::new().dim());
    frame.render_widget(Paragraph::new(rows), keys);
}

fn heading(app: &App) -> String {
    if app.in_session() {
        let next = app.syncs() + 1;
        match app.screen() {
            Screen::Sources => return format!("Sync {next}: pick the next card"),
            Screen::Confirm => return format!("Sync {next}: check and start"),
            _ => {}
        }
    }
    match app.screen() {
        Screen::Sources => "1/5  Pick the cards to copy from",
        Screen::Drive => "2/5  Pick the drive to copy to",
        Screen::Library => "3/5  Library folder on the drive",
        Screen::Name => "4/5  Name this sync",
        Screen::Confirm => "5/5  Check and start",
        Screen::Running => "Copying",
        Screen::Refused => "Stopped, no report",
        Screen::Report => "Report",
    }
    .to_string()
}

/// Must match what `App::update` does on each screen.
pub fn keys_help(app: &App) -> String {
    let keys = match app.screen() {
        Screen::Sources => " ↑↓ move   Space pick   Enter next   r look again   q quit",
        Screen::Drive => " ↑↓ move   Enter pick   Esc back   r look again   q quit",
        Screen::Library | Screen::Name => " type to edit   Enter next   Esc back   Ctrl-C quit",
        Screen::Confirm => " Enter start copying   Esc back   q quit",
        Screen::Running => " c cancel   Ctrl-C twice quits at once",
        // Before the session starts there is no session for a next card to join.
        Screen::Refused if app.in_session() => " Enter back, to fix it   n next card   q quit",
        Screen::Refused => " Enter back, to fix it   q quit",
        Screen::Report => " ↑↓ PgUp PgDn scroll   n next card   q quit",
    };
    // Only when there is more: most screens fit, and these keys then do nothing.
    match app.screen() {
        Screen::Report => keys.to_string(),
        _ if !app.can_scroll() => keys.to_string(),
        // Here the arrows move the cursor.
        Screen::Sources | Screen::Drive => format!(" PgUp PgDn scroll  {keys}"),
        _ => format!(" ↑↓ PgUp PgDn scroll  {keys}"),
    }
}

/// `text` wrapped to `width`, in `style`.
fn styled_rows(text: &str, width: u16, style: Style) -> Vec<Line<'static>> {
    wrapped(text, width)
        .into_iter()
        .map(|row| Line::from(row).style(style))
        .collect()
}

/// The words of a screen, and where they go. They scroll when they do not fit,
/// so none is cut, however small the terminal.
struct Column {
    rows: Vec<Line<'static>>,
    area: Rect,
}

/// The report has its own: it can be too long to make `Line`s of all its rows.
fn column(app: &App, body: Rect) -> Option<Column> {
    let boxed = Block::bordered().inner(body);
    let (rows, area) = match app.screen() {
        Screen::Sources | Screen::Drive => {
            let notes = note_rows(app, body.width);
            let inner = list_inner_rows(app, body.width);
            let [list, area] = list_areas(app, body, inner, notes.len());
            let mut rows = Vec::new();
            if list.height == 0 {
                rows = styled_rows(app.empty_list_note(), body.width, Style::new());
            }
            rows.extend(notes);
            (rows, area)
        }
        Screen::Library | Screen::Name => {
            let [_, help] = field_areas(body);
            let rows = field_help(app)
                .iter()
                .flat_map(|h| styled_rows(h, help.width, Style::new()))
                .collect();
            (rows, help)
        }
        Screen::Confirm => (confirm_rows(app, boxed.width), boxed),
        Screen::Running => {
            let [_, words] = running_areas(body);
            (running_rows(app, words.width), words)
        }
        Screen::Refused => match app.outcome() {
            Some(Outcome::Refused { sentence }) => (
                styled_rows(sentence, boxed.width, Style::new().red()),
                boxed,
            ),
            _ => return None,
        },
        Screen::Report => return None,
    };
    Some(Column { rows, area })
}

fn draw_column(app: &App, frame: &mut Frame, column: Column) {
    let height = usize::from(column.area.height);
    // A drawing can come before the model hears of a resize.
    let first = app.scroll().min(column.rows.len().saturating_sub(height));
    let shown: Vec<Line> = column.rows.into_iter().skip(first).take(height).collect();
    frame.render_widget(Paragraph::new(shown), column.area);
}

fn list_screen(app: &App, frame: &mut Frame, area: Rect, title: &str) {
    let block = Block::bordered().title(title.to_string());
    let notes = note_rows(app, area.width).len();
    let items: Vec<ListItem> = list_paths(app)
        .iter()
        .map(|path| list_item(app, path, area))
        .collect();
    if !items.is_empty() {
        let selected = items.get(app.cursor()).map_or(1, ListItem::height);
        let [list_area, _] = list_areas(app, area, selected, notes);
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol("> ");
        let mut state = ListState::default().with_selected(Some(app.cursor()));
        frame.render_stateful_widget(list, list_area, &mut state);
    } else {
        let sentence = empty_list_rows(app, area.width);
        let [list_area, _] = list_areas(app, area, sentence.len(), notes);
        frame.render_widget(Paragraph::new(sentence).block(block), list_area);
    }
}

fn list_paths(app: &App) -> &[PathBuf] {
    match app.screen() {
        Screen::Drive => app.drives(),
        _ => app.sources(),
    }
}

/// The empty list's sentence, as its box holds it.
fn empty_list_rows(app: &App, width: u16) -> Vec<Line<'static>> {
    styled_rows(app.empty_list_note(), width.saturating_sub(2), Style::new())
}

/// The rows the list must show whole inside its box. The row under the cursor
/// only: the list follows the cursor, so that reaches every row.
fn list_inner_rows(app: &App, width: u16) -> usize {
    let paths = list_paths(app);
    if paths.is_empty() {
        return empty_list_rows(app, width).len();
    }
    paths
        .get(app.cursor())
        .map_or(1, |path| list_rows(app, path, width).len())
}

/// The list's box first, `inner` rows in it, and the rest to the notes, which
/// scroll. The notes keep one row all the same, or they could never be read;
/// only a body of `MIN_LIST_ROWS` has none to give. An empty list too low for
/// its box and a row of notes has no box: its sentence goes first in the notes.
fn list_areas(app: &App, area: Rect, inner: usize, notes: usize) -> [Rect; 2] {
    let height = usize::from(area.height);
    let boxed = inner + 2;
    if list_paths(app).is_empty() && boxed + usize::from(notes > 0) > height {
        return [Rect { height: 0, ..area }, area];
    }
    let room = height
        .saturating_sub(boxed)
        .max(usize::from(area.height > MIN_LIST_ROWS));
    Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(notes.min(room) as u16),
    ])
    .areas(area)
}

fn note_rows(app: &App, width: u16) -> Vec<Line<'static>> {
    app.notes()
        .iter()
        .flat_map(|n| styled_rows(n, width, Style::new().yellow()))
        .collect()
}

/// A list's row, in as many rows as its path needs: two cards can differ only
/// at the end. The list's borders and its cursor take four columns.
fn list_rows(app: &App, path: &Path, width: u16) -> Vec<String> {
    let mark = match app.screen() {
        Screen::Drive if app.drive() == Some(path) => "(*)",
        Screen::Drive => "( )",
        _ if app.is_picked(path) => "[x]",
        _ => "[ ]",
    };
    wrapped(&format!("{mark} {}", path.display()), width.saturating_sub(4))
}

fn list_item(app: &App, path: &Path, area: Rect) -> ListItem<'static> {
    let rows: Vec<Line> = list_rows(app, path, area.width)
        .into_iter()
        .map(Line::from)
        .collect();
    ListItem::new(rows)
}

fn field_areas(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area)
}

fn field_help(app: &App) -> Vec<String> {
    if app.screen() == Screen::Library {
        return vec![
            "The folder on the drive that holds your photos. It must exist:".into(),
            "phoosync pro 2 never creates it. Photos already anywhere in it are skipped.".into(),
        ];
    }
    vec![
        format!(
            "New photos go to {}/{}/RAW, JPEG, ...",
            app.library().trim(),
            app.folder_preview()
        ),
        "A folder with this name from an earlier sync is reused. Empty: no name.".into(),
    ]
}

fn field(frame: &mut Frame, area: Rect, title: &str, value: &str) {
    let [input, _] = field_areas(area);
    frame.render_widget(
        Paragraph::new(value.to_string()).block(Block::bordered().title(title.to_string())),
        input,
    );
    let width = value.chars().count() as u16;
    frame.set_cursor_position(Position::new(
        (input.x + 1 + width).min(input.right().saturating_sub(2)),
        input.y + 1,
    ));
}

fn confirm_rows(app: &App, width: u16) -> Vec<Line<'static>> {
    let bold = Style::new().bold();
    let plain = Style::new();
    let mut rows = styled_rows("Copy from:", width, bold);
    for s in app.picked() {
        rows.extend(styled_rows(&format!("  {}", s.display()), width, plain));
    }
    rows.push(Line::from(""));
    rows.extend(styled_rows("Into the library:", width, bold));
    let library = format!("  {}", app.library().trim());
    rows.extend(styled_rows(&library, width, plain));
    rows.push(Line::from(""));
    rows.extend(styled_rows("New photos go to the folder:", width, bold));
    let fixed = if app.session_folder().is_some() {
        "(this session's folder)"
    } else {
        "(the date of the earliest new photo)"
    };
    let folder = format!("  {}   {fixed}", app.folder_preview());
    rows.extend(styled_rows(&folder, width, plain));
    if app.in_session() {
        rows.push(Line::from(""));
        let stays = format!(
            "Sync {} of this session: the library and name stay as they are. Quit (q) to change them.",
            app.syncs() + 1
        );
        rows.extend(styled_rows(&stays, width, plain));
    }
    rows.push(Line::from(""));
    rows.extend(styled_rows(
        "The cards are only read. Photos already in the library are skipped.",
        width,
        plain,
    ));
    rows
}

/// The gauge stays on top, so the progress is in sight whatever scrolls under it.
fn running_areas(area: Rect) -> [Rect; 2] {
    let gauge = if area.height >= GAUGE_ROWS + MIN_ROWS_UNDER_GAUGE {
        GAUGE_ROWS
    } else {
        0
    };
    Layout::vertical([Constraint::Length(gauge), Constraint::Min(0)]).areas(area)
}

fn running(app: &App, frame: &mut Frame, area: Rect) {
    let [gauge, _] = running_areas(area);
    if let Some(p) = app.running().and_then(|run| run.progress.as_ref()) {
        frame.render_widget(
            Gauge::default()
                .block(Block::bordered())
                .gauge_style(Style::new().fg(Color::Green))
                .ratio(ratio(p.done.bytes, p.total.bytes)),
            gauge,
        );
    }
}

/// The progress first, as it is what changes; the scans of the cards after.
fn running_rows(app: &App, width: u16) -> Vec<Line<'static>> {
    let Some(run) = app.running() else {
        return Vec::new();
    };
    let plain = Style::new();
    let mut rows = match &run.progress {
        Some(p) => styled_rows(&text::progress_line(p), width, plain),
        None => styled_rows("scanning the cards...", width, plain),
    };
    if run.cancelling {
        rows.extend(styled_rows(
            "Cancelling: the files being copied are finished or dropped, then the report shows.",
            width,
            Style::new().yellow(),
        ));
    }
    if let Some(p) = &run.progress {
        for line in per_source(p) {
            rows.extend(styled_rows(&line, width, plain));
        }
    }
    for line in &run.scanned {
        rows.extend(styled_rows(line, width, plain));
    }
    rows
}

fn per_source(p: &Progress) -> Vec<String> {
    if p.sources.len() < 2 {
        return Vec::new();
    }
    p.sources
        .iter()
        .map(|s| {
            format!(
                "  {}  {}/{} files, {} of {}",
                name_of(&s.root),
                s.done.files,
                s.total.files,
                text::size(s.done.bytes),
                text::size(s.total.bytes)
            )
        })
        .collect()
}

fn name_of(root: &Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |n| n.to_string_lossy().into(),
    )
}

fn ratio(done: u64, total: u64) -> f64 {
    if total == 0 {
        1.0
    } else {
        (done as f64 / total as f64).clamp(0.0, 1.0)
    }
}

fn outcome(app: &App, report: &mut ReportCache, frame: &mut Frame, area: Rect) {
    match app.outcome() {
        // Its sentence is a `column`.
        Some(Outcome::Refused { .. }) => frame.render_widget(Block::bordered(), area),
        Some(Outcome::Report {
            lines,
            result,
            complete,
        }) => {
            let color = if *complete { Color::Green } else { Color::Red };
            let view = report.view(app, result, lines, area);
            if view.result_in_rows == 0 {
                let boxed: Vec<Line> = view.boxed.iter().map(|r| Line::from(r.as_str())).collect();
                frame.render_widget(
                    Paragraph::new(boxed)
                        .fg(color)
                        .bold()
                        .block(Block::bordered()),
                    view.top,
                );
            }
            // A drawing can come before the model hears of a resize.
            let last = view.rows.len().saturating_sub(usize::from(view.rest.height));
            let first = app.scroll().min(last);
            let shown: Vec<Line> = view
                .rows
                .iter()
                .enumerate()
                .skip(first)
                .take(usize::from(view.rest.height))
                .map(|(row, text)| {
                    let line = Line::from(text.as_str());
                    if row < view.result_in_rows {
                        line.fg(color).bold()
                    } else {
                        line
                    }
                })
                .collect();
            frame.render_widget(Paragraph::new(shown), view.rest);
        }
        None => {}
    }
}

/// How many rows what scrolls takes and how many show, in a screen of `area`.
/// The model's scroll keys stop by these.
pub fn report_rows(app: &App, report: &mut ReportCache, area: Rect) -> (usize, usize) {
    match app.outcome() {
        Some(Outcome::Report { lines, result, .. }) if app.screen() == Screen::Report => {
            let view = report.view(app, result, lines, screen_areas(app, area)[1]);
            (view.rows.len(), usize::from(view.rest.height))
        }
        _ => column(app, screen_areas(app, area)[1]).map_or((0, 0), |column| {
            (column.rows.len(), usize::from(column.area.height))
        }),
    }
}

/// The report as last wrapped, kept from draw to draw: the loop draws every
/// 50 ms and takes one key per draw, so wrapping it each time makes a large
/// report slow to scroll. Kept here, not in the model: rows belong to a terminal.
#[derive(Default)]
pub struct ReportCache {
    kept: Option<KeptReport>,
}

struct KeptReport {
    outcome_id: u64,
    area: Rect,
    view: ReportView,
}

impl ReportCache {
    /// `report_view`, made again only for another report or another `area`.
    fn view(&mut self, app: &App, result: &str, lines: &[String], area: Rect) -> &ReportView {
        let outcome_id = app.outcome_id();
        self.kept
            .take_if(|kept| kept.outcome_id != outcome_id || kept.area != area);
        &self
            .kept
            .get_or_insert_with(|| KeptReport {
                outcome_id,
                area,
                view: report_view(result, lines, area),
            })
            .view
    }
}

/// Where the report's parts go. The rows are drawn one by one, past any `u16`
/// of a `Paragraph`'s scroll, and counted here as they are drawn.
struct ReportView {
    /// The result's box; no height when the result is in `rows`.
    top: Rect,
    /// What the result's box holds; nothing when the result is in `rows`.
    boxed: Vec<String>,
    rest: Rect,
    rows: Vec<String>,
    /// How many of `rows` are the result, when it has no box.
    result_in_rows: usize,
}

fn report_view(result: &str, lines: &[String], area: Rect) -> ReportView {
    let boxed = wrapped(result, area.width.saturating_sub(2));
    let result_rows = boxed.len();
    let body = lines.iter().flat_map(|l| wrapped(l, area.width));
    if result_rows + 2 + MIN_REPORT_ROWS <= usize::from(area.height) {
        let [top, rest] = Layout::vertical([
            Constraint::Length(result_rows as u16 + 2),
            Constraint::Min(0),
        ])
        .areas(area);
        return ReportView {
            top,
            boxed,
            rest,
            rows: body.collect(),
            result_in_rows: 0,
        };
    }
    // Too tall for a box: nothing else would show, and its end would be lost.
    let mut rows = wrapped(result, area.width);
    let result_in_rows = rows.len();
    rows.push(String::new());
    rows.extend(body);
    ReportView {
        top: Rect::default(),
        boxed: Vec::new(),
        rest: area,
        rows,
        result_in_rows,
    }
}

/// `text` in rows of at most `width` columns, none of it lost: it breaks after
/// a space, or inside a word too long for a row. By graphemes and columns as
/// the drawing counts them, so a wide character never runs past the edge.
fn wrapped(text: &str, width: u16) -> Vec<String> {
    #[cfg(test)]
    WRAPS.with(|wraps| wraps.set(wraps.get() + 1));
    let width = usize::from(width).max(1);
    let mut rows = Vec::new();
    for line in text.split('\n') {
        if line.is_ascii() && line.len() <= width {
            rows.push(line.to_string());
            continue;
        }
        let span = Span::raw(line);
        let mut row = String::new();
        let mut used = 0;
        // Where a break would keep a word whole: bytes and columns before it.
        let mut after_space = None;
        for grapheme in span.styled_graphemes(Style::new()) {
            let columns = Span::raw(grapheme.symbol).width();
            if used > 0 && used + columns > width {
                match after_space.take() {
                    Some((bytes, columns_before)) => {
                        let rest = row.split_off(bytes);
                        rows.push(std::mem::replace(&mut row, rest));
                        used -= columns_before;
                    }
                    None => {
                        rows.push(std::mem::take(&mut row));
                        used = 0;
                    }
                }
            }
            row.push_str(grapheme.symbol);
            used += columns;
            // Not in the indent: that would leave a row of spaces alone.
            if grapheme.symbol == " " && !row.trim_start().is_empty() {
                after_space = Some((row.len(), used));
            }
        }
        rows.push(row);
    }
    rows
}

#[cfg(test)]
thread_local! {
    /// How many times this thread called `wrapped`: a count, as a time bound
    /// could flake under load.
    static WRAPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub fn wraps_so_far() -> usize {
    WRAPS.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use photosync_core::{
        Counts, Event, EventKind, FailReason, FileOutcome, FileReport, MediaType, Phase, ScanError,
        SourceFile, SourceProgress, SourceReport, Volumes,
    };
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::app::{Key, Msg};

    /// The screen as text, one string per row.
    pub fn draw(app: &App, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut report = ReportCache::default();
        terminal.draw(|f| render(app, &mut report, f)).unwrap();
        rows(terminal.backend())
    }

    pub fn rows(backend: &TestBackend) -> Vec<String> {
        let buffer = backend.buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    fn has(screen: &[String], part: &str) -> bool {
        screen.iter().any(|row| row.contains(part))
    }

    fn app() -> App {
        let mut app = App::new();
        app.update(Msg::Volumes(Volumes {
            sources: vec![
                PathBuf::from("/Volumes/X-T5"),
                PathBuf::from("/Volumes/CARD2"),
            ],
            destinations: vec![PathBuf::from("/Volumes/SSD")],
            failures: Vec::new(),
        }));
        app
    }

    fn press(app: &mut App, keys: &[Key]) {
        for k in keys {
            app.update(Msg::Key(*k));
        }
    }

    #[test]
    fn sources_screen_lists_cards_with_marks_and_keys() {
        let mut app = app();
        press(&mut app, &[Key::Down, Key::Char(' ')]);
        let screen = draw(&app, 80, 12);
        assert!(
            has(&screen, "1/5  Pick the cards to copy from"),
            "{screen:#?}"
        );
        assert!(has(&screen, "[ ] /Volumes/X-T5"), "{screen:#?}");
        assert!(has(&screen, "> [x] /Volumes/CARD2"), "{screen:#?}");
        assert!(has(&screen, "Space pick"), "{screen:#?}");
    }

    #[test]
    fn empty_lists_say_what_to_plug_in() {
        let mut app = App::new();
        app.update(Msg::Volumes(Volumes::default()));
        assert!(has(&draw(&app, 120, 10), "No camera card found."));
    }

    #[test]
    fn library_field_shows_the_default_and_the_rule() {
        let mut app = app();
        press(&mut app, &[Key::Enter, Key::Enter]);
        let screen = draw(&app, 100, 12);
        assert!(has(&screen, "/Volumes/SSD/Photos"), "{screen:#?}");
        assert!(has(&screen, "It must exist"), "{screen:#?}");
    }

    #[test]
    fn confirm_screen_names_sources_library_and_folder() {
        let mut app = app();
        press(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        for c in "Tirana".chars() {
            press(&mut app, &[Key::Char(c)]);
        }
        press(&mut app, &[Key::Enter]);
        let screen = draw(&app, 100, 16);
        for part in [
            "/Volumes/X-T5",
            "/Volumes/SSD/Photos",
            "<date> - Tirana",
            "Enter start",
        ] {
            assert!(has(&screen, part), "{part}: {screen:#?}");
        }
    }

    #[test]
    fn running_screen_shows_progress_eta_per_source_and_cancelling() {
        let mut app = app();
        press(
            &mut app,
            &[
                Key::Char(' '),
                Key::Down,
                Key::Char(' '),
                Key::Enter,
                Key::Enter,
            ],
        );
        press(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        assert_eq!(app.screen(), Screen::Running);
        let c = |files, bytes| Counts { files, bytes };
        app.update(Msg::Progress(Event {
            kind: EventKind::Bytes,
            progress: Progress {
                phase: Phase::Copying,
                done: c(1, 25_000_000),
                total: c(4, 100_000_000),
                sources: vec![
                    SourceProgress {
                        root: PathBuf::from("/Volumes/X-T5"),
                        done: c(1, 25_000_000),
                        total: c(3, 90_000_000),
                    },
                    SourceProgress {
                        root: PathBuf::from("/Volumes/CARD2"),
                        done: c(0, 0),
                        total: c(1, 10_000_000),
                    },
                ],
                elapsed: Duration::from_secs(10),
                eta: Some(Duration::from_secs(30)),
            },
        }));
        press(&mut app, &[Key::Char('c')]);
        let screen = draw(&app, 100, 16);
        for part in [
            "copying 1/4 files, 25.0 MB of 100.0 MB (25%), ETA 30s",
            "X-T5  1/3 files, 25.0 MB of 90.0 MB",
            "CARD2  0/1 files",
            "Cancelling",
            "c cancel",
        ] {
            assert!(has(&screen, part), "{part}: {screen:#?}");
        }
    }

    /// The facts the CLI report gives, incl. "in place, flush not
    /// confirmed" and "Do not format the card", show on the report screen.
    #[test]
    fn report_screen_shows_not_complete_and_flush_not_confirmed() {
        let file = SourceFile {
            path: PathBuf::from("/c/DCIM/D.RAF"),
            media_type: Some(MediaType::Raw),
            size: 10,
            modified: std::time::UNIX_EPOCH,
        };
        let source = SourceReport {
            root: PathBuf::from("/c"),
            files: vec![FileReport {
                file,
                outcome: FileOutcome::Failed(FailReason::NotFlushed {
                    to: PathBuf::from("/l/D.RAF"),
                    error: std::io::Error::from_raw_os_error(libc::EIO),
                }),
            }],
            unreadable: Vec::new(),
        };
        let mut lines = text::source_section(&source);
        let result = text::result_line(&source.totals(), std::slice::from_ref(&source));
        lines.push(result.clone());
        let app = App::with_outcome_for_test(Outcome::Report {
            lines,
            result,
            complete: false,
        });
        let screen = draw(&app, 200, 20);
        for part in [
            "Result: NOT COMPLETE: 1 failed, 0 cancelled, 0 unreadable.",
            "1 of the failed in place, flush not confirmed.",
            "Do not format the card.",
            "  in place, flush not confirmed  /c/DCIM/D.RAF -> /l/D.RAF: ",
            "  of the failed, 1 in place, flush not confirmed",
            "q quit",
        ] {
            assert!(has(&screen, part), "{part}: {screen:#?}");
        }
    }

    /// F08: the name stays for the session, so the way out is to start again,
    /// and the result's box must show it whole on a plain terminal.
    #[test]
    fn report_screen_shows_the_way_out_of_a_name_collision_at_80_columns() {
        let source = SourceReport {
            root: PathBuf::from("/c"),
            files: vec![FileReport {
                file: SourceFile {
                    path: PathBuf::from("/c/DCIM/D.RAF"),
                    media_type: Some(MediaType::Raw),
                    size: 10,
                    modified: std::time::UNIX_EPOCH,
                },
                outcome: FileOutcome::Failed(FailReason::NameCollision {
                    existing: PathBuf::from("/l/D.RAF"),
                }),
            }],
            unreadable: Vec::new(),
        };
        let lines = text::source_section(&source);
        let result = text::result_line(&source.totals(), std::slice::from_ref(&source));
        let app = App::with_outcome_for_test(Outcome::Report {
            lines,
            result,
            complete: false,
        });
        let screen = draw(&app, 80, 24);
        let top = screen[..5].join(" ");
        let words: Vec<&str> = top
            .split(|c: char| c.is_whitespace() || c == '│')
            .filter(|w| !w.is_empty())
            .collect();
        let words = words.join(" ");
        for part in [
            "start photosync again to sync under another name",
            "files already on the drive are skipped.",
        ] {
            assert!(words.contains(part), "{part}: {screen:#?}");
        }
    }

    #[test]
    fn report_screen_shows_duplicates_and_the_card_at_80_columns() {
        let file = |path: &str| SourceFile {
            path: PathBuf::from(path),
            media_type: Some(MediaType::Raw),
            size: 10,
            modified: std::time::UNIX_EPOCH,
        };
        let source = SourceReport {
            root: PathBuf::from("/Volumes/NO NAME"),
            files: vec![
                FileReport {
                    file: file("/Volumes/NO NAME/DCIM/100_FUJI/DSCF0001.RAF"),
                    outcome: FileOutcome::AlreadySynced {
                        library_file: PathBuf::from("/l/DSCF0001.RAF"),
                    },
                },
                FileReport {
                    file: file("/Volumes/NO NAME/DCIM/100_FUJI/DSCF0002.RAF"),
                    outcome: FileOutcome::DuplicateInSession {
                        of: PathBuf::from("/Volumes/UNTITLED/DCIM/100_FUJI/DSCF0002.RAF"),
                    },
                },
            ],
            unreadable: Vec::new(),
        };
        let lines = text::source_section(&source);
        let result = text::result_line(&source.totals(), std::slice::from_ref(&source));
        let app = App::with_outcome_for_test(Outcome::Report {
            lines,
            result,
            complete: true,
        });
        let screen = draw(&app, 80, 24);
        for part in [
            "  of the skipped, 1 already on drive, 1 duplicate in session",
            "  duplicate in session  100_FUJI/DSCF0002.RAF, on drive from /Volumes/UNTITLED",
        ] {
            assert!(has(&screen, part), "{part}: {screen:#?}");
        }
    }

    /// The screen with its wrapping undone: no space and no box, so a line
    /// reads the same wherever it broke.
    fn unwrapped(screen: &[String]) -> String {
        screen
            .concat()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '│')
            .collect()
    }

    fn no_spaces(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    fn report_app(lines: Vec<String>, result: &str) -> App {
        App::with_outcome_for_test(Outcome::Report {
            lines,
            result: result.to_string(),
            complete: false,
        })
    }

    /// S048: `wrapped` measures with a `Span`, a drawn row with a `Line`. Mixed
    /// graphemes (wide, combining, a flag, a joined emoji) must fit both ways.
    #[test]
    fn wrapped_rows_of_mixed_graphemes_fit_as_a_drawn_line_measures_them() {
        let text = "Tiranë e\u{301} 写真カード 🇦🇱 flag 👩‍💻 joined /Volumes/DCIM/写真写真写真写真";
        for width in 2..40 {
            let rows = wrapped(text, width);
            for row in &rows {
                assert!(Line::from(row.as_str()).width() <= usize::from(width), "{row:?} at {width}");
            }
            assert_eq!(rows.concat(), text, "at {width}");
        }
    }

    /// S037: the reason comes last on the line, so it was what a cut took.
    #[test]
    fn a_long_failed_line_shows_its_reason_at_80_columns() {
        let path = "/run/media/max/NO NAME/DCIM/100_FUJI/DSCF0001.RAF";
        let error = std::io::Error::from_raw_os_error(libc::EIO);
        let reason = no_spaces(&error.to_string());
        let source = SourceReport {
            root: PathBuf::from("/run/media/max/NO NAME"),
            files: vec![FileReport {
                file: SourceFile {
                    path: PathBuf::from(path),
                    media_type: Some(MediaType::Raw),
                    size: 10,
                    modified: std::time::UNIX_EPOCH,
                },
                outcome: FileOutcome::Failed(FailReason::Write {
                    path: PathBuf::from(
                        "/run/media/max/My Passport/Photos/2026-10-28 - Tirana/RAW/DSCF0001.RAF",
                    ),
                    error,
                }),
            }],
            unreadable: Vec::new(),
        };
        let lines = text::source_section(&source);
        let long = lines.iter().find(|l| l.contains(path)).unwrap();
        assert!(long.chars().count() > 80, "not a long line: {long}");
        let screen = draw(&report_app(lines.clone(), "Result: NOT COMPLETE."), 80, 24);
        let shown = unwrapped(&screen);
        assert!(shown.contains(&no_spaces(long)), "{screen:#?}");
        assert!(shown.contains(&reason), "{screen:#?}");
    }

    /// S037: the card is what says which card to look at.
    #[test]
    fn a_duplicate_line_shows_a_long_linux_card_whole_at_80_columns() {
        let file = |path: &str| SourceFile {
            path: PathBuf::from(path),
            media_type: Some(MediaType::Raw),
            size: 10,
            modified: std::time::UNIX_EPOCH,
        };
        let source = SourceReport {
            root: PathBuf::from("/run/media/max/UNTITLED"),
            files: vec![FileReport {
                file: file("/run/media/max/UNTITLED/DCIM/100_FUJI/DSCF0002.RAF"),
                outcome: FileOutcome::DuplicateInSession {
                    of: PathBuf::from("/run/media/max/NO NAME/DCIM/100_FUJI/DSCF0002.RAF"),
                },
            }],
            unreadable: Vec::new(),
        };
        let lines = text::source_section(&source);
        let long = lines
            .iter()
            .find(|l| l.contains("on drive from /run/media/max/NO NAME"))
            .unwrap();
        assert!(long.chars().count() > 80, "not a long line: {long}");
        let screen = draw(&report_app(lines.clone(), "Result: all fine."), 80, 24);
        assert!(unwrapped(&screen).contains(&no_spaces(long)), "{screen:#?}");
    }

    /// A wide character takes two columns: counting chars would run past
    /// the edge and lose the end all the same.
    #[test]
    fn a_line_of_wide_characters_is_shown_whole() {
        let line = format!("  failed  {}.RAF: disk full", "/写真カード".repeat(12));
        let screen = draw(&report_app(vec![line.clone()], "Result."), 40, 24);
        assert!(unwrapped(&screen).contains(&no_spaces(&line)), "{screen:#?}");
    }

    /// S037, S033's note: a session's result can name many syncs, and it says
    /// whether the cards are safe to format. The report below it still shows.
    #[test]
    fn a_long_result_grows_its_box_and_is_shown_whole() {
        let result = (1..=20)
            .map(|n| format!("Sync {n} on /run/media/max/CARD{n} was not complete."))
            .collect::<Vec<_>>()
            .join(" ");
        let app = report_app(vec!["first line of the report".into()], &result);
        let screen = draw(&app, 80, 40);
        assert!(unwrapped(&screen).contains(&no_spaces(&result)), "{screen:#?}");
        assert!(has(&screen, "first line of the report"), "{screen:#?}");
    }

    #[test]
    fn wrapping_keeps_words_whole_and_loses_nothing() {
        let rows = |text, width| wrapped(text, width);
        assert_eq!(rows("short", 80), ["short"]);
        assert_eq!(rows("", 80), [""], "a blank line stays a row");
        assert_eq!(rows("aaa bbb ccc", 7), ["aaa ", "bbb ccc"]);
        assert_eq!(rows("aaaaaaaaaa", 4), ["aaaa", "aaaa", "aa"]);
        assert_eq!(rows("aa bbbbbbbbbb", 4), ["aa ", "bbbb", "bbbb", "bb"]);
        assert_eq!(rows("    aaaaaaaa", 6), ["    aa", "aaaaaa"], "no row of spaces");
        assert_eq!(rows("写真カード", 4), ["写真", "カー", "ド"]);
        assert_eq!(rows("写真", 1), ["写", "真"], "wider than a row: alone on one");
        assert_eq!(rows("one\ntwo", 80), ["one", "two"]);
    }

    /// A report on a tiny terminal must not panic the view either.
    #[test]
    fn tiny_terminal_draws_a_report_without_panic() {
        let app = report_app(vec!["a line".repeat(30), String::new()], &"word ".repeat(40));
        for (width, height) in [(10, 4), (2, 2), (1, 1), (3, 12), (80, 3)] {
            draw(&app, width, height);
        }
    }

    #[test]
    fn refused_screen_shows_the_sentence() {
        let app = App::with_outcome_for_test(Outcome::Refused {
            sentence: "The library /x is on /c, which is a camera card.".into(),
        });
        let screen = draw(&app, 100, 10);
        assert!(has(
            &screen,
            "The library /x is on /c, which is a camera card."
        ));
        assert!(has(&screen, "Enter back"));
    }

    /// S042: a bad name is refused before the session, so there is no next
    /// card to take: the footer does not offer it, and `n` does nothing.
    #[test]
    fn a_bad_name_is_refused_without_offering_the_next_card() {
        let mut app = app();
        press(&mut app, &[Key::Enter, Key::Enter, Key::Enter]);
        press(&mut app, &[Key::Char('a'), Key::Char('/'), Key::Char('b')]);
        press(&mut app, &[Key::Enter, Key::Enter]);
        assert_eq!(app.screen(), Screen::Refused);
        assert!(!app.in_session());

        let screen = draw(&app, 100, 10);
        assert!(!has(&screen, "n next card"), "{screen:#?}");
        assert!(has(&screen, "Enter back"), "{screen:#?}");
        assert!(has(&screen, "q quit"), "{screen:#?}");

        press(&mut app, &[Key::Char('n')]);
        assert_eq!(app.screen(), Screen::Refused);
        press(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Confirm);
    }

    /// S047: a warning names a path and an error, so its end is what a cut took.
    #[test]
    fn a_long_note_shows_whole_on_the_card_and_drive_lists_at_80_columns() {
        let mut app = App::new();
        app.update(Msg::Volumes(Volumes {
            sources: vec![PathBuf::from("/Volumes/X-T5")],
            destinations: vec![PathBuf::from("/Volumes/SSD")],
            failures: vec![ScanError {
                path: PathBuf::from("/run/media/max/My Passport With A Long Name/DCIM/100_FUJI"),
                source: std::io::Error::from_raw_os_error(libc::EIO),
            }],
        }));
        let note = app.notes()[0].clone();
        assert!(note.chars().count() > 80, "not a long note: {note}");
        let note = note.as_str();
        let screen = draw(&app, 80, 24);
        assert!(unwrapped(&screen).contains(&no_spaces(note)), "{screen:#?}");
        press(&mut app, &[Key::Enter]);
        assert_eq!(app.screen(), Screen::Drive);
        let screen = draw(&app, 80, 24);
        assert!(unwrapped(&screen).contains(&no_spaces(note)), "{screen:#?}");
    }

    /// S047: the marks and the cursor take columns too, and two cards can
    /// differ only at the end of the path.
    #[test]
    fn a_long_path_in_a_list_shows_whole_at_80_columns() {
        let long = format!("/run/media/max/{}", "Very Long Card Name ".repeat(5));
        let mut app = App::new();
        app.update(Msg::Volumes(Volumes {
            sources: vec![PathBuf::from(&long), PathBuf::from("/Volumes/CARD2")],
            destinations: vec![PathBuf::from(&long), PathBuf::from("/Volumes/SSD")],
            failures: Vec::new(),
        }));
        for screen_now in [Screen::Sources, Screen::Drive] {
            assert_eq!(app.screen(), screen_now);
            let screen = draw(&app, 80, 24);
            assert!(unwrapped(&screen).contains(&no_spaces(&long)), "{screen:#?}");
            assert!(has(&screen, "/Volumes/"), "{screen:#?}");
            press(&mut app, &[Key::Enter]);
        }
    }

    fn running_app() -> App {
        let mut app = app();
        press(
            &mut app,
            &[
                Key::Char(' '),
                Key::Down,
                Key::Char(' '),
                Key::Enter,
                Key::Enter,
                Key::Enter,
                Key::Enter,
                Key::Enter,
            ],
        );
        assert_eq!(app.screen(), Screen::Running);
        app
    }

    /// S047: the end of a scan line says how much could not be read.
    #[test]
    fn a_long_scan_line_shows_whole_on_the_copying_screen_at_80_columns() {
        let line = "scanned /run/media/max/My Long Card Name: 1234 photos (45.6 GB), 12 ignored, 3 unreadable, device 16777234";
        let mut app = running_app();
        app.update(Msg::Scanned(line.into()));
        let screen = draw(&app, 80, 24);
        assert!(unwrapped(&screen).contains(&no_spaces(line)), "{screen:#?}");
    }

    /// S047: with several cards the progress line names each, and past the
    /// edge a card's count, or the ETA of a line per card, was cut.
    #[test]
    fn the_progress_lines_of_long_card_names_show_whole_at_80_columns() {
        let c = |files, bytes| Counts { files, bytes };
        let root = |name: &str| PathBuf::from(format!("/run/media/max/{name}"));
        let long = "A Card With A Name Long Enough To Pass The Edge Of The Terminal For Sure";
        let progress = Progress {
            phase: Phase::Copying,
            done: c(1, 25_000_000),
            total: c(4, 100_000_000),
            sources: vec![
                SourceProgress {
                    root: root(long),
                    done: c(1, 25_000_000),
                    total: c(3, 90_000_000),
                },
                SourceProgress {
                    root: root("CARD2"),
                    done: c(0, 0),
                    total: c(1, 10_000_000),
                },
            ],
            elapsed: Duration::from_secs(10),
            eta: Some(Duration::from_secs(30)),
        };
        let line = text::progress_line(&progress);
        let each = per_source(&progress)[0].clone();
        assert!(line.chars().count() > 80, "not a long line: {line}");
        assert!(each.chars().count() > 80, "not a long line: {each}");
        let mut app = running_app();
        app.update(Msg::Progress(Event {
            kind: EventKind::Bytes,
            progress,
        }));
        let screen = draw(&app, 80, 24);
        let shown = unwrapped(&screen);
        assert!(shown.contains(&no_spaces(&line)), "{screen:#?}");
        assert!(shown.contains(&no_spaces(&each)), "{screen:#?}");
    }

    /// S047: the hint is the one line that says why a key did nothing.
    #[test]
    fn a_long_hint_shows_whole_at_80_columns() {
        let mut app = App::new();
        app.update(Msg::Volumes(Volumes::default()));
        press(&mut app, &[Key::Enter]);
        let hint = app.hint().unwrap().to_string();
        assert!(hint.chars().count() > 80, "not a long hint: {hint}");
        let screen = draw(&app, 80, 24);
        let shown = unwrapped(&screen[1..]);
        assert!(shown.contains(&no_spaces(&hint)), "{screen:#?}");
    }

    /// S047: on a narrow terminal the keys are how to get to the rest.
    #[test]
    fn the_keys_show_whole_on_a_narrow_terminal() {
        let app = App::with_outcome_for_test(Outcome::Refused {
            sentence: "No.".into(),
        });
        let screen = draw(&app, 20, 12);
        let shown = unwrapped(&screen);
        assert!(shown.contains(&no_spaces(&keys_help(&app))), "{screen:#?}");
    }

    /// A narrow terminal must not panic the view.
    #[test]
    fn tiny_terminal_draws_without_panic() {
        let mut app = app();
        for keys in [
            &[][..],
            &[Key::Enter],
            &[Key::Enter],
            &[Key::Enter],
            &[Key::Enter],
        ] {
            press(&mut app, keys);
            draw(&app, 10, 4);
        }
    }
}
