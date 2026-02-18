use crate::compiler;
use crate::dsl::parser;
use crate::event::{Event, EventHandler};
use crate::live_player::LivePlayer;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub struct App {
    pub should_quit: bool,
    pub path: PathBuf,
    pub status_message: String,
    pub is_playing: bool,
    pub bpm: u32,
    player: LivePlayer,
    event_handler: EventHandler,
}

impl App {
    pub fn new(path: PathBuf, port_index: usize) -> Result<Self> {
        let player = LivePlayer::new(port_index)?;
        // Initial compile
        let content = fs::read_to_string(&path).unwrap_or_default();
        let _ = Self::compile_and_update(&content, &player);

        Ok(Self {
            should_quit: false,
            event_handler: EventHandler::new(path.clone(), Duration::from_millis(250))?,
            path,
            status_message: "Ready".to_string(),
            is_playing: false,
            bpm: 120,
            player,
        })
    }

    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> Result<()> {
        self.player.play();
        self.is_playing = true;

        loop {
            // map_err because B::Error might not satisfy IntoDiagnostic bounds (Send+Sync)
            terminal
                .draw(|f| self.ui(f))
                .map_err(|e| miette::miette!("Draw error: {:?}", e))?;

            match self.event_handler.next()? {
                Event::Key(key) => {
                    self.handle_key(key);
                }
                Event::FileChange => {
                    // Hot-swap
                    self.status_message = "File changed, recompiling...".to_string();

                    let content = fs::read_to_string(&self.path).unwrap_or_default();

                    match Self::compile_and_update(&content, &self.player) {
                        Ok((bpm, msg)) => {
                            self.bpm = bpm;
                            self.status_message = format!("Reloaded! {}", msg);

                            // Auto-format if valid
                            let formatted = crate::dsl::formatter::format_string(&content);
                            if content != formatted {
                                if let Err(e) = fs::write(&self.path, &formatted) {
                                    self.status_message = format!("Format save error: {}", e);
                                } else {
                                    // Re-compile to get events for MIDI export
                                    if let Ok(song) = parser::parse_song(content.clone()) {
                                        if let Ok(compiler_inst) =
                                            crate::compiler::Compiler::new(&song)
                                        {
                                            if let Ok(events) = compiler_inst.compile(&song) {
                                                if let Err(e) = crate::midi::file::save_to_midi(
                                                    &events,
                                                    &self.path.with_extension("mid"),
                                                    bpm,
                                                ) {
                                                    self.status_message =
                                                        format!("MIDI save error: {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Compilation failed! {}", e);
                        }
                    }
                }
                Event::Tick => {}
            }

            if self.should_quit {
                break;
            }
        }
        self.player.stop();
        Ok(())
    }

    fn compile_and_update(content: &str, player: &LivePlayer) -> Result<(u32, String)> {
        match parser::parse_song(content.to_string()) {
            Ok(song) => match compiler::Compiler::new(&song) {
                Ok(compiler_inst) => match compiler_inst.compile(&song) {
                    Ok(events) => {
                        let events: Vec<crate::compiler::MidiEvent> = events.to_vec();
                        let bpm = song.metadata.bpm;
                        let msg = format!("{} events, {} BPM", events.len(), bpm);
                        player.update(events, song.metadata);
                        Ok((bpm, msg))
                    }
                    Err(e) => Err(miette::miette!("Compile error: {}", e)),
                },
                Err(e) => Err(miette::miette!("Compiler Init error: {}", e)),
            },
            Err(e) => Err(miette::miette!("Parse error: {}", e)),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char(' ') => {
                if self.is_playing {
                    self.player.pause();
                    self.is_playing = false;
                    self.status_message = "Paused".to_string();
                } else {
                    self.player.play();
                    self.is_playing = true;
                    self.status_message = "Playing".to_string();
                }
            }
            _ => {}
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Style},
            widgets::{Block, Borders, Paragraph},
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(f.area());

        let title = Paragraph::new(format!("Loom Live - {}", self.path.display()))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        let status = Paragraph::new(format!(
            "Status: {}\nBPM: {}\nState: {}",
            self.status_message,
            self.bpm,
            if self.is_playing { "PLAYING" } else { "PAUSED" }
        ))
        .block(Block::default().title("Info").borders(Borders::ALL))
        .style(
            if self.status_message.contains("error") || self.status_message.contains("failed") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            },
        );
        f.render_widget(status, chunks[1]);

        let footer = Paragraph::new("q: Quit | Space: Play/Pause")
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }
}
