use std::io::{stdout, Stdout};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use miette::{IntoDiagnostic, Result};
use ratatui::prelude::*;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).into_diagnostic()?;
    enable_raw_mode().into_diagnostic()?;
    let backend = CrosstermBackend::new(stdout());
    Terminal::new(backend).into_diagnostic()
}

pub fn restore() -> Result<()> {
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).into_diagnostic()?;
    disable_raw_mode().into_diagnostic()?;
    Ok(())
}
