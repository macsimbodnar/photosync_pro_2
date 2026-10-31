//! The one file that calls iced's `text`, `button` and `checkbox`. `Wrapping::Word`
//! cuts a word longer than a row with nothing wrong in its bounds, so words reach the
//! screen only through `words`, and `tests/gui_split.rs` holds that (DEC-066 (4), (7), DEC-072 (4)).

use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Text, button, checkbox, column, container, progress_bar, row, scrollable, text,
    text_input,
};
use iced::{Alignment, Element, Length, Padding, Theme, border};

/// Words on the screen, never cut.
pub fn words<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).wrapping(Wrapping::WordOrGlyph)
}

/// Why the last click did nothing, in the colour the TUI gives it.
pub fn hint<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    words(content).style(text::warning)
}

/// A refusal's sentence: the banner's red. `text::danger` on the window is 2.74 to 1, too
/// thin to read a whole sentence by (DEC-079 (5)).
pub fn red_box<'a, M: 'a>(content: impl text::IntoFragment<'a>) -> Element<'a, M> {
    container(words(content))
        .style(container::danger)
        .width(Length::Fill)
        .padding(8)
        .into()
}

/// A one-line field. What is typed stays on its line, as in the TUI (DEC-072 (4)).
pub fn field<'a, M: Clone + 'a>(
    id: &'static str,
    placeholder: &str,
    value: &str,
    on_input: fn(String) -> M,
    on_submit: M,
) -> Element<'a, M> {
    text_input(placeholder, value)
        .id(id)
        .on_input(on_input)
        .on_submit(on_submit)
        .into()
}

/// Words in a box of at most `most` high that scroll inside it, so they can never push
/// what is under them off the window, and nothing has to be measured (DEC-071 (3), (4)).
pub fn word_box<'a, M: 'a>(id: &'static str, content: String, most: f32) -> Element<'a, M> {
    let inside = scrollable(words(content).width(Length::Fill))
        .id(id)
        .spacing(8);
    container(inside)
        .width(Length::Fill)
        .max_height(most)
        .into()
}

/// Green only for a session that is complete (DEC-069 (2)).
pub fn result_style(complete: bool) -> fn(&Theme) -> container::Style {
    if complete {
        container::success
    } else {
        container::danger
    }
}

/// The result over the report: a `word_box` on its colour, as the banner's reason is on red.
pub fn result_box<'a, M: 'a>(
    id: &'static str,
    content: String,
    most: f32,
    complete: bool,
) -> Element<'a, M> {
    container(word_box(id, content, most))
        .style(result_style(complete))
        .width(Length::Fill)
        .padding(8)
        .into()
}

/// Words under the line that names them.
pub fn under<'a, M: 'a>(line: &'a str, items: Column<'a, M>) -> Element<'a, M> {
    column![words(line), items.padding(Padding::ZERO.left(16.0))]
        .spacing(2)
        .into()
}

/// The not-safe banner: its line, and under it its reason in a box (DEC-071 (2)).
pub fn red_banner<'a, M: 'a>(line: &'a str, reason: Option<Element<'a, M>>) -> Element<'a, M> {
    let mut inside = column![words(line)].spacing(4);
    if let Some(reason) = reason {
        inside = inside.push(reason);
    }
    container(inside)
        .style(container::danger)
        .width(Length::Fill)
        .padding(8)
        .into()
}

/// A bar, `share` of it full.
pub fn bar<'a, M: 'a>(share: f32) -> Element<'a, M> {
    progress_bar(0.0..=1.0, share).girth(10.0).into()
}

/// A button; with no message it is drawn held, and a click sends nothing.
pub fn action<'a, M: Clone + 'a>(label: &'a str, message: Option<M>) -> Element<'a, M> {
    button(words(label)).on_press_maybe(message).into()
}

/// A list row: a click anywhere on it sends `message`. The checkbox is bare, or its
/// label would draw past `words`, and has no `on_toggle`, so it captures nothing.
pub fn list_row<'a, M: Clone + 'a>(
    picked: bool,
    under_cursor: bool,
    label: String,
    message: M,
) -> Element<'a, M> {
    let mark = checkbox(picked).style(looks_enabled);
    let line = row![mark, words(label).width(Length::Fill)]
        .spacing(10)
        .align_y(Alignment::Center);
    button(line)
        .on_press(message)
        .width(Length::Fill)
        // Next with no card ticked takes this row, so it must show (DEC-068 (2)).
        .style(if under_cursor {
            row_under_cursor
        } else {
            button::text
        })
        .into()
}

/// A bar with light words, as the other rows have. The theme's secondary pair gives dark
/// thin words on mid grey, 4.87 to 1 (S062).
pub fn row_under_cursor(theme: &Theme, status: button::Status) -> button::Style {
    let background = theme.extended_palette().background;
    let pair = match status {
        button::Status::Hovered => background.strong,
        _ => background.strongest,
    };
    button::Style {
        background: Some(pair.color.into()),
        text_color: pair.text,
        border: border::rounded(2),
        ..button::Style::default()
    }
}

/// With no `on_toggle` a checkbox draws as disabled, and this one is not.
fn looks_enabled(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let status = match status {
        checkbox::Status::Disabled { is_checked } => checkbox::Status::Active { is_checked },
        other => other,
    };
    checkbox::primary(theme, status)
}
