use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MidiEvent {
    Note {
        time: f64,     // Absolute time in beats
        duration: f64, // Duration in beats
        channel: u8,   // 0-based
        note: u8,
        velocity: u8,
    },
    ControlChange {
        time: f64,
        channel: u8, // 0-based
        cc: u8,
        value: u8,
    },
    ProgramChange {
        time: f64,
        channel: u8, // 0-based
        program: u8,
    },
}

impl MidiEvent {
    pub fn time(&self) -> f64 {
        match self {
            Self::Note { time, .. }
            | Self::ControlChange { time, .. }
            | Self::ProgramChange { time, .. } => *time,
        }
    }

    pub fn channel(&self) -> u8 {
        match self {
            Self::Note { channel, .. }
            | Self::ControlChange { channel, .. }
            | Self::ProgramChange { channel, .. } => *channel,
        }
    }

    pub fn note_end_time(&self) -> Option<f64> {
        match self {
            Self::Note { time, duration, .. } => Some(*time + *duration),
            _ => None,
        }
    }

    pub fn timing_order(&self) -> u8 {
        match self {
            Self::ControlChange { cc: 0, .. } => 0,
            Self::ControlChange { cc: 32, .. } => 1,
            Self::ProgramChange { .. } => 2,
            Self::ControlChange { .. } => 3,
            Self::Note { .. } => 10,
        }
    }

    pub fn note(&self) -> Option<u8> {
        match self {
            Self::Note { note, .. } => Some(*note),
            _ => None,
        }
    }

    pub fn velocity(&self) -> Option<u8> {
        match self {
            Self::Note { velocity, .. } => Some(*velocity),
            _ => None,
        }
    }

    pub fn cc(&self) -> Option<u8> {
        match self {
            Self::ControlChange { cc, .. } => Some(*cc),
            _ => None,
        }
    }

    pub fn value(&self) -> Option<u8> {
        match self {
            Self::ControlChange { value, .. } => Some(*value),
            _ => None,
        }
    }

    pub fn program(&self) -> Option<u8> {
        match self {
            Self::ProgramChange { program, .. } => Some(*program),
            _ => None,
        }
    }
}
