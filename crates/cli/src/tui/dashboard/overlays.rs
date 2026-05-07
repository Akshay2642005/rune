use super::{DeployField, InputMode, Overlay, components::centered_rect};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render whichever overlay is active. Call after all normal widgets.
pub(super) fn render_overlay(frame: &mut Frame<'_>, overlay: &Overlay, mode: InputMode) {
    match overlay {
        Overlay::Search { query } => render_search(frame, query, mode),
        Overlay::Deploy {
            id,
            route,
            subdomain,
            wasm_path,
            focus,
        } => {
            render_deploy(frame, id, route, subdomain, wasm_path, *focus, mode);
        }
        Overlay::InvokeResult { body, scroll } => render_invoke_result(frame, body, *scroll),
        // Confirm and CreateKey are rendered inline in ui.rs (they were there first).
        _ => {}
    }
}

fn render_search(frame: &mut Frame<'_>, query: &str, mode: InputMode) {
    let cursor = if mode == InputMode::Insert { "▌" } else { "█" };
    // Narrow bar at the bottom of the screen — adjust height here.
    let area = bottom_bar(3, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "/",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(query, Style::default().fg(Color::White)),
            Span::styled(cursor, Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("[i] insert  [Esc] normal/apply", Style::default().fg(Color::DarkGray)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Search")),
        area,
    );
}

fn render_deploy(
    frame: &mut Frame<'_>,
    id: &str,
    route: &str,
    subdomain: &str,
    wasm_path: &str,
    focus: DeployField,
    mode: InputMode,
) {
    // Popup dimensions — adjust width/height here.
    let (w, h): (u16, u16) = (60, 14);
    let area = centered_rect(w, h, frame.area());
    frame.render_widget(Clear, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // id
            Constraint::Length(2), // route
            Constraint::Length(2), // subdomain
            Constraint::Length(2), // wasm_path
            Constraint::Length(1), // spacer
            Constraint::Length(1), // hints
        ])
        .margin(1)
        .split(area);

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Deploy Function"),
        area,
    );

    let fields: &[(&str, &str, DeployField, bool)] = &[
        ("ID        ", id, DeployField::Id, focus == DeployField::Id),
        (
            "Route     ",
            route,
            DeployField::Route,
            focus == DeployField::Route,
        ),
        (
            "Subdomain ",
            subdomain,
            DeployField::Subdomain,
            focus == DeployField::Subdomain,
        ),
        (
            "WASM path ",
            wasm_path,
            DeployField::WasmPath,
            focus == DeployField::WasmPath,
        ),
    ];

    for (i, (label, value, _, active)) in fields.iter().enumerate() {
        let color = if *active {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let cursor = if *active {
            if mode == InputMode::Insert { "▌" } else { "█" }
        } else { "" };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    *label,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(*value, Style::default().fg(Color::White)),
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
            ])),
            inner[i],
        );
    }

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[i]", Style::default().fg(Color::Cyan)),
            Span::raw(" insert  "),
            Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
            Span::raw(" next  "),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" deploy  "),
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" normal/cancel"),
        ])),
        inner[5],
    );
}

pub(super) fn render_invoke_result(frame: &mut Frame<'_>, body: &str, scroll: u16) {
    // Popup dimensions — adjust width/height here.
    let (w, h): (u16, u16) = (70, 20);
    let area = centered_rect(w, h, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::raw(body))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Response  [↑/↓] scroll  [c] copy  [q/Esc] close"),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
}

/// A thin bar anchored to the bottom of `area`. Adjust `height` here.
fn bottom_bar(height: u16, area: Rect) -> Rect {
    Rect::new(
        area.x,
        area.y + area.height.saturating_sub(height),
        area.width,
        height.min(area.height),
    )
}
