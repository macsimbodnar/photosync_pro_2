//! Draws an `App` with ratatui. Reads the model only: every rule is in
//! `app`, every sentence about the sync in `text`.

use std::path::Path;

use photosync_core::Progress;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Gauge, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, Outcome, Screen};
use crate::text;

pub fn render(app: &App, frame: &mut Frame) {
    let [title, body, hint, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(
        Line::from(vec![
            Span::from(" phoosync pro 2 ").bold().reversed(),
            Span::from(format!("  {}", heading(app))),
        ]),
        title,
    );
    match app.screen() {
        Screen::Sources => sources(app, frame, body),
        Screen::Drive => drives(app, frame, body),
        Screen::Library => field(
            frame,
            body,
            "Library folder",
            app.library(),
            &[
                "The folder on the drive that holds your photos. It must exist:".into(),
                "phoosync pro 2 never creates it. Photos already anywhere in it are skipped.".into(),
            ],
        ),
        Screen::Name => field(
            frame,
            body,
            "Name (optional)",
            app.name(),
            &[
                format!(
                    "New photos go to {}/{}/RAW, JPEG, ...",
                    app.library().trim(),
                    app.folder_preview()
                ),
                "A folder with this name from an earlier sync is reused. Empty: no name.".into(),
            ],
        ),
        Screen::Confirm => confirm(app, frame, body),
        Screen::Running => running(app, frame, body),
        Screen::Refused | Screen::Report => outcome(app, frame, body),
    }
    if let Some(h) = app.hint() {
        frame.render_widget(Line::from(h.to_string()).yellow(), hint);
    }
    frame.render_widget(Line::from(keys_help(app)).dim(), keys);
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
pub fn keys_help(app: &App) -> &'static str {
    match app.screen() {
        Screen::Sources => " ↑↓ move   Space pick   Enter next   r look again   q quit",
        Screen::Drive => " ↑↓ move   Enter pick   Esc back   r look again   q quit",
        Screen::Library | Screen::Name => " type to edit   Enter next   Esc back   Ctrl-C quit",
        Screen::Confirm => " Enter start copying   Esc back   q quit",
        Screen::Running => " c cancel   Ctrl-C twice quits at once",
        // Before the session starts there is no session for a next card to join.
        Screen::Refused if app.in_session() => " Enter back, to fix it   n next card   q quit",
        Screen::Refused => " Enter back, to fix it   q quit",
        Screen::Report => " ↑↓ PgUp PgDn scroll   n next card   q quit",
    }
}

fn list_screen(app: &App, frame: &mut Frame, area: Rect, title: &str, rows: Vec<ListItem>) {
    let [list_area, notes_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(app.notes().len() as u16),
    ])
    .areas(area);
    let block = Block::bordered().title(title.to_string());
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(app.empty_list_note())
                .wrap(Wrap { trim: false })
                .block(block),
            list_area,
        );
    } else {
        let list = List::new(rows)
            .block(block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol("> ");
        let mut state = ListState::default().with_selected(Some(app.cursor()));
        frame.render_stateful_widget(list, list_area, &mut state);
    }
    let notes: Vec<Line> = app
        .notes()
        .iter()
        .map(|n| Line::from(n.as_str()).yellow())
        .collect();
    frame.render_widget(Paragraph::new(notes), notes_area);
}

fn sources(app: &App, frame: &mut Frame, area: Rect) {
    let rows = app
        .sources()
        .iter()
        .map(|s| {
            let mark = if app.is_picked(s) { "[x]" } else { "[ ]" };
            ListItem::new(format!("{mark} {}", s.display()))
        })
        .collect();
    list_screen(app, frame, area, "Camera cards", rows);
}

fn drives(app: &App, frame: &mut Frame, area: Rect) {
    let rows = app
        .drives()
        .iter()
        .map(|d| {
            let mark = if app.drive() == Some(d.as_path()) {
                "(*)"
            } else {
                "( )"
            };
            ListItem::new(format!("{mark} {}", d.display()))
        })
        .collect();
    list_screen(app, frame, area, "Drives", rows);
}

fn field(frame: &mut Frame, area: Rect, title: &str, value: &str, help: &[String]) {
    let [input, rest] = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(value.to_string()).block(Block::bordered().title(title.to_string())),
        input,
    );
    let width = value.chars().count() as u16;
    frame.set_cursor_position(Position::new(
        (input.x + 1 + width).min(input.right().saturating_sub(2)),
        input.y + 1,
    ));
    let help: Vec<Line> = help.iter().map(|h| Line::from(h.as_str())).collect();
    frame.render_widget(Paragraph::new(help).wrap(Wrap { trim: false }), rest);
}

fn confirm(app: &App, frame: &mut Frame, area: Rect) {
    let mut lines = vec![Line::from("Copy from:".bold())];
    lines.extend(
        app.picked()
            .iter()
            .map(|s| Line::from(format!("  {}", s.display()))),
    );
    lines.push(Line::from(""));
    lines.push(Line::from("Into the library:".bold()));
    lines.push(Line::from(format!("  {}", app.library().trim())));
    lines.push(Line::from(""));
    lines.push(Line::from("New photos go to the folder:".bold()));
    let fixed = if app.session_folder().is_some() {
        "(this session's folder)"
    } else {
        "(the date of the earliest new photo)"
    };
    lines.push(Line::from(format!("  {}   {fixed}", app.folder_preview())));
    if app.in_session() {
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Sync {} of this session: the library and name stay as they are. Quit (q) to change them.",
            app.syncs() + 1
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "The cards are only read. Photos already in the library are skipped.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::bordered()),
        area,
    );
}

fn running(app: &App, frame: &mut Frame, area: Rect) {
    let Some(run) = app.running() else {
        return;
    };
    let sources = run.progress.as_ref().map_or(0, |p| p.sources.len()) as u16;
    let [scans, gauge, line, each, state] = Layout::vertical([
        Constraint::Length(run.scanned.len() as u16),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(if sources > 1 { sources } else { 0 }),
        Constraint::Min(0),
    ])
    .areas(area);
    let scanned: Vec<Line> = run.scanned.iter().map(|s| Line::from(s.as_str())).collect();
    frame.render_widget(Paragraph::new(scanned), scans);
    match &run.progress {
        Some(p) => {
            frame.render_widget(
                Gauge::default()
                    .block(Block::bordered())
                    .gauge_style(Style::new().fg(Color::Green))
                    .ratio(ratio(p.done.bytes, p.total.bytes)),
                gauge,
            );
            frame.render_widget(Line::from(text::progress_line(p)), line);
            frame.render_widget(Paragraph::new(per_source(p)), each);
        }
        None => frame.render_widget(Line::from("scanning the cards..."), line),
    }
    if run.cancelling {
        frame.render_widget(
            Paragraph::new(
                "Cancelling: the files being copied are finished or dropped, then the report shows.",
            )
            .wrap(Wrap { trim: false })
            .yellow(),
            state,
        );
    }
}

fn per_source(p: &Progress) -> Vec<Line<'static>> {
    if p.sources.len() < 2 {
        return Vec::new();
    }
    p.sources
        .iter()
        .map(|s| {
            Line::from(format!(
                "  {}  {}/{} files, {} of {}",
                name_of(&s.root),
                s.done.files,
                s.total.files,
                text::size(s.done.bytes),
                text::size(s.total.bytes)
            ))
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

fn outcome(app: &App, frame: &mut Frame, area: Rect) {
    match app.outcome() {
        Some(Outcome::Refused { sentence, .. }) => frame.render_widget(
            Paragraph::new(sentence.as_str())
                .wrap(Wrap { trim: false })
                .red()
                .block(Block::bordered()),
            area,
        ),
        Some(Outcome::Report {
            lines,
            result,
            complete,
        }) => {
            let [top, rest] =
                Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(area);
            let color = if *complete { Color::Green } else { Color::Red };
            frame.render_widget(
                Paragraph::new(result.as_str())
                    .wrap(Wrap { trim: false })
                    .fg(color)
                    .bold()
                    .block(Block::bordered()),
                top,
            );
            let body: Vec<Line> = lines.iter().map(|l| Line::from(l.as_str())).collect();
            frame.render_widget(
                Paragraph::new(body).scroll((u16::try_from(app.scroll()).unwrap_or(u16::MAX), 0)),
                rest,
            );
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use photosync_core::{
        Counts, Event, EventKind, FailReason, FileOutcome, FileReport, MediaType, Phase,
        SourceFile, SourceProgress, SourceReport, Volumes,
    };
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::app::{Key, Msg};

    /// The screen as text, one string per row.
    pub fn draw(app: &App, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(app, f)).unwrap();
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

    /// The report body neither wraps nor scrolls sideways: at the width of
    /// a plain terminal, the two kinds of skipped and the card must fit.
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
