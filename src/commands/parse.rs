use crate::cli::{ParseFormat, ParseSortKey};
use loom::dsl::parser;
use loom::{compiler, dsl};
use miette::{miette, IntoDiagnostic, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tabled::{Table, Tabled};

#[derive(Debug, Clone, Serialize)]
struct ParsedEvent {
    event_type: String,
    track: String,
    channel: u8, // 1-based
    note: Option<u8>,
    note_name: Option<String>,
    velocity: Option<u8>,
    cc: Option<u8>,
    value: Option<u8>,
    program: Option<u8>,
    time: f64,
    duration: Option<f64>,
    end_time: Option<f64>,
}

#[derive(Tabled)]
struct ParsedEventTableRow {
    #[tabled(rename = "Type")]
    event_type: String,
    #[tabled(rename = "Track")]
    track: String,
    #[tabled(rename = "CH")]
    channel: u8,
    #[tabled(rename = "Note")]
    note: String,
    #[tabled(rename = "Name")]
    note_name: String,
    #[tabled(rename = "Vel")]
    velocity: String,
    #[tabled(rename = "CC")]
    cc: String,
    #[tabled(rename = "Val")]
    value: String,
    #[tabled(rename = "PC")]
    program: String,
    #[tabled(rename = "Time")]
    time: String,
    #[tabled(rename = "Duration")]
    duration: String,
    #[tabled(rename = "End")]
    end_time: String,
}

impl From<&ParsedEvent> for ParsedEventTableRow {
    fn from(e: &ParsedEvent) -> Self {
        Self {
            event_type: e.event_type.clone(),
            track: e.track.clone(),
            channel: e.channel,
            note: e.note.map_or_else(String::new, |v| v.to_string()),
            note_name: e.note_name.clone().unwrap_or_default(),
            velocity: e.velocity.map_or_else(String::new, |v| v.to_string()),
            cc: e.cc.map_or_else(String::new, |v| v.to_string()),
            value: e.value.map_or_else(String::new, |v| v.to_string()),
            program: e.program.map_or_else(String::new, |v| v.to_string()),
            time: format!("{:.2}", e.time),
            duration: e.duration.map_or_else(String::new, |v| format!("{:.2}", v)),
            end_time: e.end_time.map_or_else(String::new, |v| format!("{:.2}", v)),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct ParseFilter {
    track: Option<String>,
    event_type: Option<String>,
    channel: Option<u8>, // 1-based
    note: Option<u8>,
    note_name: Option<String>,
    velocity_min: Option<u8>,
    velocity_max: Option<u8>,
    time_min: Option<f64>,
    time_max: Option<f64>,
}

pub fn handle_parse(
    input: PathBuf,
    format: ParseFormat,
    sort: ParseSortKey,
    filters: &[String],
    summary: bool,
) -> Result<()> {
    let content = fs::read_to_string(&input).into_diagnostic()?;
    let song = parser::parse_song(content)?;
    let mut events = collect_parsed_events(&song)?;
    let parsed_filter = parse_filters(filters)?;
    events = apply_filters(events, &parsed_filter);
    sort_events(&mut events, sort);
    print_events(&events, format)?;
    if summary {
        print_summary(&events, format);
    }
    Ok(())
}

fn midi_note_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let idx = (note % 12) as usize;
    let octave = (note / 12) as i8 - 1;
    format!("{}{}", NAMES[idx], octave)
}

fn parse_filters(inputs: &[String]) -> Result<ParseFilter> {
    let mut f = ParseFilter::default();

    for input in inputs {
        for pair in input.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| miette!("Invalid --filter '{}': expected key=value", pair))?;

            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();

            match key.as_str() {
                "track" => f.track = Some(value.to_string()),
                "type" | "event_type" => f.event_type = Some(value.to_ascii_lowercase()),
                "channel" => {
                    let ch = value
                        .parse::<u8>()
                        .map_err(|_| miette!("Invalid channel '{}'", value))?;
                    if !(1..=16).contains(&ch) {
                        return Err(miette!("Invalid channel '{}': must be 1..16", value));
                    }
                    f.channel = Some(ch);
                }
                "note" => {
                    let note = value
                        .parse::<u8>()
                        .map_err(|_| miette!("Invalid note '{}'", value))?;
                    if note > 127 {
                        return Err(miette!("Invalid note '{}': must be 0..127", value));
                    }
                    f.note = Some(note);
                }
                "note_name" => f.note_name = Some(value.to_ascii_uppercase()),
                "velocity" => {
                    let vel = value
                        .parse::<u8>()
                        .map_err(|_| miette!("Invalid velocity '{}'", value))?;
                    if vel > 127 {
                        return Err(miette!("Invalid velocity '{}': must be 0..127", value));
                    }
                    f.velocity_min = Some(vel);
                    f.velocity_max = Some(vel);
                }
                "velocity_min" => {
                    let vel = value
                        .parse::<u8>()
                        .map_err(|_| miette!("Invalid velocity_min '{}'", value))?;
                    if vel > 127 {
                        return Err(miette!("Invalid velocity_min '{}': must be 0..127", value));
                    }
                    f.velocity_min = Some(vel);
                }
                "velocity_max" => {
                    let vel = value
                        .parse::<u8>()
                        .map_err(|_| miette!("Invalid velocity_max '{}'", value))?;
                    if vel > 127 {
                        return Err(miette!("Invalid velocity_max '{}': must be 0..127", value));
                    }
                    f.velocity_max = Some(vel);
                }
                "time_min" => {
                    f.time_min = Some(
                        value
                            .parse::<f64>()
                            .map_err(|_| miette!("Invalid time_min '{}'", value))?,
                    );
                }
                "time_max" => {
                    f.time_max = Some(
                        value
                            .parse::<f64>()
                            .map_err(|_| miette!("Invalid time_max '{}'", value))?,
                    );
                }
                _ => {
                    return Err(miette!(
                        "Unknown filter key '{}'. Supported: type,event_type,track,channel,note,note_name,velocity,velocity_min,velocity_max,time_min,time_max",
                        key
                    ));
                }
            }
        }
    }

    Ok(f)
}

fn apply_filters(events: Vec<ParsedEvent>, f: &ParseFilter) -> Vec<ParsedEvent> {
    events
        .into_iter()
        .filter(|e| {
            f.track.as_ref().is_none_or(|v| &e.track == v)
                && f.event_type
                    .as_ref()
                    .is_none_or(|v| e.event_type.to_ascii_lowercase() == *v)
                && f.channel.is_none_or(|v| e.channel == v)
                && f.note.is_none_or(|v| e.note == Some(v))
                && f.note_name
                    .as_ref()
                    .is_none_or(|v| e.note_name.as_deref().unwrap_or("").to_ascii_uppercase() == *v)
                && f.velocity_min.is_none_or(|v| e.velocity.unwrap_or(0) >= v)
                && f.velocity_max.is_none_or(|v| e.velocity.unwrap_or(0) <= v)
                && f.time_min.is_none_or(|v| e.time >= v)
                && f.time_max.is_none_or(|v| e.time <= v)
        })
        .collect()
}

fn sort_events(events: &mut [ParsedEvent], key: ParseSortKey) {
    match key {
        ParseSortKey::Time => events.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap()
                .then_with(|| a.note.unwrap_or(0).cmp(&b.note.unwrap_or(0)))
                .then_with(|| a.channel.cmp(&b.channel))
        }),
        ParseSortKey::Note => events.sort_by(|a, b| {
            a.note
                .unwrap_or(0)
                .cmp(&b.note.unwrap_or(0))
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Channel => events.sort_by(|a, b| {
            a.channel
                .cmp(&b.channel)
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Velocity => events.sort_by(|a, b| {
            a.velocity
                .unwrap_or(0)
                .cmp(&b.velocity.unwrap_or(0))
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Duration => events.sort_by(|a, b| {
            a.duration
                .unwrap_or(0.0)
                .partial_cmp(&b.duration.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Track => events.sort_by(|a, b| {
            a.track
                .cmp(&b.track)
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
    }
}

fn collect_parsed_events(song: &dsl::token::Song) -> Result<Vec<ParsedEvent>> {
    let mut out = Vec::new();

    for track in &song.tracks {
        if track.muted {
            continue;
        }
        let single_song = dsl::token::Song {
            metadata: song.metadata.clone(),
            tracks: vec![track.clone()],
            templates: song.templates.clone(),
        };
        let compiler_inst = compiler::Compiler::new(&single_song)?;
        let events = compiler_inst
            .compile(&single_song)
            .map_err(|e| miette!("Compiler error: {}", e))?;

        for event in events {
            out.push(ParsedEvent {
                event_type: "note".to_string(),
                track: track.name.clone(),
                channel: event.channel.saturating_add(1),
                note: Some(event.note),
                note_name: Some(midi_note_name(event.note)),
                velocity: Some(event.velocity),
                cc: None,
                value: None,
                program: None,
                time: event.time,
                duration: Some(event.duration),
                end_time: Some(event.time + event.duration),
            });
        }

        let channel = track.channel.saturating_sub(1).min(15).saturating_add(1);
        for init in &track.init_events {
            match init {
                dsl::token::TrackInitEvent::BankSelect { msb, lsb } => {
                    out.push(ParsedEvent {
                        event_type: "cc".to_string(),
                        track: track.name.clone(),
                        channel,
                        note: None,
                        note_name: None,
                        velocity: None,
                        cc: Some(0),
                        value: Some(*msb),
                        program: None,
                        time: 0.0,
                        duration: None,
                        end_time: None,
                    });
                    out.push(ParsedEvent {
                        event_type: "cc".to_string(),
                        track: track.name.clone(),
                        channel,
                        note: None,
                        note_name: None,
                        velocity: None,
                        cc: Some(32),
                        value: Some(*lsb),
                        program: None,
                        time: 0.0,
                        duration: None,
                        end_time: None,
                    });
                }
                dsl::token::TrackInitEvent::ProgramChange { program } => out.push(ParsedEvent {
                    event_type: "pc".to_string(),
                    track: track.name.clone(),
                    channel,
                    note: None,
                    note_name: None,
                    velocity: None,
                    cc: None,
                    value: None,
                    program: Some(*program),
                    time: 0.0,
                    duration: None,
                    end_time: None,
                }),
                dsl::token::TrackInitEvent::ControlChange { cc, value } => out.push(ParsedEvent {
                    event_type: "cc".to_string(),
                    track: track.name.clone(),
                    channel,
                    note: None,
                    note_name: None,
                    velocity: None,
                    cc: Some(*cc),
                    value: Some(*value),
                    program: None,
                    time: 0.0,
                    duration: None,
                    end_time: None,
                }),
            }
        }
    }

    Ok(out)
}

fn print_events(events: &[ParsedEvent], format: ParseFormat) -> Result<()> {
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

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn print_summary(events: &[ParsedEvent], format: ParseFormat) {
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
