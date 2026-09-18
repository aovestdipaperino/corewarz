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
    /// on error.
    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        let terminal_result = Terminal::new(CrosstermBackend::new(io::stdout()));
        let result = match terminal_result {
            Ok(mut terminal) => self.event_loop(&mut terminal),
            Err(e) => Err(e),
        };
        // The terminal must be restored on every path, including an error
        // creating it or an error during the event loop.
        let disable_result = disable_raw_mode();
        let leave_result = io::stdout().execute(LeaveAlternateScreen).map(|_| ());
        result.and(disable_result).and(leave_result)
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
