use std::{
    io::{self},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tokio::sync::oneshot;

use crate::client::RuneClient;

/// Spinner frames — cycles through these while waiting.
const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const FRAME_MS: Duration = Duration::from_millis(80);
/// How long to wait for the server before giving up.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Outcome of the splash check.
pub enum SplashResult {
    Ready,
    Offline(String),
}

/// Show a spinner while probing the server. Returns `Ready` or `Offline(reason)`.
/// The caller is responsible for entering/leaving the alternate screen.
pub async fn check(client: &RuneClient) -> anyhow::Result<SplashResult> {
    // Spawn the health probe in the background.
    let (tx, rx) = oneshot::channel::<Result<(), String>>();
    let c = client.clone();
    tokio::spawn(async move {
        let result = tokio::time::timeout(CONNECT_TIMEOUT, c.list_functions())
            .await
            .map(|r| r.map(|_| ()).map_err(|e| e.to_string()))
            .unwrap_or_else(|_| Err("Server did not respond in time.".into()));
        let _ = tx.send(result);
    });

    // Enter TUI for the splash screen.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let started = Instant::now();
    let mut frame_idx: usize = 0;
    let mut rx = rx;

    let result = loop {
        // Check if the probe finished.
        match rx.try_recv() {
            Ok(Ok(())) => break SplashResult::Ready,
            Ok(Err(msg)) => break SplashResult::Offline(msg),
            Err(oneshot::error::TryRecvError::Closed) => {
                break SplashResult::Offline("Connection closed unexpectedly.".into());
            }
            Err(oneshot::error::TryRecvError::Empty) => {}
        }

        // Hard timeout guard (shouldn't be needed, but belt-and-suspenders).
        if started.elapsed() > CONNECT_TIMEOUT + Duration::from_secs(1) {
            break SplashResult::Offline("Timed out connecting to server.".into());
        }

        terminal.draw(|f| render_splash(f, SPINNER[frame_idx % SPINNER.len()]))?;
        frame_idx += 1;

        // Allow quitting during the splash with 'q' or Ctrl-C.
        if event::poll(FRAME_MS)?
            && let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
            && matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            break SplashResult::Offline("Aborted.".into());
        }
    };

    // If offline, show the error for a moment before handing back.
    if let SplashResult::Offline(ref msg) = result {
        let msg = msg.clone();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            terminal.draw(|f| render_error(f, &msg))?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(result)
}

fn render_splash(frame: &mut Frame<'_>, spinner: &str) {
    let area = frame.area();
    let mid = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area)[1];

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("  {} Connecting to rune server…", spinner),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                "  Press q to abort",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        mid,
    );
}

fn render_error(frame: &mut Frame<'_>, msg: &str) {
    let area = frame.area();
    let mid = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area)[1];

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("  ✗ {}", msg),
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                "  Is rune-server running?",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        mid,
    );
}
