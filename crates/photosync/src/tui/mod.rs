//! The terminal front-end: `photosync` with no command. A thin shell over
//! `app`: it turns crossterm keys into `app::Key`, runs the model's effects
//! (volume discovery, the worker thread), and draws with `view`. A GUI
//! replaces this module and `view`, and keeps `app`, `worker` and `flow`.

mod view;

use std::io;
use std::panic;
use std::path::PathBuf;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

use photosync_core::{Volumes, discover_volumes};
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Effect, Key, Msg};
use crate::worker;

/// How long to wait for a key before drawing again: the progress needs
/// redrawing while the worker runs.
const FRAME: Duration = Duration::from_millis(50);

/// Runs the TUI until the user quits, and returns the model, which says
/// what to print and the exit code (`App::exit`). The terminal is restored
/// on every way out: return, error, or a panic of this thread (ratatui's
/// panic hook, and `Restore` while unwinding). A worker's panic leaves it
/// as it is: the loop shows it as the sync's end.
pub fn run(search_roots: Vec<PathBuf>) -> Result<App, Failed> {
    // Before init: a failed init may have turned on raw mode already.
    let _restore = Restore(ratatui::restore);
    let mut terminal = ratatui::try_init().map_err(Failed::Start)?;
    restore_on_panic_of_this_thread_only();
    run_loop(&mut terminal, read_key, || discover_volumes(&search_roots)).map_err(Failed::Running)
}

/// Why the TUI ended without the user quitting.
#[derive(Debug)]
pub enum Failed {
    /// The terminal could not be set up: nothing ran.
    Start(io::Error),
    /// Drawing or reading a key failed: a copy may have been running.
    Running(io::Error),
}

/// Calls its function when dropped, unwinding included.
struct Restore<F: FnMut()>(F);

impl<F: FnMut()> Drop for Restore<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// ratatui's panic hook restores the terminal on a panic in any thread.
/// A worker's panic must not: the loop goes on drawing, and would draw
/// over the shell. So the hook in place acts for this thread only. Another
/// thread's panic is not printed either, as stderr is the screen: the
/// worker catches it and sends its text as the sync's end (`worker`).
fn restore_on_panic_of_this_thread_only() {
    let tui = thread::current().id();
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if thread::current().id() == tui {
            hook(info);
        }
    }));
}

/// The loop, with the keys and discovery injected, so tests run it on a
/// `TestBackend` with scripted keys. `next_key` sees the model, so a
/// script can wait for a screen; it may block up to a frame.
pub fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut next_key: impl FnMut(&App) -> io::Result<Option<Key>>,
    discover: impl Fn() -> Volumes,
) -> io::Result<App> {
    let mut app = App::checking_library();
    let mut effects = vec![Effect::Discover];
    let mut from_worker: Option<mpsc::Receiver<Msg>> = None;
    loop {
        while let Some(effect) = effects.pop() {
            match effect {
                Effect::Discover => effects.extend(app.update(Msg::Volumes(discover()))),
                Effect::Start(job) => {
                    let (tx, rx) = mpsc::channel();
                    // A send fails only once the receiver is gone, i.e. the
                    // user quit: the process is exiting, INV-7 keeps it safe.
                    worker::spawn(job, move |msg| {
                        let _ = tx.send(msg);
                    });
                    from_worker = Some(rx);
                }
                Effect::Quit => return Ok(app),
            }
        }
        if let Some(rx) = &from_worker {
            loop {
                match rx.try_recv() {
                    Ok(msg) => effects.extend(app.update(msg)),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        from_worker = None;
                        effects.extend(app.update(Msg::WorkerGone));
                        break;
                    }
                }
            }
        }
        draw_frame(terminal, &mut app)?;
        if let Some(key) = next_key(&app)? {
            effects.extend(app.update(Msg::Key(key)));
        }
    }
}

/// Draws, then tells the model how the report wrapped: only a drawing knows
/// the size of the screen, and the scroll keys stop by it.
fn draw_frame<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut report = (0, 0);
    terminal
        .draw(|frame| {
            view::render(app, frame);
            report = view::report_rows(app, frame.area());
        })
        .map_err(|e| io::Error::other(e.to_string()))?;
    app.report_drawn(report.0, report.1);
    Ok(())
}

fn read_key(_: &App) -> io::Result<Option<Key>> {
    if !event::poll(FRAME)? {
        return Ok(None);
    }
    Ok(match event::read()? {
        event::Event::Key(k) if k.kind == KeyEventKind::Press => key(k),
        _ => None,
    })
}

fn key(k: KeyEvent) -> Option<Key> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Char('c') if ctrl => Key::CtrlC,
        // No other shortcut: a Ctrl or Alt letter is not text either.
        KeyCode::Char(_) if ctrl || k.modifiers.contains(KeyModifiers::ALT) => return None,
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
