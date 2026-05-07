use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend, widgets::ListState};
use tokio::sync::mpsc;
use tokio::time::timeout;

mod components;
mod overlays;
mod ui;

use crate::client::{FunctionRecord, KeyRecord, RuneClient};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(15);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const TOAST_TTL: Duration = Duration::from_secs(4);

pub(super) enum BgResult {
    Refreshed {
        functions: Vec<FunctionRecord>,
        keys: Vec<KeyRecord>,
    },
    RefreshFailed(String),
    ActionDone(String),
    ActionFailed(String),
    Invoked(String), // response body from invoke
}

pub(super) struct Notification {
    message: String,
    born: Instant,
}

impl Notification {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            born: Instant::now(),
        }
    }

    fn live(&self) -> Option<&str> {
        (self.born.elapsed() < TOAST_TTL).then_some(&self.message)
    }
}

/// Vim-style input mode shared across all text-input contexts.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum InputMode {
    #[default]
    Normal,
    Insert,
}

pub(super) enum ConfirmAction {
    DeleteFunction(FunctionRecord),
    RevokeKey(KeyRecord),
}

/// Deploy form field currently focused.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DeployField {
    Id,
    Route,
    Subdomain,
    WasmPath,
}

impl DeployField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Id => Self::Route,
            Self::Route => Self::Subdomain,
            Self::Subdomain => Self::WasmPath,
            Self::WasmPath => Self::Id,
        }
    }
    pub(super) fn prev(self) -> Self {
        match self {
            Self::Id => Self::WasmPath,
            Self::Route => Self::Id,
            Self::Subdomain => Self::Route,
            Self::WasmPath => Self::Subdomain,
        }
    }
}

pub(super) enum Overlay {
    Confirm {
        message: String,
        action: ConfirmAction,
    },
    CreateKey {
        input: String,
    },
    Search {
        query: String,
    },
    Deploy {
        id: String,
        route: String,
        subdomain: String,
        wasm_path: String,
        focus: DeployField,
    },
    InvokeResult {
        body: String,
        scroll: u16,
    },
}

// ── Config edit state ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigField { ControlPlane, FunctionUrl, ApiKey }

impl ConfigField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::ControlPlane => Self::FunctionUrl,
            Self::FunctionUrl => Self::ApiKey,
            Self::ApiKey => Self::ControlPlane,
        }
    }
    pub(super) fn prev(self) -> Self {
        match self {
            Self::ControlPlane => Self::ApiKey,
            Self::FunctionUrl => Self::ControlPlane,
            Self::ApiKey => Self::FunctionUrl,
        }
    }
}

pub(super) struct ConfigEdit {
    pub(super) control_plane: String,
    pub(super) function_url: String,
    pub(super) api_key: String,
    pub(super) focus: ConfigField,
}

impl ConfigEdit {
    fn from_client(client: &RuneClient) -> Self {
        Self {
            control_plane: client.server_url().to_owned(),
            function_url: client.function_url.clone(),
            api_key: client.api_key().to_owned(),
            focus: ConfigField::ControlPlane,
        }
    }

    pub(super) fn focused_mut(&mut self) -> &mut String {
        match self.focus {
            ConfigField::ControlPlane => &mut self.control_plane,
            ConfigField::FunctionUrl => &mut self.function_url,
            ConfigField::ApiKey => &mut self.api_key,
        }
    }
}

pub async fn run(client: RuneClient) -> anyhow::Result<()> {
    let mut tui = Tui::enter()?;
    let mut app = DashboardApp::new(client);
    let result = app.run(&mut tui).await;
    tui.exit()?;
    result
}

pub(super) struct DashboardApp {
    pub(super) client: RuneClient,
    pub(super) tab: DashboardTab,
    pub(super) functions: Vec<FunctionRecord>,
    pub(super) keys: Vec<KeyRecord>,
    pub(super) function_state: ListState,
    pub(super) key_state: ListState,
    pub(super) help_open: bool,
    pub(super) overlay: Option<Overlay>,
    pub(super) toast: Option<Notification>,
    pub(super) last_refresh: Option<Instant>,
    pub(super) loading: bool,
    /// Active search query — filters the visible list.
    pub(super) search: String,
    /// Whether the last refresh succeeded (for connection dot).
    pub(super) connected: bool,
    /// Editable config fields shown on the Config tab.
    pub(super) config_edit: ConfigEdit,
    /// Vim-style input mode for all text-input contexts.
    pub(super) input_mode: InputMode,
}

impl DashboardApp {
    fn new(client: RuneClient) -> Self {
        let mut function_state = ListState::default();
        function_state.select(Some(0));
        let mut key_state = ListState::default();
        key_state.select(Some(0));
        let config_edit = ConfigEdit::from_client(&client);

        Self {
            client,
            tab: DashboardTab::Functions,
            functions: Vec::new(),
            keys: Vec::new(),
            function_state,
            key_state,
            help_open: false,
            overlay: None,
            toast: None,
            last_refresh: None,
            loading: false,
            search: String::new(),
            connected: false,
            config_edit,
            input_mode: InputMode::Normal,
        }
    }

    async fn run(&mut self, tui: &mut Tui) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel::<BgResult>(8);
        self.spawn_refresh(tx.clone());

        'outer: loop {
            tui.terminal.draw(|frame| self.render(frame))?;

            // Drain all pending key events first so Shift+Tab (a multi-byte
            // escape sequence) is never delayed by the POLL_INTERVAL sleep.
            while event::poll(Duration::ZERO)? {
                let Event::Key(key) = event::read()? else { continue };
                if key.kind != KeyEventKind::Press { continue; }

                if self.overlay.is_some() {
                    self.handle_overlay_key(key.code, key.modifiers, tx.clone());
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') if self.tab != DashboardTab::Config => break 'outer,
                    KeyCode::Char('?') => self.help_open = !self.help_open,
                    KeyCode::Esc if self.help_open => self.help_open = false,
                    KeyCode::Esc if !self.search.is_empty() => {
                        self.search.clear();
                        self.clamp_selection();
                    }
                    KeyCode::Tab => { self.search.clear(); self.input_mode = InputMode::Normal; self.next_tab(); }
                    KeyCode::BackTab => { self.search.clear(); self.input_mode = InputMode::Normal; self.previous_tab(); }
                    KeyCode::Char('r') if self.tab != DashboardTab::Config && !self.loading => {
                        self.spawn_refresh(tx.clone());
                    }
                    // ── Config tab ────────────────────────────────────────────
                    KeyCode::Up if self.tab == DashboardTab::Config && self.input_mode == InputMode::Normal => {
                        self.config_edit.focus = self.config_edit.focus.prev();
                    }
                    KeyCode::Down if self.tab == DashboardTab::Config && self.input_mode == InputMode::Normal => {
                        self.config_edit.focus = self.config_edit.focus.next();
                    }
                    KeyCode::Char('k') if self.tab == DashboardTab::Config && self.input_mode == InputMode::Normal => {
                        self.config_edit.focus = self.config_edit.focus.prev();
                    }
                    KeyCode::Char('j') if self.tab == DashboardTab::Config && self.input_mode == InputMode::Normal => {
                        self.config_edit.focus = self.config_edit.focus.next();
                    }
                    KeyCode::Enter if self.tab == DashboardTab::Config && self.input_mode == InputMode::Normal => {
                        self.config_edit.focus = self.config_edit.focus.next();
                    }
                    KeyCode::Char('i') | KeyCode::Char('a') if self.tab == DashboardTab::Config && self.input_mode == InputMode::Normal => {
                        self.input_mode = InputMode::Insert;
                    }
                    KeyCode::Esc if self.tab == DashboardTab::Config && self.input_mode == InputMode::Insert => {
                        self.input_mode = InputMode::Normal;
                    }
                    KeyCode::Backspace if self.tab == DashboardTab::Config && self.input_mode == InputMode::Insert => {
                        self.config_edit.focused_mut().pop();
                    }
                    KeyCode::Char(c) if self.tab == DashboardTab::Config && self.input_mode == InputMode::Insert => {
                        self.config_edit.focused_mut().push(c);
                    }
                    KeyCode::F(2) if self.tab == DashboardTab::Config => {
                        self.save_config(tx.clone());
                    }
                    // ── Other tabs ────────────────────────────────────────────
                    KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
                    KeyCode::Down | KeyCode::Char('j') => self.select_next(),
                    KeyCode::Home | KeyCode::Char('g') => self.select_first(),
                    KeyCode::End | KeyCode::Char('G') => self.select_last(),
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.select_half_page_down();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.select_half_page_up();
                    }
                    KeyCode::Char('/') => {
                        self.overlay = Some(Overlay::Search { query: self.search.clone() });
                    }
                    KeyCode::Char('c') if self.tab == DashboardTab::Functions => {
                        if let Some(f) = self.selected_function() {
                            let text = format!("{}{}", self.client.function_url, f.route);
                            copy_to_clipboard(&text);
                            self.toast = Some(Notification::new(format!("Copied: {text}")));
                        }
                    }
                    KeyCode::Char('c') if self.tab == DashboardTab::Keys => {
                        if let Some(k) = self.selected_key() {
                            let id = k.id.clone();
                            copy_to_clipboard(&id);
                            self.toast = Some(Notification::new(format!("Copied key ID: {id}")));
                        }
                    }
                    KeyCode::Char('i') if self.tab == DashboardTab::Functions => {
                        if let Some(f) = self.selected_function().cloned() {
                            self.loading = true;
                            let client = self.client.clone();
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                let result = match timeout(REFRESH_TIMEOUT, client.invoke_function(&f.route)).await {
                                    Ok(Ok(body)) => BgResult::Invoked(body),
                                    Ok(Err(e)) => BgResult::ActionFailed(format!("Invoke error: {e}")),
                                    Err(_) => BgResult::ActionFailed("Invoke timed out.".into()),
                                };
                                let _ = tx2.send(result).await;
                            });
                        } else {
                            self.toast = Some(Notification::new("No function selected."));
                        }
                    }
                    KeyCode::Char('d') if self.tab == DashboardTab::Functions => {
                        if let Some(f) = self.selected_function().cloned() {
                            self.overlay = Some(Overlay::Confirm {
                                message: format!("Delete function '{}'?", f.id),
                                action: ConfirmAction::DeleteFunction(f),
                            });
                        } else {
                            self.toast = Some(Notification::new("No function selected."));
                        }
                    }
                    KeyCode::Char('x') if self.tab == DashboardTab::Keys => {
                        if let Some(k) = self.selected_key().cloned() {
                            self.overlay = Some(Overlay::Confirm {
                                message: format!("Revoke key '{}'?", k.name),
                                action: ConfirmAction::RevokeKey(k),
                            });
                        } else {
                            self.toast = Some(Notification::new("No key selected."));
                        }
                    }
                    KeyCode::Char('n') if self.tab == DashboardTab::Keys => {
                        self.overlay = Some(Overlay::CreateKey { input: String::new() });
                    }
                    KeyCode::Char('D') if self.tab == DashboardTab::Functions => {
                        self.overlay = Some(Overlay::Deploy {
                            id: String::new(),
                            route: String::new(),
                            subdomain: String::new(),
                            wasm_path: String::new(),
                            focus: DeployField::Id,
                        });
                    }
                    _ => {}
                }
            }

            tokio::select! {
                Some(result) = rx.recv() => {
                    self.loading = false;
                    self.apply_bg_result(result);
                }
                _ = tokio::time::sleep(POLL_INTERVAL) => {
                    if self.should_auto_refresh() && !self.loading {
                        self.spawn_refresh(tx.clone());
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_overlay_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        tx: mpsc::Sender<BgResult>,
    ) {
        match &self.overlay {
            Some(Overlay::Confirm { .. }) => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let Some(Overlay::Confirm { action, .. }) = self.overlay.take() else {
                        return;
                    };
                    self.loading = true;
                    let client = self.client.clone();
                    tokio::spawn(async move {
                        let result = match action {
                            ConfirmAction::DeleteFunction(f) => {
                                match timeout(REFRESH_TIMEOUT, client.delete_function(&f.id)).await
                                {
                                    Ok(Ok(())) => {
                                        BgResult::ActionDone(format!("Deleted '{}'.", f.id))
                                    }
                                    Ok(Err(e)) => BgResult::ActionFailed(format!("Error: {e}")),
                                    Err(_) => BgResult::ActionFailed("Request timed out.".into()),
                                }
                            }
                            ConfirmAction::RevokeKey(k) => {
                                match timeout(REFRESH_TIMEOUT, client.revoke_key(&k.id)).await {
                                    Ok(Ok(())) => {
                                        BgResult::ActionDone(format!("Revoked '{}'.", k.name))
                                    }
                                    Ok(Err(e)) => BgResult::ActionFailed(format!("Error: {e}")),
                                    Err(_) => BgResult::ActionFailed("Request timed out.".into()),
                                }
                            }
                        };
                        let _ = tx.send(result).await;
                        let _ = tx.send(spawn_refresh_task(client).await).await;
                    });
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.overlay = None;
                    self.toast = Some(Notification::new("Cancelled."));
                }
                _ => {}
            },

            Some(Overlay::CreateKey { .. }) => match code {
                KeyCode::Esc if self.input_mode == InputMode::Insert => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.overlay = None;
                    self.toast = Some(Notification::new("Cancelled."));
                }
                KeyCode::Char('i') | KeyCode::Char('a') if self.input_mode == InputMode::Normal => {
                    self.input_mode = InputMode::Insert;
                }
                KeyCode::Enter => {
                    let Some(Overlay::CreateKey { input }) = self.overlay.take() else {
                        return;
                    };
                    self.input_mode = InputMode::Normal;
                    let name = input.trim().to_string();
                    if name.is_empty() {
                        self.toast = Some(Notification::new("Name cannot be empty."));
                        return;
                    }
                    self.loading = true;
                    let client = self.client.clone();
                    tokio::spawn(async move {
                        let result = match timeout(REFRESH_TIMEOUT, client.create_key(&name)).await
                        {
                            Ok(Ok(created)) => BgResult::ActionDone(format!(
                                "Created '{}'. Key: {}",
                                name, created.key
                            )),
                            Ok(Err(e)) => BgResult::ActionFailed(format!("Error: {e}")),
                            Err(_) => BgResult::ActionFailed("Request timed out.".into()),
                        };
                        let _ = tx.send(result).await;
                        let _ = tx.send(spawn_refresh_task(client).await).await;
                    });
                }
                _ => {
                    if let Some(Overlay::CreateKey { input }) = &mut self.overlay {
                        handle_text_input(code, input, self.input_mode);
                    }
                }
            },

            Some(Overlay::Search { .. }) => match code {
                KeyCode::Esc if self.input_mode == InputMode::Insert => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc | KeyCode::Enter => {
                    let Some(Overlay::Search { query }) = self.overlay.take() else {
                        return;
                    };
                    self.input_mode = InputMode::Normal;
                    self.search = query;
                    self.clamp_selection();
                }
                KeyCode::Char('i') | KeyCode::Char('a') if self.input_mode == InputMode::Normal => {
                    self.input_mode = InputMode::Insert;
                }
                _ => {
                    if let Some(Overlay::Search { query }) = &mut self.overlay {
                        handle_text_input(code, query, self.input_mode);
                    }
                }
            },

            Some(Overlay::Deploy { .. }) => match code {
                KeyCode::Esc if self.input_mode == InputMode::Insert => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.overlay = None;
                    self.toast = Some(Notification::new("Cancelled."));
                }
                KeyCode::Char('i') | KeyCode::Char('a') if self.input_mode == InputMode::Normal => {
                    self.input_mode = InputMode::Insert;
                }
                KeyCode::Tab if self.input_mode == InputMode::Normal => {
                    if let Some(Overlay::Deploy { focus, .. }) = &mut self.overlay {
                        *focus = focus.next();
                    }
                }
                KeyCode::BackTab if self.input_mode == InputMode::Normal => {
                    if let Some(Overlay::Deploy { focus, .. }) = &mut self.overlay {
                        *focus = focus.prev();
                    }
                }
                KeyCode::Enter => {
                    let Some(Overlay::Deploy {
                        id,
                        route,
                        subdomain,
                        wasm_path,
                        ..
                    }) = self.overlay.take()
                    else {
                        return;
                    };
                    self.input_mode = InputMode::Normal;
                    let id = id.trim().to_string();
                    let wasm_path = wasm_path.trim().to_string();
                    if id.is_empty() || wasm_path.is_empty() {
                        self.toast = Some(Notification::new("ID and WASM path are required."));
                        return;
                    }
                    let route = if route.trim().is_empty() {
                        None
                    } else {
                        Some(route.trim().to_string())
                    };
                    let subdomain = if subdomain.trim().is_empty() {
                        None
                    } else {
                        Some(subdomain.trim().to_string())
                    };
                    self.loading = true;
                    let client = self.client.clone();
                    tokio::spawn(async move {
                        let path = std::path::PathBuf::from(&wasm_path);
                        let result = match timeout(
                            REFRESH_TIMEOUT,
                            client.deploy(&id, route.as_deref(), subdomain.as_deref(), &path),
                        )
                        .await
                        {
                            Ok(Ok(f)) => {
                                BgResult::ActionDone(format!("Deployed '{}' at {}.", f.id, f.route))
                            }
                            Ok(Err(e)) => BgResult::ActionFailed(format!("Deploy failed: {e}")),
                            Err(_) => BgResult::ActionFailed("Deploy timed out.".into()),
                        };
                        let _ = tx.send(result).await;
                        let _ = tx.send(spawn_refresh_task(client).await).await;
                    });
                }
                _ => {
                    if let Some(Overlay::Deploy {
                        id,
                        route,
                        subdomain,
                        wasm_path,
                        focus,
                    }) = &mut self.overlay
                    {
                        let field = match focus {
                            DeployField::Id => id,
                            DeployField::Route => route,
                            DeployField::Subdomain => subdomain,
                            DeployField::WasmPath => wasm_path,
                        };
                        handle_text_input(code, field, self.input_mode);
                    }
                }
            },

            Some(Overlay::InvokeResult { .. }) => match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.overlay = None;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(Overlay::InvokeResult { scroll, .. }) = &mut self.overlay {
                        *scroll = scroll.saturating_add(1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(Overlay::InvokeResult { scroll, .. }) = &mut self.overlay {
                        *scroll = scroll.saturating_sub(1);
                    }
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::NONE) => {
                    if let Some(Overlay::InvokeResult { body, .. }) = &self.overlay {
                        let text = body.clone();
                        copy_to_clipboard(&text);
                        self.toast = Some(Notification::new("Response copied."));
                    }
                }
                _ => {}
            },

            None => {}
        }
    }

    fn spawn_refresh(&mut self, tx: mpsc::Sender<BgResult>) {
        self.loading = true;
        self.last_refresh = Some(Instant::now());
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = tx.send(spawn_refresh_task(client).await).await;
        });
    }

    fn apply_bg_result(&mut self, result: BgResult) {
        match result {
            BgResult::Refreshed { functions, keys } => {
                self.functions = functions;
                self.keys = keys;
                self.clamp_selection();
                self.last_refresh = Some(Instant::now());
                self.connected = true;
            }
            BgResult::RefreshFailed(msg) => {
                self.connected = false;
                self.toast = Some(Notification::new(msg));
            }
            BgResult::ActionDone(msg) => {
                self.toast = Some(Notification::new(msg));
            }
            BgResult::ActionFailed(msg) => {
                self.toast = Some(Notification::new(msg));
            }
            BgResult::Invoked(body) => {
                self.loading = false;
                self.overlay = Some(Overlay::InvokeResult { body, scroll: 0 });
            }
        }
    }

    /// Save config to disk, rebuild the client, and trigger a refresh.
    fn save_config(&mut self, tx: mpsc::Sender<BgResult>) {
        let cfg = crate::config::RuneConfig {
            server_url: Some(self.config_edit.control_plane.trim().to_string()),
            function_url: Some(self.config_edit.function_url.trim().to_string()),
            api_key: Some(self.config_edit.api_key.trim().to_string()),
        };
        if let Err(e) = cfg.save() {
            self.toast = Some(Notification::new(format!("Save failed: {e}")));
            return;
        }
        match RuneClient::with_function_url(
            cfg.server_url.as_deref().unwrap_or(""),
            cfg.function_url.as_deref().unwrap_or(""),
            cfg.api_key.as_deref().unwrap_or(""),
        ) {
            Ok(new_client) => {
                self.client = new_client;
                self.toast = Some(Notification::new("Config saved."));
                self.spawn_refresh(tx);
            }
            Err(e) => self.toast = Some(Notification::new(format!("Invalid config: {e}"))),
        }
    }

    fn next_tab(&mut self) {
        self.tab = self.tab.next();
    }
    fn previous_tab(&mut self) {
        self.tab = self.tab.previous();
    }

    fn select_next(&mut self) {
        let len = self.active_filtered_len();
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
        let len = self.active_filtered_len();
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
        if self.active_filtered_len() == 0 {
            self.active_state_mut().select(None);
        } else {
            self.active_state_mut().select(Some(0));
        }
    }

    fn select_last(&mut self) {
        let len = self.active_filtered_len();
        if len == 0 {
            self.active_state_mut().select(None);
        } else {
            self.active_state_mut().select(Some(len - 1));
        }
    }

    fn select_half_page_down(&mut self) {
        let len = self.active_filtered_len();
        if len == 0 {
            return;
        }
        let step = (len / 2).max(1);
        let next = match self.active_state().selected() {
            Some(i) => (i + step).min(len - 1),
            None => 0,
        };
        self.active_state_mut().select(Some(next));
    }

    fn select_half_page_up(&mut self) {
        let step = (self.active_filtered_len() / 2).max(1);
        let prev = match self.active_state().selected() {
            Some(i) => i.saturating_sub(step),
            None => 0,
        };
        self.active_state_mut().select(Some(prev));
    }

    fn clamp_selection(&mut self) {
        let fn_len = self.filtered_functions().len();
        let key_len = self.filtered_keys().len();
        clamp_list_state(&mut self.function_state, fn_len);
        clamp_list_state(&mut self.key_state, key_len);
    }

    fn should_auto_refresh(&self) -> bool {
        self.last_refresh
            .is_none_or(|last| last.elapsed() >= AUTO_REFRESH_INTERVAL)
    }

    /// Filtered function list based on current search query.
    pub(super) fn filtered_functions(&self) -> Vec<FunctionRecord> {
        if self.search.is_empty() {
            self.functions.clone()
        } else {
            let q = self.search.to_lowercase();
            self.functions
                .iter()
                .filter(|f| f.id.to_lowercase().contains(&q) || f.route.to_lowercase().contains(&q))
                .cloned()
                .collect()
        }
    }

    /// Filtered key list based on current search query.
    pub(super) fn filtered_keys(&self) -> Vec<KeyRecord> {
        if self.search.is_empty() {
            self.keys.clone()
        } else {
            let q = self.search.to_lowercase();
            self.keys
                .iter()
                .filter(|k| k.name.to_lowercase().contains(&q))
                .cloned()
                .collect()
        }
    }

    pub(super) fn selected_function(&self) -> Option<&FunctionRecord> {
        self.function_state
            .selected()
            .and_then(|i| self.functions.get(i))
    }

    pub(super) fn selected_key(&self) -> Option<&KeyRecord> {
        self.key_state.selected().and_then(|i| self.keys.get(i))
    }

    fn active_filtered_len(&self) -> usize {
        match self.tab {
            DashboardTab::Functions => self.filtered_functions().len(),
            DashboardTab::Keys => self.filtered_keys().len(),
            DashboardTab::Config => 0,
        }
    }

    fn active_state(&self) -> &ListState {
        match self.tab {
            DashboardTab::Functions => &self.function_state,
            DashboardTab::Keys | DashboardTab::Config => &self.key_state,
        }
    }

    fn active_state_mut(&mut self) -> &mut ListState {
        match self.tab {
            DashboardTab::Functions => &mut self.function_state,
            DashboardTab::Keys | DashboardTab::Config => &mut self.key_state,
        }
    }

    pub(super) fn toast_text(&self) -> &str {
        self.toast.as_ref().and_then(|n| n.live()).unwrap_or("")
    }
}

async fn spawn_refresh_task(client: RuneClient) -> BgResult {
    match timeout(REFRESH_TIMEOUT, async {
        tokio::join!(client.list_functions(), client.list_keys())
    })
    .await
    {
        Ok((Ok(functions), Ok(keys))) => BgResult::Refreshed { functions, keys },
        Ok((Err(e), _)) => BgResult::RefreshFailed(format!("Failed to load functions: {e}")),
        Ok((_, Err(e))) => BgResult::RefreshFailed(format!("Failed to load keys: {e}")),
        Err(_) => BgResult::RefreshFailed("Control plane unavailable.".into()),
    }
}

/// Apply a single keystroke to a text buffer. Handles `Backspace` and printable chars.
/// Returns `true` if the key was consumed.
pub(super) fn handle_text_input(code: KeyCode, buf: &mut String, mode: InputMode) -> bool {
    if mode != InputMode::Insert { return false; }
    match code {
        KeyCode::Backspace => { buf.pop(); true }
        KeyCode::Char(c) => { buf.push(c); true }
        _ => false,
    }
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DashboardTab {
    Functions,
    Keys,
    Config,
}

impl DashboardTab {
    fn next(self) -> Self {
        match self {
            Self::Functions => Self::Keys,
            Self::Keys => Self::Config,
            Self::Config => Self::Functions,
        }
    }
    fn previous(self) -> Self {
        match self {
            Self::Functions => Self::Config,
            Self::Keys => Self::Functions,
            Self::Config => Self::Keys,
        }
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
