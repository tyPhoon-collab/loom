use crate::compiler::MidiEvent;
use crate::dsl::token::Frontmatter;
use crate::sequencer::{Core, PlaybackState};
use miette::Result;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};
use std::time::Duration;

static PLAYER_SENDER: OnceLock<Sender<PlayerCommand>> = OnceLock::new();

pub enum PlayerCommand {
    UpdateSequence(Vec<MidiEvent>, Frontmatter),
    Play,
    Pause,
    Stop,
}

pub struct LivePlayer {
    command_sender: Sender<PlayerCommand>,
    thread_handle: Option<JoinHandle<Result<()>>>,
}

impl LivePlayer {
    pub fn new(port_index: usize) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let _ = PLAYER_SENDER.set(tx.clone());

        let handle = thread::spawn(move || {
            if let Err(e) = run_player_loop(port_index, rx) {
                eprintln!("Player thread error: {}", e);
            }
            Ok(())
        });

        Ok(Self {
            command_sender: tx,
            thread_handle: Some(handle),
        })
    }

    pub fn update(&self, events: Vec<MidiEvent>, metadata: Frontmatter) {
        let _ = self
            .command_sender
            .send(PlayerCommand::UpdateSequence(events, metadata));
    }

    pub fn play(&self) {
        let _ = self.command_sender.send(PlayerCommand::Play);
    }

    pub fn pause(&self) {
        let _ = self.command_sender.send(PlayerCommand::Pause);
    }

    pub fn stop(&mut self) {
        let _ = self.command_sender.send(PlayerCommand::Stop);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn global_stop() {
        if let Some(tx) = PLAYER_SENDER.get() {
            let _ = tx.send(PlayerCommand::Stop);
        }
    }
}

impl Drop for LivePlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_player_loop(port_index: usize, rx: Receiver<PlayerCommand>) -> Result<()> {
    let mut core = Core::new(port_index, "Loom Live")?;
    let tick_rate = Duration::from_millis(5);

    loop {
        // Handle Commands
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                PlayerCommand::UpdateSequence(events, metadata) => {
                    core.load(events, metadata);
                }
                PlayerCommand::Play => {
                    core.play();
                }
                PlayerCommand::Pause => {
                    core.pause();
                }
                PlayerCommand::Stop => {
                    core.stop();
                    return Ok(());
                }
            }
        }

        let state = core.tick();
        if state == PlaybackState::Playing {
            // Check loop or end?
            // Core handles looping if metadata.loop is true.
            // If not looping and end reached, Core stops.
        }

        thread::sleep(tick_rate);
    }
}
