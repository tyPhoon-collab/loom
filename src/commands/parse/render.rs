use super::model::{midi_note_name, ParsedEvent, ParsedEventTableRow};
use crate::cli::ParseFormat;
use miette::{IntoDiagnostic, Result};
use std::collections::BTreeSet;
use tabled::Table;

pub(super) fn print_events(events: &[ParsedEvent], format: ParseFormat) -> Result<()> {
    match format {
        ParseFormat::Table => {
            let rows: Vec<ParsedEventTableRow> =
                events.iter().map(ParsedEventTableRow::from).collect();
            println!("{}", Table::new(rows));
        }
        ParseFormat::Json => {
            let json = serde_json::to_string_pretty(events).into_diagnostic()?;
            println!("{}", json);
        }
        ParseFormat::Csv => {
            println!("event_type,track,channel,note,note_name,velocity,cc,value,program,time,duration,end_time");
            for e in events {
                println!(
                    "{},{},{},{},{},{},{},{},{},{:.6},{},{}",
                    csv_escape(&e.event_type),
                    csv_escape(&e.track),
                    e.channel,
                    e.note.map_or_else(String::new, |v| v.to_string()),
                    csv_escape(e.note_name.as_deref().unwrap_or("")),
                    e.velocity.map_or_else(String::new, |v| v.to_string()),
                    e.cc.map_or_else(String::new, |v| v.to_string()),
                    e.value.map_or_else(String::new, |v| v.to_string()),
                    e.program.map_or_else(String::new, |v| v.to_string()),
                    e.time,
                    e.duration.map(|v| format!("{:.6}", v)).unwrap_or_default(),
                    e.end_time.map(|v| format!("{:.6}", v)).unwrap_or_default()
                );
            }
        }
    }

    Ok(())
}

pub(super) fn print_summary(events: &[ParsedEvent], format: ParseFormat) {
    let emit = |line: String| match format {
        ParseFormat::Table => println!("{}", line),
        ParseFormat::Json | ParseFormat::Csv => eprintln!("{}", line),
    };

    if events.is_empty() {
        emit("Summary: events=0".to_string());
        return;
    }

    let mut tracks = BTreeSet::new();
    let mut channels = BTreeSet::new();
    let mut note_min = u8::MAX;
    let mut note_max = u8::MIN;
    let mut vel_min = u8::MAX;
    let mut vel_max = u8::MIN;
    let mut vel_sum: u64 = 0;
    let mut t_min = f64::MAX;
    let mut t_max = f64::MIN;
    let mut end_max = f64::MIN;

    for e in events {
        tracks.insert(e.track.clone());
        channels.insert(e.channel);
        if let Some(note) = e.note {
            note_min = note_min.min(note);
            note_max = note_max.max(note);
        }
        if let Some(velocity) = e.velocity {
            vel_min = vel_min.min(velocity);
            vel_max = vel_max.max(velocity);
            vel_sum += velocity as u64;
        }
        t_min = t_min.min(e.time);
        t_max = t_max.max(e.time);
        end_max = end_max.max(e.end_time.unwrap_or(e.time));
    }

    let note_count = events.iter().filter(|e| e.note.is_some()).count();
    let vel_count = events.iter().filter(|e| e.velocity.is_some()).count();
    let vel_avg = if vel_count > 0 {
        vel_sum as f64 / vel_count as f64
    } else {
        0.0
    };
    emit(format!(
        "Summary: events={}, tracks={}, channels={:?}",
        events.len(),
        tracks.len(),
        channels
    ));
    emit(format!(
        "  time: start={:.2}, last_on={:.2}, end={:.2}",
        t_min, t_max, end_max
    ));
    if note_count > 0 {
        emit(format!(
            "  note_range: {}({}) .. {}({})",
            note_min,
            midi_note_name(note_min),
            note_max,
            midi_note_name(note_max)
        ));
    }
    if vel_count > 0 {
        emit(format!(
            "  velocity: min={}, max={}, avg={:.2}",
            vel_min, vel_max, vel_avg
        ));
    }
}

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}
