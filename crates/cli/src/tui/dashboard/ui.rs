use super::{
    DashboardApp, DashboardTab, Overlay,
    components::{bottom_right_rect, centered_rect, key_span, nav_item},
    overlays::render_overlay,
};

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
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(10),   // body
                Constraint::Length(1), // toast line
                Constraint::Length(1), // footer
            ])
            .split(frame.area());

        let version = self.version_tag();
        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(42), // nav tabs (wider for Config)
                Constraint::Min(0),
                Constraint::Length(3), // connection dot
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
                Span::raw(" "),
                Span::styled("│", Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                nav_item("Config", self.tab == DashboardTab::Config),
            ])),
            top[0],
        );

        // Connection status dot: green = connected, red = offline, yellow = loading
        let (dot, dot_color) = if self.loading {
            ("● ", Color::Yellow)
        } else if self.connected {
            ("● ", Color::Green)
        } else {
            ("● ", Color::Red)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(dot, Style::default().fg(dot_color)))
                .alignment(Alignment::Right),
            top[2],
        );

        frame.render_widget(
            Paragraph::new(version)
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::DarkGray)),
            top[3],
        );

        match self.tab {
            DashboardTab::Functions => self.render_functions_tab(frame, outer[1]),
            DashboardTab::Keys => self.render_keys_tab(frame, outer[1]),
            DashboardTab::Config => self.render_config_tab(frame, outer[1]),
        }

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                self.toast_text().to_owned(),
                Style::default().fg(Color::DarkGray),
            )))
            .alignment(Alignment::Right),
            outer[2],
        );

        let footer = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10), // mode label
                Constraint::Min(0),     // center hints
                Constraint::Length(0),  // unused
            ])
            .split(outer[3]);

        frame.render_widget(Paragraph::new(self.footer_left_line()), footer[0]);
        frame.render_widget(
            Paragraph::new(self.footer_center_line())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            footer[1],
        );
        frame.render_widget(Paragraph::new(""), footer[2]); // empty — toast moved to outer[2]

        if self.help_open {
            self.render_help_popup(frame);
        }
        match &self.overlay {
            Some(Overlay::Confirm { message, .. }) => {
                render_confirm_popup(frame, &message.clone());
            }
            Some(Overlay::CreateKey { input }) => {
                render_create_key_popup(frame, &input.clone(), self.input_mode);
            }
            Some(other) => render_overlay(frame, other, self.input_mode),
            None => {}
        }
    }

    fn render_functions_tab(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // filtered_functions() returns owned Vec — no borrow conflict with function_state.
        let filtered = self.filtered_functions();
        let title = if self.search.is_empty() {
            format!("Functions ({})", self.functions.len())
        } else {
            format!("Functions ({}/{})", filtered.len(), self.functions.len())
        };

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|f| {
                let subdomain = f.subdomain.clone().unwrap_or_else(|| "—".into());
                ListItem::new(vec![
                    Line::from(Span::styled(
                        f.id.clone(),
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                    Line::from(vec![
                        Span::styled("route", Style::default().fg(Color::DarkGray)),
                        Span::raw(": "),
                        Span::styled(f.route.clone(), Style::default().fg(Color::White)),
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
            .block(Block::default().borders(Borders::ALL).title(title))
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
                        .borders(Borders::ALL)
                        .title("Selected function"),
                )
                .wrap(Wrap { trim: false }),
            panels[1],
        );
    }

    fn render_keys_tab(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // filtered_keys() returns owned Vec — no borrow conflict with key_state.
        let filtered = self.filtered_keys();
        let title = if self.search.is_empty() {
            format!("API Keys ({})", self.keys.len())
        } else {
            format!("API Keys ({}/{})", filtered.len(), self.keys.len())
        };

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|k| {
                ListItem::new(vec![
                    Line::from(Span::styled(
                        k.name.clone(),
                        Style::default().fg(Color::Yellow).bold(),
                    )),
                    Line::from(vec![
                        Span::styled("id", Style::default().fg(Color::DarkGray)),
                        Span::raw(": "),
                        Span::styled(k.id.clone(), Style::default().fg(Color::Gray)),
                    ]),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
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
                .block(Block::default().borders(Borders::ALL).title("Selected key"))
                .wrap(Wrap { trim: false }),
            panels[1],
        );
    }

    fn render_config_tab(&mut self, frame: &mut Frame<'_>, area: Rect) {
        use super::ConfigField;

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Configuration");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // control plane
                Constraint::Length(2), // function url
                Constraint::Length(2), // api key
                Constraint::Length(1), // spacer
                Constraint::Length(1), // hint
                Constraint::Min(0),
            ])
            .margin(1)
            .split(inner);

        let fields: &[(&str, &str, ConfigField)] = &[
            (
                "Control Plane URL",
                &self.config_edit.control_plane,
                ConfigField::ControlPlane,
            ),
            (
                "Function URL     ",
                &self.config_edit.function_url,
                ConfigField::FunctionUrl,
            ),
            (
                "API Key          ",
                &self.config_edit.api_key,
                ConfigField::ApiKey,
            ),
        ];

        for (i, (label, value, field)) in fields.iter().enumerate() {
            let active = self.config_edit.focus == *field;
            let color = if active { Color::Cyan } else { Color::DarkGray };
            let cursor = if active {
                if self.input_mode == super::InputMode::Insert { "▌" } else { "█" }
            } else { "" };
            // Mask the API key except when focused
            let display = if *field == ConfigField::ApiKey && !active && !value.is_empty() {
                format!("{}…{}", &value[..value.len().min(12)], "*".repeat(6))
            } else {
                value.to_string()
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        *label,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(": "),
                    Span::styled(display, Style::default().fg(Color::White)),
                    Span::styled(cursor, Style::default().fg(Color::Cyan)),
                ])),
                rows[i],
            );
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                key_span("↑/↓"),
                Span::raw(" next/prev  "),
                key_span("i"),
                Span::raw(" insert  "),
                key_span("F2"),
                Span::raw(" save"),
            ])),
            rows[4],
        );
    }

    fn footer_left_line(&self) -> Line<'static> {
        let is_input_ctx = matches!(
            &self.overlay,
            Some(Overlay::CreateKey { .. }) | Some(Overlay::Search { .. }) | Some(Overlay::Deploy { .. })
        ) || (self.overlay.is_none() && self.tab == DashboardTab::Config);

        let (label, color) = if is_input_ctx {
            match self.input_mode {
                super::InputMode::Insert => ("INSERT", Color::Green),
                super::InputMode::Normal => ("NORMAL", Color::Cyan),
            }
        } else {
            match &self.overlay {
                Some(Overlay::Confirm { .. }) => ("CONFIRM", Color::Red),
                Some(Overlay::InvokeResult { .. }) => ("INVOKE", Color::Cyan),
                None if self.help_open => ("HELP", Color::Cyan),
                _ => ("NORMAL", Color::Cyan),
            }
        };
        let mut spans = vec![Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )];
        if !self.search.is_empty() && self.overlay.is_none() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("/{}", self.search),
                Style::default().fg(Color::Magenta),
            ));
        }
        Line::from(spans)
    }

    fn footer_center_line(&self) -> Line<'static> {
        let refresh = if self.loading {
            "loading…"
        } else {
            self.last_refresh
                .map(|_| "refreshed")
                .unwrap_or("not loaded")
        };
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

    pub(super) fn version_tag(&self) -> String {
        format!("rune-{}", env!("CARGO_PKG_VERSION"))
    }

    fn render_help_popup(&self, frame: &mut Frame<'_>) {
        // Popup dimensions — adjust width/height here if you add/remove entries.
        let (w, h): (u16, u16) = (40, 20);
        let area = bottom_right_rect(w, h, frame.area());

        frame.render_widget(
            Block::default().style(Style::default().bg(Color::DarkGray)),
            area,
        );
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from(key_span("Navigation")),
                Line::from(vec![key_span("Tab/BackTab"), Span::raw(" switch view")]),
                Line::from(vec![key_span("↑/↓  j/k"), Span::raw(" move")]),
                Line::from(vec![key_span("g/G"), Span::raw(" first/last")]),
                Line::from(vec![key_span("Ctrl-d/u"), Span::raw(" half-page scroll")]),
                Line::from(""),
                Line::from(key_span("Actions")),
                Line::from(vec![key_span("r"), Span::raw(" refresh")]),
                Line::from(vec![key_span("/"), Span::raw(" search/filter")]),
                Line::from(vec![key_span("c"), Span::raw(" copy URL / key ID")]),
                Line::from(vec![key_span("i"), Span::raw(" invoke function (GET)")]),
                Line::from(vec![key_span("D"), Span::raw(" deploy form (Functions)")]),
                Line::from(vec![key_span("d"), Span::raw(" delete function")]),
                Line::from(vec![key_span("x"), Span::raw(" revoke key")]),
                Line::from(vec![key_span("n"), Span::raw(" new key (Keys tab)")]),
                Line::from(""),
                Line::from(vec![key_span("?"), Span::raw(" toggle help")]),
                Line::from(vec![
                    Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                    Span::raw(" close / clear search"),
                ]),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Help")),
            area,
        );
    }

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
                    Span::raw(" delete  "),
                    Span::styled("[i]", Style::default().fg(Color::Cyan).bold()),
                    Span::raw(" invoke  "),
                    Span::styled("[c]", Style::default().fg(Color::Green).bold()),
                    Span::raw(" copy URL"),
                ]),
            ]),
            None => Text::from(vec![
                Line::from(Span::styled(
                    "No functions deployed.",
                    Style::default().fg(Color::Yellow).bold(),
                )),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(Color::Gray)),
                    Span::styled("[D]", Style::default().fg(Color::Green).bold()),
                    Span::styled(" to deploy one.", Style::default().fg(Color::Gray)),
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
                    Span::raw(" revoke  "),
                    Span::styled("[n]", Style::default().fg(Color::Green).bold()),
                    Span::raw(" new key  "),
                    Span::styled("[c]", Style::default().fg(Color::Cyan).bold()),
                    Span::raw(" copy ID"),
                ]),
            ]),
            None => Text::from(vec![
                Line::from(Span::styled(
                    "No API keys found.",
                    Style::default().fg(Color::Yellow).bold(),
                )),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(Color::Gray)),
                    Span::styled("[n]", Style::default().fg(Color::Green).bold()),
                    Span::styled(" to create one.", Style::default().fg(Color::Gray)),
                ]),
            ]),
        }
    }
}

fn render_confirm_popup(frame: &mut Frame<'_>, message: &str) {
    let (w, h): (u16, u16) = (50, 6);
    let area = centered_rect(w, h, frame.area());
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::DarkGray)),
        area,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(Span::styled(message, Style::default().fg(Color::White))),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "[y]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" confirm  "),
                Span::styled("[n/Esc]", Style::default().fg(Color::DarkGray)),
                Span::raw(" cancel"),
            ]),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Confirm")),
        area,
    );
}

fn render_create_key_popup(frame: &mut Frame<'_>, input: &str, mode: super::InputMode) {
    let cursor = if mode == super::InputMode::Insert { "▌" } else { "█" };
    let (w, h): (u16, u16) = (44, 6);
    let area = centered_rect(w, h, frame.area());
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::DarkGray)),
        area,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
                Span::styled(input, Style::default().fg(Color::White)),
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "[Enter]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" create  "),
                Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
                Span::raw(" cancel"),
            ]),
        ]))
        .block(Block::default().borders(Borders::ALL).title("New API Key")),
        area,
    );
}
