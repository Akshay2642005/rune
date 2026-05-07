use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
};

pub(super) fn nav_item(label: &'static str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            label,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, Style::default().fg(Color::Gray))
    }
}

/// Returns a `Span` with cyan bold styling — used for key hints.
pub(super) fn key_span(label: &'static str) -> Span<'static> {
    Span::styled(
        label,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

/// Centers a fixed-size popup over `area`.
/// Change `width`/`height` at the call site to resize the popup.
pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    // Fill(1) on both sides splits leftover space equally → true centering.
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(vertical[1])[1]
}

/// Places a fixed-size rect in the bottom-right corner with a small margin.
/// Adjust `margin_right` / `margin_bottom` to move it away from the edges.
pub(super) fn bottom_right_rect(width: u16, height: u16, area: Rect) -> Rect {
    let margin_right: u16 = 2; // columns from the right edge
    let margin_bottom: u16 = 3; // rows from the bottom edge

    let x = area.x + area.width.saturating_sub(width + margin_right);
    let y = area.y + area.height.saturating_sub(height + margin_bottom);
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
