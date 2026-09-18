//! The terminal application: event loop, speed control, and stepping.

use std::io;
use std::time::{Duration, Instant};

use corewarz_core::{Mars, Outcome};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::render::render;
use crate::state::VisualState;

/// Cycles executed per second at each speed setting.
const SPEEDS: [u32; 7] = [1, 5, 25, 100, 500, 2000, 10_000];

/// RAII guard for terminal state restoration.
///
/// This guard ensures that the terminal is restored to its original state
/// even if a panic occurs during rendering or event processing. Without this,
/// a panic would unwind past the restoration code, leaving the terminal in raw
/// mode and on the alternate screen, rendering the user's shell unusable.
struct TerminalGuard;

impl TerminalGuard {
    /// Enables raw mode and enters the alternate screen.
    ///
    /// # Errors
    /// Returns an error if either `enable_raw_mode()` or entering the alternate
    /// screen fails.
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Disable raw mode and leave the alternate screen. These calls should
        // not panic, and if they fail, there is nothing useful we can do during
        // unwinding, so we ignore errors.
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
    }
}

/// A running visualizer.
pub struct App {
    mars: Mars,
    state: VisualState,
    speed: usize,
    paused: bool,
    outcome: Option<Outcome>,
}

impl App {
    /// Creates an application driving `mars`, starting paused.
    #[must_use]
    pub fn new(mars: Mars) -> Self {
        let state = VisualState::new(mars.core().size());
        Self {
            mars,
            state,
            speed: 3,
            paused: true,
            outcome: None,
        }
    }

    /// Runs the event loop until the user quits.
    ///
    /// # Errors
    /// Returns any terminal I/O error. The terminal is always restored, even
    /// on error or panic.
    pub fn run(&mut self) -> io::Result<()> {
        let _guard = TerminalGuard::new()?;
        let terminal_result = Terminal::new(CrosstermBackend::new(io::stdout()));
        match terminal_result {
            Ok(mut terminal) => self.event_loop(&mut terminal),
            Err(e) => Err(e),
        }
    }

    fn event_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        let frame_time = Duration::from_millis(33);
        loop {
            let started = Instant::now();
            terminal.draw(|frame| render(frame, &self.mars, &self.state))?;
            self.state.decay();

            if self.handle_input(frame_time.saturating_sub(started.elapsed()))? {
                return Ok(());
            }

            if !self.paused && self.outcome.is_none() {
                let budget = (SPEEDS[self.speed] / 30).max(1);
                for _ in 0..budget {
                    if let Some(outcome) = self.mars.outcome() {
                        self.outcome = Some(outcome);
                        self.paused = true;
                        break;
                    }
                    self.mars.step(&mut self.state);
                }
            }
        }
    }

    /// Handles pending input. Returns `true` when the user has asked to quit.
    fn handle_input(&mut self, timeout: Duration) -> io::Result<bool> {
        if !event::poll(timeout)? {
            return Ok(false);
        }
        let Event::Key(key) = event::read()? else {
            return Ok(false);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Char('s') => {
                if self.mars.outcome().is_none() {
                    self.mars.step(&mut self.state);
                }
                self.paused = true;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.speed = (self.speed + 1).min(SPEEDS.len() - 1);
            }
            KeyCode::Char('-') => self.speed = self.speed.saturating_sub(1),
            _ => {}
        }
        Ok(false)
    }
}
