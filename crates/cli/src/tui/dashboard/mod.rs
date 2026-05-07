use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use tokio::time::timeout;

// mod terminal;
// mod ui;

use crate::client::{FunctionRecord, KeyRecord, RuneClient};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(15);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

pub async fn run(client: RuneClient) -> anyhow::Result<()> {
    let mut tui = Tui::enter()?;
    let mut app = DashboardApp::new(client);
    let result = app.run(&mut tui).await;
    tui.exit()?;
    result
}

struct DashboardApp {
    client: RuneClient,
    tab: DashboardTab,
    functions: Vec<FunctionRecord>,
    keys: Vec<KeyRecord>,
    function_state: ListState,
    key_state: ListState,
    help_open: bool,
    status: String,
    last_refresh: Option<Instant>,
}

impl DashboardApp {
    fn new(client: RuneClient) -> Self {
        let mut function_state = ListState::default();
        function_state.select(Some(0));
        let mut key_state = ListState::default();
        key_state.select(Some(0));

        Self {
            client,
            tab: DashboardTab::Functions,
            functions: Vec::new(),
            keys: Vec::new(),
            function_state,
            key_state,
            help_open: false,
            status: "Loading control plane data...".into(),
            last_refresh: None,
        }
    }

    async fn run(&mut self, tui: &mut Tui) -> anyhow::Result<()> {
        self.refresh().await;

        loop {
            tui.terminal.draw(|frame| self.render(frame))?;

            if self.should_auto_refresh() {
                self.refresh().await;
                continue;
            }

            if !event::poll(POLL_INTERVAL)? {
                continue;
            }

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('?') => self.help_open = !self.help_open,
                KeyCode::Esc if self.help_open => self.help_open = false,
                KeyCode::Tab => self.next_tab(),
                KeyCode::BackTab => self.previous_tab(),
                KeyCode::Char('r') => self.refresh().await,
                KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => self.select_next(),
                KeyCode::Home | KeyCode::Char('g') => self.select_first(),
                KeyCode::End | KeyCode::Char('G') => self.select_last(),
                KeyCode::Char('d') if self.tab == DashboardTab::Functions => {
                    self.delete_selected_function().await;
                }
                KeyCode::Char('x') if self.tab == DashboardTab::Keys => {
                    self.revoke_selected_key().await;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
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
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Selected function"),
                    )
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
            Span::styled(
                self.mode_label(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
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
            Span::styled(
                "Tab",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" switch  "),
            Span::styled(
                "r",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" refresh  "),
            Span::styled(
                "?",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
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
        let area = centered_rect(42, 16, frame.area());
        frame.render_widget(Clear, area);

        let help = Paragraph::new(Text::from(vec![
            Line::from(vec![Span::styled(
                "Help",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Tab",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" switch view"),
            ]),
            Line::from(vec![
                Span::styled(
                    "↑/↓",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" move selection"),
            ]),
            Line::from(vec![
                Span::styled(
                    "r",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" refresh"),
            ]),
            Line::from(vec![
                Span::styled(
                    "d",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" delete function"),
            ]),
            Line::from(vec![
                Span::styled(
                    "x",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" revoke key"),
            ]),
            Line::from(vec![
                Span::styled(
                    "?",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" toggle help"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                Span::raw(" closes this popup"),
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
                        function
                            .subdomain
                            .clone()
                            .unwrap_or_else(|| "—".to_string()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("WASM", Style::default().fg(Color::DarkGray)),
                    Span::raw(": "),
                    Span::styled(
                        function.wasm_path.clone(),
                        Style::default().fg(Color::White),
                    ),
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
                    Span::styled(
                        key.created_at.to_string(),
                        Style::default().fg(Color::White),
                    ),
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

    async fn refresh(&mut self) {
        let refresh_result = timeout(REFRESH_TIMEOUT, async {
            tokio::join!(self.client.list_functions(), self.client.list_keys())
        })
        .await;

        match refresh_result {
            Ok((functions_result, keys_result)) => match (functions_result, keys_result) {
                (Ok(functions), Ok(keys)) => {
                    self.functions = functions;
                    self.keys = keys;
                    self.clamp_selection();
                    self.last_refresh = Some(Instant::now());
                    self.status = format!(
                        "Loaded {} function(s) and {} key(s).",
                        self.functions.len(),
                        self.keys.len()
                    );
                }
                (Err(err), _) => {
                    self.status = format!("Failed to load functions: {err}");
                }
                (_, Err(err)) => {
                    self.status = format!("Failed to load API keys: {err}");
                }
            },
            Err(_) => {
                self.status = "Control plane unavailable.".into();
            }
        }
    }

    async fn delete_selected_function(&mut self) {
        let Some(function) = self.selected_function().cloned() else {
            self.status = "No function selected to delete.".into();
            return;
        };

        match self.client.delete_function(&function.id).await {
            Ok(()) => {
                self.status = format!("Deleted function '{}'.", function.id);
                self.refresh().await;
            }
            Err(err) => {
                self.status = format!("Failed to delete '{}': {err}", function.id);
            }
        }
    }

    async fn revoke_selected_key(&mut self) {
        let Some(key) = self.selected_key().cloned() else {
            self.status = "No API key selected to revoke.".into();
            return;
        };

        match self.client.revoke_key(&key.id).await {
            Ok(()) => {
                self.status = format!("Revoked key '{}'.", key.name);
                self.refresh().await;
            }
            Err(err) => {
                self.status = format!("Failed to revoke key '{}': {err}", key.name);
            }
        }
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
    }

    fn previous_tab(&mut self) {
        self.tab = self.tab.previous();
    }

    fn select_next(&mut self) {
        let len = self.active_len();
        if len == 0 {
            self.active_state_mut().select(None);
            return;
        }

        let next = match self.active_state().selected() {
            Some(index) if index + 1 < len => index + 1,
            _ => len - 1,
        };
        self.active_state_mut().select(Some(next));
    }

    fn select_previous(&mut self) {
        let len = self.active_len();
        if len == 0 {
            self.active_state_mut().select(None);
            return;
        }

        let previous = match self.active_state().selected() {
            Some(index) if index > 0 => index - 1,
            _ => 0,
        };
        self.active_state_mut().select(Some(previous));
    }

    fn select_first(&mut self) {
        if self.active_len() == 0 {
            self.active_state_mut().select(None);
        } else {
            self.active_state_mut().select(Some(0));
        }
    }

    fn select_last(&mut self) {
        let len = self.active_len();
        if len == 0 {
            self.active_state_mut().select(None);
        } else {
            self.active_state_mut().select(Some(len - 1));
        }
    }

    fn clamp_selection(&mut self) {
        clamp_list_state(&mut self.function_state, self.functions.len());
        clamp_list_state(&mut self.key_state, self.keys.len());
    }

    fn should_auto_refresh(&self) -> bool {
        self.last_refresh
            .is_none_or(|last| last.elapsed() >= AUTO_REFRESH_INTERVAL)
    }

    fn selected_function(&self) -> Option<&FunctionRecord> {
        self.function_state
            .selected()
            .and_then(|index| self.functions.get(index))
    }

    fn selected_key(&self) -> Option<&KeyRecord> {
        self.key_state
            .selected()
            .and_then(|index| self.keys.get(index))
    }

    fn active_len(&self) -> usize {
        match self.tab {
            DashboardTab::Functions => self.functions.len(),
            DashboardTab::Keys => self.keys.len(),
        }
    }

    fn active_state(&self) -> &ListState {
        match self.tab {
            DashboardTab::Functions => &self.function_state,
            DashboardTab::Keys => &self.key_state,
        }
    }

    fn active_state_mut(&mut self) -> &mut ListState {
        match self.tab {
            DashboardTab::Functions => &mut self.function_state,
            DashboardTab::Keys => &mut self.key_state,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardTab {
    Functions,
    Keys,
}

impl DashboardTab {
    fn next(self) -> Self {
        match self {
            Self::Functions => Self::Keys,
            Self::Keys => Self::Functions,
        }
    }

    fn previous(self) -> Self {
        self.next()
    }
}

fn clamp_list_state(state: &mut ListState, len: usize) {
    match (len, state.selected()) {
        (0, _) => state.select(None),
        (_, None) => state.select(Some(0)),
        (len, Some(index)) if index >= len => state.select(Some(len - 1)),
        _ => {}
    }
}

struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn exit(&mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_list_state_selects_first_item_when_needed() {
        let mut state = ListState::default();
        clamp_list_state(&mut state, 3);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn clamp_list_state_drops_selection_for_empty_lists() {
        let mut state = ListState::default();
        state.select(Some(4));
        clamp_list_state(&mut state, 0);
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn dashboard_tab_cycles_between_views() {
        assert_eq!(DashboardTab::Functions.next(), DashboardTab::Keys);
        assert_eq!(DashboardTab::Keys.next(), DashboardTab::Functions);
    }
}
