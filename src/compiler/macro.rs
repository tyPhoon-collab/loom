use crate::compiler::event::MidiEvent;
use crate::dsl::token::TemplateMacro;

/// Apply a post-processing macro to a slice of generated MidiEvents.
pub fn apply_macro(events: &mut [MidiEvent], macro_kind: &TemplateMacro, time_scale: f64) {
    match macro_kind {
        TemplateMacro::Vel(vel) => {
            for event in events.iter_mut() {
                if let MidiEvent::Note { velocity, .. } = event {
                    *velocity = *vel;
                }
            }
        }
        TemplateMacro::Arp => {
            // arp — Spread simultaneous notes evenly across their block duration
            apply_arp(events, time_scale);
        }
        TemplateMacro::Strum => {
            // strum — Small timing offsets between simultaneous notes (guitar-like)
            apply_strum(events, time_scale);
        }
        TemplateMacro::Rev | TemplateMacro::Pan(_) => {}
    }
}

/// Arpeggiate: spread simultaneous notes evenly across the block duration.
fn apply_arp(events: &mut [MidiEvent], _time_scale: f64) {
    if events.is_empty() {
        return;
    }

    // Group events by their start time
    let mut time_groups: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, event) in events.iter().enumerate() {
        if let MidiEvent::Note { time, .. } = event {
            let key = (*time * 1_000_000.0) as u64; // Use microsecond precision for grouping
            time_groups.entry(key).or_default().push(i);
        }
    }

    for indices in time_groups.values() {
        let count = indices.len();
        if count <= 1 {
            continue;
        }
        // Get the original duration of the block these notes belong to
        let original_duration = match events[indices[0]] {
            MidiEvent::Note { duration, .. } => duration,
            _ => continue,
        };
        let step = original_duration / count as f64;

        // Sort by pitch (low to high) for natural arpeggio
        let mut sorted_indices: Vec<usize> = indices.clone();
        sorted_indices.sort_by(|&a, &b| {
            let note_a = match events[a] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            let note_b = match events[b] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            note_a.cmp(&note_b)
        });

        for (nth, &idx) in sorted_indices.iter().enumerate() {
            if let MidiEvent::Note { time, duration, .. } = &mut events[idx] {
                *time += step * nth as f64;
                *duration = step;
            }
        }
    }
}

/// Strum: add small timing offsets between simultaneous notes (guitar-like feel).
fn apply_strum(events: &mut [MidiEvent], _time_scale: f64) {
    if events.is_empty() {
        return;
    }

    let strum_interval = 0.03; // ~30ms worth of beats at moderate tempo

    // Group events by their start time
    let mut time_groups: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, event) in events.iter().enumerate() {
        if let MidiEvent::Note { time, .. } = event {
            let key = (*time * 1_000_000.0) as u64;
            time_groups.entry(key).or_default().push(i);
        }
    }

    for indices in time_groups.values() {
        let count = indices.len();
        if count <= 1 {
            continue;
        }

        // Sort by pitch: low notes first (strum from bass string up)
        let mut sorted_indices: Vec<usize> = indices.clone();
        sorted_indices.sort_by(|&a, &b| {
            let note_a = match events[a] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            let note_b = match events[b] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            note_a.cmp(&note_b)
        });

        for (nth, &idx) in sorted_indices.iter().enumerate() {
            let offset = strum_interval * nth as f64;
            if let MidiEvent::Note { time, duration, .. } = &mut events[idx] {
                *time += offset;
                // Shorten duration slightly to keep the end time the same
                *duration = (*duration - offset).max(0.01);
            }
        }
    }
}
