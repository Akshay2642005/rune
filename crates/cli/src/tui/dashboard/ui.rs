use super::{
    DashboardApp, DashboardTab,
    components::{bottom_right_rect, key_span, nav_item},
};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

impl DashboardApp {
    pub(super) fn render(&mut self, frame: &mut Frame<'_>) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(frame.area());

        // ── header ──────────────────────────────────────────────────────────
        let version = self.version_tag();
        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30),
                Constraint::Min(0),
                Constraint::Length((version.len() + 1) as u16),
            ])
            .split(outer[0]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                nav_item("Functions", self.tab == DashboardTab::Functions),
                Span::raw(" "),
                Span::styled("│", Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                nav_item("API Keys", self.tab == DashboardTab::Keys),
            ])),
            top[0],
        );
        frame.render_widget(
            Paragraph::new(version)
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::DarkGray)),
            top[2],
        );

        // ── body ─────────────────────────────────────────────────────────────
        match self.tab {
            DashboardTab::Functions => self.render_functions_tab(frame, outer[1]),
            DashboardTab::Keys => self.render_keys_tab(frame, outer[1]),
        }

        // ── footer ───────────────────────────────────────────────────────────
        let footer = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(46),
                Constraint::Percentage(20),
            ])
            .split(outer[2]);

        frame.render_widget(Paragraph::new(self.footer_left_line()), footer[0]);
        frame.render_widget(
            Paragraph::new(self.footer_center_line())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            footer[1],
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                self.footer_right_line(),
                Style::default().fg(Color::DarkGray),
            )))
            .alignment(Alignment::Right),
            footer[2],
        );

        if self.help_open {
            self.render_help_popup(frame);
        }
    }

    fn render_functions_tab(&mut self, frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let items: Vec<ListItem> = self
            .functions
            .iter()
            .map(|f| {
                let subdomain = f.subdomain.as_deref().unwrap_or("—");
                ListItem::new(vec![
                    Line::from(Span::styled(
                        f.id.as_str(),
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                    Line::from(vec![
                        Span::styled("route", Style::default().fg(Color::DarkGray)),
                        Span::raw(": "),
                        Span::styled(f.route.as_str(), Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("subdomain", Style::default().fg(Color::DarkGray)),
                        Span::raw(": "),
                        Span::styled(subdomain, Style::default().fg(Color::Gray)),
                    ]),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(format!("Functions ({})", self.functions.len())),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");
        frame.render_stateful_widget(list, panels[0], &mut self.function_state);

        frame.render_widget(
            Paragraph::new(self.function_details())
                .block(
                    Block::default()
                        .border_type(ratatui::widgets::BorderType::Rounded)
                        .borders(Borders::ALL)
                        .title("Selected function"),
                )
                .wrap(Wrap { trim: false }),
            panels[1],
        );
    }

    fn render_keys_tab(&mut self, frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let items: Vec<ListItem> = self
            .keys
            .iter()
            .map(|k| {
                ListItem::new(vec![
                    Line::from(Span::styled(
                        k.name.as_str(),
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                    Line::from(vec![
                        Span::styled("id", Style::default().fg(Color::DarkGray)),
                        Span::raw(": "),
                        Span::styled(k.id.as_str(), Style::default().fg(Color::Gray)),
                    ]),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(format!("API Keys ({})", self.keys.len())),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");
        frame.render_stateful_widget(list, panels[0], &mut self.key_state);

        frame.render_widget(
            Paragraph::new(self.key_details())
                .block(
                    Block::default()
                        .border_type(ratatui::widgets::BorderType::Rounded)
                        .borders(Borders::ALL)
                        .title("Selected key"),
                )
                .wrap(Wrap { trim: false }),
            panels[1],
        );
    }

    // ── footer helpers ───────────────────────────────────────────────────────

    fn footer_left_line(&self) -> Line<'static> {
        // Left footer: current mode only.
        Line::from(Span::styled(
            self.mode_label(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
    }

    fn footer_center_line(&self) -> Line<'static> {
        let refresh = self
            .last_refresh
            .map(|_| "refreshed")
            .unwrap_or("not loaded");
        Line::from(vec![
            key_span("Tab"),
            Span::raw(" switch  "),
            key_span("r"),
            Span::raw(" refresh  "),
            key_span("?"),
            Span::raw(" help  "),
            Span::styled(
                "q",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" quit  "),
            Span::styled(format!("({refresh})"), Style::default().fg(Color::DarkGray)),
        ])
    }

    /// Right footer: fading toast notification. Empty once the TTL expires.
    fn footer_right_line(&self) -> String {
        self.toast
            .as_ref()
            .and_then(|n| n.live())
            .unwrap_or("")
            .to_owned()
    }

    pub(super) fn version_tag(&self) -> String {
        format!("rune-{}", env!("CARGO_PKG_VERSION"))
    }

    fn mode_label(&self) -> &'static str {
        if self.help_open { "HELP" } else { "NORMAL" }
    }

    // ── help popup ───────────────────────────────────────────────────────────

    fn render_help_popup(&self, frame: &mut Frame<'_>) {
        // Popup dimensions — adjust width/height here if you add/remove entries.
        let (w, h): (u16, u16) = (34, 12);
        let area = bottom_right_rect(w, h, frame.area());

        // Shadow: a DarkGray block 1 col right + 1 row below the popup.
        // Simulates depth; DarkGray is the closest ratatui gets to transparency.

        frame.render_widget(
            Block::default().style(Style::default().bg(Color::DarkGray)),
            area,
        );

        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from(key_span("Help")),
                Line::from(""),
                Line::from(vec![key_span("Tab"), Span::raw(" switch view")]),
                Line::from(vec![key_span("↑/↓"), Span::raw(" move")]),
                Line::from(vec![key_span("r"), Span::raw(" refresh")]),
                Line::from(vec![key_span("d"), Span::raw(" delete function")]),
                Line::from(vec![key_span("x"), Span::raw(" revoke key")]),
                Line::from(vec![key_span("?"), Span::raw(" toggle help")]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                    Span::raw(" close"),
                ]),
            ]))
            .block(
                Block::default()
                    .border_type(ratatui::widgets::BorderType::Plain)
                    .borders(Borders::ALL)
                    .title("Help"),
            ),
            area,
        );
    }

    // ── detail panels ────────────────────────────────────────────────────────

    fn function_details(&self) -> Text<'static> {
        match self.selected_function() {
            Some(f) => Text::from(vec![
                Line::from(vec![
                    Span::styled("ID", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(f.id.clone(), Style::default().fg(Color::Yellow).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Route", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(f.route.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Subdomain", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(
                        f.subdomain.clone().unwrap_or_else(|| "—".to_string()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("WASM", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(f.wasm_path.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[d]", Style::default().fg(Color::Red).bold()),
                    Span::raw(" delete this function"),
                ]),
            ]),
            None => Text::from(vec![
                Line::from(Span::styled(
                    "No functions deployed.",
                    Style::default().fg(Color::Yellow).bold(),
                )),
                Line::from(vec![
                    Span::styled("Deploy one with ", Style::default().fg(Color::Gray)),
                    Span::styled("rune deploy ...", Style::default().fg(Color::Cyan).bold()),
                    Span::styled(" and press ", Style::default().fg(Color::Gray)),
                    Span::styled("[r]", Style::default().fg(Color::Cyan).bold()),
                    Span::styled(" to refresh.", Style::default().fg(Color::Gray)),
                ]),
            ]),
        }
    }

    fn key_details(&self) -> Text<'static> {
        match self.selected_key() {
            Some(k) => Text::from(vec![
                Line::from(vec![
                    Span::styled("Name", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(k.name.clone(), Style::default().fg(Color::Yellow).bold()),
                ]),
                Line::from(vec![
                    Span::styled("ID", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(k.id.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Created At", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(k.created_at.to_string(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[x]", Style::default().fg(Color::Red).bold()),
                    Span::raw(" revoke this key"),
                ]),
            ]),
            None => Text::from(vec![
                Line::from(Span::styled(
                    "No API keys found.",
                    Style::default().fg(Color::Yellow).bold(),
                )),
                Line::from(vec![
                    Span::styled("Generate one with ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        "rune auth generate-key ...",
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                    Span::styled(" and press ", Style::default().fg(Color::Gray)),
                    Span::styled("[r]", Style::default().fg(Color::Cyan).bold()),
                    Span::styled(" to refresh.", Style::default().fg(Color::Gray)),
                ]),
            ]),
        }
    }
}
