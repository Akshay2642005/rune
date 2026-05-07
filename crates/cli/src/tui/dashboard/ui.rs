use super::{DashboardApp, DashboardTab};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

impl DashboardApp {
    pub(super) fn render(&mut self, frame: &mut Frame<'_>) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(10), Constraint::Length(1)])
            .split(frame.area());

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

        let body = outer[1];
        match self.tab {
            DashboardTab::Functions => {
                let panels = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(body);

                let items = self
                    .functions
                    .iter()
                    .map(|function| {
                        let subdomain = function.subdomain.as_deref().unwrap_or("—");
                        ListItem::new(vec![
                            Line::from(vec![Span::styled(
                                function.id.as_str(),
                                Style::default().fg(Color::Yellow).bold(),
                            )]),
                            Line::from(vec![
                                Span::styled("route", Style::default().fg(Color::DarkGray)),
                                Span::raw(": "),
                                Span::styled(
                                    function.route.as_str(),
                                    Style::default().fg(Color::White),
                                ),
                            ]),
                            Line::from(vec![
                                Span::styled("subdomain", Style::default().fg(Color::DarkGray)),
                                Span::raw(": "),
                                Span::styled(subdomain, Style::default().fg(Color::Gray)),
                            ]),
                        ])
                    })
                    .collect::<Vec<_>>();

                let list = List::new(items)
                    .block(
                        Block::default()
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

                let details = Paragraph::new(self.function_details())
                    .block(Block::default().borders(Borders::ALL).title("Selected function"))
                    .wrap(Wrap { trim: false });
                frame.render_widget(details, panels[1]);
            }
            DashboardTab::Keys => {
                let panels = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(body);

                let items = self
                    .keys
                    .iter()
                    .map(|key| {
                        ListItem::new(vec![
                            Line::from(vec![Span::styled(
                                key.name.as_str(),
                                Style::default().fg(Color::Yellow).bold(),
                            )]),
                            Line::from(vec![
                                Span::styled("id", Style::default().fg(Color::DarkGray)),
                                Span::raw(": "),
                                Span::styled(key.id.as_str(), Style::default().fg(Color::Gray)),
                            ]),
                        ])
                    })
                    .collect::<Vec<_>>();

                let list = List::new(items)
                    .block(
                        Block::default()
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

                let details = Paragraph::new(self.key_details())
                    .block(Block::default().borders(Borders::ALL).title("Selected key"))
                    .wrap(Wrap { trim: false });
                frame.render_widget(details, panels[1]);
            }
        }

        let footer = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(46),
                Constraint::Percentage(20),
            ])
            .split(outer[2]);

        frame.render_widget(
            Paragraph::new(self.footer_left_line()).style(Style::default().fg(Color::Green)),
            footer[0],
        );
        frame.render_widget(
            Paragraph::new(self.footer_center_line())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            footer[1],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                self.footer_right_line(),
                Style::default().fg(Color::DarkGray),
            )]))
            .alignment(Alignment::Right),
            footer[2],
        );

        if self.help_open {
            self.render_help_popup(frame);
        }
    }

    fn footer_left_line(&self) -> Line<'static> {
        Line::from(vec![
            Span::styled(self.mode_label(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(self.status.clone(), Style::default().fg(Color::DarkGray)),
        ])
    }

    fn footer_center_line(&self) -> Line<'static> {
        let refresh = self
            .last_refresh
            .map(|_| "refreshed")
            .unwrap_or("not loaded");

        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" switch  "),
            Span::styled("r", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" refresh  "),
            Span::styled("?", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" help  "),
            Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" quit  "),
            Span::styled(format!("({refresh})"), Style::default().fg(Color::DarkGray)),
        ])
    }

    fn footer_right_line(&self) -> String {
        format!("{} fn  {} key", self.functions.len(), self.keys.len())
    }

    fn mode_label(&self) -> &'static str {
        if self.help_open { "HELP" } else { "NORMAL" }
    }

    fn version_tag(&self) -> String {
        format!("rune-{}", env!("CARGO_PKG_VERSION"))
    }

    fn render_help_popup(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(34, 12, frame.area());
        frame.render_widget(Clear, area);

        let help = Paragraph::new(Text::from(vec![
            Line::from(vec![Span::styled(
                "Help",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" switch view"),
            ]),
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" move"),
            ]),
            Line::from(vec![
                Span::styled("r", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" refresh"),
            ]),
            Line::from(vec![
                Span::styled("?", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" toggle help"),
            ]),
            Line::from(vec![
                Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                Span::raw(" close"),
            ]),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Help"));

        frame.render_widget(help, area);
    }

    fn function_details(&self) -> Text<'static> {
        match self.selected_function() {
            Some(function) => Text::from(vec![
                Line::from(vec![
                    Span::styled("ID", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(
                        function.id.clone(),
                        Style::default().fg(Color::Yellow).bold(),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Route", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(function.route.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Subdomain", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(
                        function.subdomain.clone().unwrap_or_else(|| "—".to_string()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("WASM", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(function.wasm_path.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[d]", Style::default().fg(Color::Red).bold()),
                    Span::raw(" delete this function"),
                ]),
            ]),
            None => Text::from(vec![
                Line::from(vec![Span::styled(
                    "No functions deployed.",
                    Style::default().fg(Color::Yellow).bold(),
                )]),
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
            Some(key) => Text::from(vec![
                Line::from(vec![
                    Span::styled("Name", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(key.name.clone(), Style::default().fg(Color::Yellow).bold()),
                ]),
                Line::from(vec![
                    Span::styled("ID", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(key.id.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Created At", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(key.created_at.to_string(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("[x]", Style::default().fg(Color::Red).bold()),
                    Span::raw(" revoke this key"),
                ]),
            ]),
            None => Text::from(vec![
                Line::from(vec![Span::styled(
                    "No API keys found.",
                    Style::default().fg(Color::Yellow).bold(),
                )]),
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

fn nav_item(label: &'static str, active: bool) -> Span<'static> {
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

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width) / 2),
            Constraint::Length(width),
            Constraint::Percentage((100 - width) / 2),
        ])
        .split(vertical[1])[1]
}
