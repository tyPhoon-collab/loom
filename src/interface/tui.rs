use std::io::{stdout, Stdout};

use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use miette::{IntoDiagnostic, Result};
use ratatui::prelude::*;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new(terminal: Terminal<CrosstermBackend<Stdout>>) -> Self {
        Self { terminal }
    }
}

impl std::ops::Deref for Tui {
    type Target = Terminal<CrosstermBackend<Stdout>>;
    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl std::ops::DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = restore();
    }
}

pub fn init() -> Result<Tui> {
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).into_diagnostic()?;
    enable_raw_mode().into_diagnostic()?;
    execute!(
        stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
        )
    )
    .into_diagnostic()?;
    install_panic_hook();
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend).into_diagnostic()?;
    Ok(Tui::new(terminal))
}

fn restore() -> Result<()> {
    execute!(
        stdout(),
        PopKeyboardEnhancementFlags,
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .into_diagnostic()?;
    disable_raw_mode().into_diagnostic()?;
    Ok(())
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        crate::live_player::LivePlayer::global_stop();
        original(info);
    }));
}
