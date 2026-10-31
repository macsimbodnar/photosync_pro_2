//! The one file that calls iced's `text`, `button` and `checkbox`. `Wrapping::Word`
//! cuts a word longer than a row with nothing wrong in its bounds, so words reach the
//! screen only through `words`, and `tests/gui_split.rs` holds that (DEC-066 (4), (7), DEC-072 (4)).

use iced::widget::text::Wrapping;
use iced::widget::{Text, button, checkbox, row, text};
use iced::{Alignment, Element, Length, Theme};

/// Words on the screen, never cut.
pub fn words<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).wrapping(Wrapping::WordOrGlyph)
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
        // Next with none ticked takes this row, so it must show (DEC-068 (2)).
        .style(if under_cursor {
            button::secondary
        } else {
            button::text
        })
        .into()
}

/// With no `on_toggle` a checkbox draws as disabled, and this one is not.
fn looks_enabled(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let status = match status {
        checkbox::Status::Disabled { is_checked } => checkbox::Status::Active { is_checked },
        other => other,
    };
    checkbox::primary(theme, status)
}
