use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend, widgets::ListState};
use tokio::time::timeout;

mod components;
mod ui;

use crate::client::{FunctionRecord, KeyRecord, RuneClient};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(15);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// How long a toast notification stays fully visible before it disappears.
const TOAST_TTL: Duration = Duration::from_secs(4);

/// A transient status message shown in the bottom-right footer.
struct Notification {
    message: String,
    born: Instant,
}

impl Notification {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            born: Instant::now(),
        }
    }

    /// Returns `None` once the TTL has elapsed.
    fn live(&self) -> Option<&str> {
        (self.born.elapsed() < TOAST_TTL).then_some(&self.message)
    }
}

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
    toast: Option<Notification>,
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
            toast: None,
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

    async fn refresh(&mut self) {
        let result = timeout(REFRESH_TIMEOUT, async {
            tokio::join!(self.client.list_functions(), self.client.list_keys())
        })
        .await;

        match result {
            Ok((Ok(functions), Ok(keys))) => {
                self.functions = functions;
                self.keys = keys;
                self.clamp_selection();
                self.last_refresh = Some(Instant::now());
            }
            Ok((Err(err), _)) => {
                self.toast = Some(Notification::new(format!(
                    "Failed to load functions: {err}"
                )))
            }
            Ok((_, Err(err))) => {
                self.toast = Some(Notification::new(format!("Failed to load API keys: {err}")))
            }
            Err(_) => self.toast = Some(Notification::new("Control plane unavailable.")),
        }
    }

    async fn delete_selected_function(&mut self) {
        let Some(function) = self.selected_function().cloned() else {
            self.toast = Some(Notification::new("No function selected to delete."));
            return;
        };
        match self.client.delete_function(&function.id).await {
            Ok(()) => {
                self.toast = Some(Notification::new(format!(
                    "Deleted function '{}'.",
                    function.id
                )));
                self.refresh().await;
            }
            Err(err) => {
                self.toast = Some(Notification::new(format!(
                    "Failed to delete '{}': {err}",
                    function.id
                )))
            }
        }
    }

    async fn revoke_selected_key(&mut self) {
        let Some(key) = self.selected_key().cloned() else {
            self.toast = Some(Notification::new("No API key selected to revoke."));
            return;
        };
        match self.client.revoke_key(&key.id).await {
            Ok(()) => {
                self.toast = Some(Notification::new(format!("Revoked key '{}'.", key.name)));
                self.refresh().await;
            }
            Err(err) => {
                self.toast = Some(Notification::new(format!(
                    "Failed to revoke key '{}': {err}",
                    key.name
                )))
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
            Some(i) if i + 1 < len => i + 1,
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
        let prev = match self.active_state().selected() {
            Some(i) if i > 0 => i - 1,
            _ => 0,
        };
        self.active_state_mut().select(Some(prev));
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
            .and_then(|i| self.functions.get(i))
    }

    fn selected_key(&self) -> Option<&KeyRecord> {
        self.key_state.selected().and_then(|i| self.keys.get(i))
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
        (len, Some(i)) if i >= len => state.select(Some(len - 1)),
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
        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(stdout))?,
        })
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
