use crate::compiler::MidiEvent;
use crate::dsl::token::Frontmatter;
use crate::sequencer::{Core, PlaybackState};
use miette::Result;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

static PLAYER_SENDER: OnceLock<Sender<PlayerCommand>> = OnceLock::new();

pub enum PlayerCommand {
    UpdateSequence(Vec<MidiEvent>, Frontmatter),
    Play,
    Pause,
    Stop,
    Restart,
    PreviewNote {
        channel: u8,
        note: u8,
        velocity: u8,
        duration: Duration,
    },
    PreviewNoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    PreviewProgramChange {
        channel: u8,
        program: u8,
    },
    PreviewControlChange {
        channel: u8,
        cc: u8,
        value: u8,
    },
    PreviewNoteOff {
        channel: u8,
        note: u8,
    },
    PreviewSilenceAll,
}

pub struct LivePlayer {
    command_sender: Sender<PlayerCommand>,
    thread_handle: Option<JoinHandle<Result<()>>>,
    playback_state: Arc<Mutex<PlaybackState>>,
}

impl LivePlayer {
    pub fn new(port_index: usize, current_beat_ref: Arc<Mutex<f64>>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let _ = PLAYER_SENDER.set(tx.clone());
        let playback_state = Arc::new(Mutex::new(PlaybackState::Stopped));
        let playback_state_ref = Arc::clone(&playback_state);

        let handle = thread::spawn(move || {
            if let Err(e) = run_player_loop(port_index, rx, current_beat_ref, playback_state_ref) {
                eprintln!("Player thread error: {}", e);
            }
            Ok(())
        });

        Ok(Self {
            command_sender: tx,
            thread_handle: Some(handle),
            playback_state,
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

    pub fn restart(&self) {
        let _ = self.command_sender.send(PlayerCommand::Restart);
    }

    pub fn preview_note(&self, channel: u8, note: u8, velocity: u8, duration: Duration) {
        let _ = self.command_sender.send(PlayerCommand::PreviewNote {
            channel,
            note,
            velocity,
            duration,
        });
    }

    pub fn preview_note_on(&self, channel: u8, note: u8, velocity: u8) {
        let _ = self.command_sender.send(PlayerCommand::PreviewNoteOn {
            channel,
            note,
            velocity,
        });
    }

    pub fn preview_program_change(&self, channel: u8, program: u8) {
        let _ = self
            .command_sender
            .send(PlayerCommand::PreviewProgramChange { channel, program });
    }

    pub fn preview_control_change(&self, channel: u8, cc: u8, value: u8) {
        let _ = self
            .command_sender
            .send(PlayerCommand::PreviewControlChange { channel, cc, value });
    }

    pub fn preview_note_off(&self, channel: u8, note: u8) {
        let _ = self
            .command_sender
            .send(PlayerCommand::PreviewNoteOff { channel, note });
    }

    pub fn preview_silence_all(&self) {
        let _ = self.command_sender.send(PlayerCommand::PreviewSilenceAll);
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state
            .lock()
            .map(|state| *state)
            .unwrap_or(PlaybackState::Stopped)
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

fn run_player_loop(
    port_index: usize,
    rx: Receiver<PlayerCommand>,
    current_beat_ref: Arc<Mutex<f64>>,
    playback_state_ref: Arc<Mutex<PlaybackState>>,
) -> Result<()> {
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
                    core.pause()?;
                }
                PlayerCommand::Restart => {
                    core.restart()?;
                }
                PlayerCommand::PreviewNote {
                    channel,
                    note,
                    velocity,
                    duration,
                } => {
                    core.preview_note(channel, note, velocity, duration)?;
                }
                PlayerCommand::PreviewNoteOn {
                    channel,
                    note,
                    velocity,
                } => {
                    core.preview_note_on(channel, note, velocity)?;
                }
                PlayerCommand::PreviewProgramChange { channel, program } => {
                    core.preview_program_change(channel, program)?;
                }
                PlayerCommand::PreviewControlChange { channel, cc, value } => {
                    core.preview_control_change(channel, cc, value)?;
                }
                PlayerCommand::PreviewNoteOff { channel, note } => {
                    core.preview_note_off(channel, note)?;
                }
                PlayerCommand::PreviewSilenceAll => {
                    core.silence_preview_notes()?;
                }
                PlayerCommand::Stop => {
                    core.stop()?;
                    publish_playback_state(&playback_state_ref, PlaybackState::Stopped);
                    return Ok(());
                }
            }
        }

        let state = core.tick()?;
        publish_playback_state(&playback_state_ref, state);
        if state == PlaybackState::Playing || state == PlaybackState::Paused {
            if let Ok(mut beat) = current_beat_ref.lock() {
                *beat = core.current_beat();
            }
        }

        thread::sleep(tick_rate);
    }
}

fn publish_playback_state(state_ref: &Arc<Mutex<PlaybackState>>, state: PlaybackState) {
    if let Ok(mut current) = state_ref.lock() {
        *current = state;
    }
}
