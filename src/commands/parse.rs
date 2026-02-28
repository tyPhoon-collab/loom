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
    track: String,
    channel: u8, // 1-based
    note: u8,
    note_name: String,
    velocity: u8,
    time: f64,
    duration: f64,
    end_time: f64,
}

#[derive(Tabled)]
struct ParsedEventTableRow {
    #[tabled(rename = "Track")]
    track: String,
    #[tabled(rename = "CH")]
    channel: u8,
    #[tabled(rename = "Note")]
    note: u8,
    #[tabled(rename = "Name")]
    note_name: String,
    #[tabled(rename = "Vel")]
    velocity: u8,
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
            track: e.track.clone(),
            channel: e.channel,
            note: e.note,
            note_name: e.note_name.clone(),
            velocity: e.velocity,
            time: format!("{:.2}", e.time),
            duration: format!("{:.2}", e.duration),
            end_time: format!("{:.2}", e.end_time),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct ParseFilter {
    track: Option<String>,
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
                        "Unknown filter key '{}'. Supported: track,channel,note,note_name,velocity,velocity_min,velocity_max,time_min,time_max",
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
                && f.channel.is_none_or(|v| e.channel == v)
                && f.note.is_none_or(|v| e.note == v)
                && f.note_name
                    .as_ref()
                    .is_none_or(|v| e.note_name.to_ascii_uppercase() == *v)
                && f.velocity_min.is_none_or(|v| e.velocity >= v)
                && f.velocity_max.is_none_or(|v| e.velocity <= v)
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
                .then_with(|| a.note.cmp(&b.note))
                .then_with(|| a.channel.cmp(&b.channel))
        }),
        ParseSortKey::Note => events.sort_by(|a, b| {
            a.note
                .cmp(&b.note)
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Channel => events.sort_by(|a, b| {
            a.channel
                .cmp(&b.channel)
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Velocity => events.sort_by(|a, b| {
            a.velocity
                .cmp(&b.velocity)
                .then_with(|| a.time.partial_cmp(&b.time).unwrap())
        }),
        ParseSortKey::Duration => events.sort_by(|a, b| {
            a.duration
                .partial_cmp(&b.duration)
                .unwrap()
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
                track: track.name.clone(),
                channel: event.channel.saturating_add(1),
                note: event.note,
                note_name: midi_note_name(event.note),
                velocity: event.velocity,
                time: event.time,
                duration: event.duration,
                end_time: event.time + event.duration,
            });
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
            println!("track,channel,note,note_name,velocity,time,duration,end_time");
            for e in events {
                println!(
                    "{},{},{},{},{},{:.6},{:.6},{:.6}",
                    csv_escape(&e.track),
                    e.channel,
                    e.note,
                    csv_escape(&e.note_name),
                    e.velocity,
                    e.time,
                    e.duration,
                    e.end_time
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
        note_min = note_min.min(e.note);
        note_max = note_max.max(e.note);
        vel_min = vel_min.min(e.velocity);
        vel_max = vel_max.max(e.velocity);
        vel_sum += e.velocity as u64;
        t_min = t_min.min(e.time);
        t_max = t_max.max(e.time);
        end_max = end_max.max(e.end_time);
    }

    let vel_avg = vel_sum as f64 / events.len() as f64;
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
    emit(format!(
        "  note_range: {}({}) .. {}({})",
        note_min,
        midi_note_name(note_min),
        note_max,
        midi_note_name(note_max)
    ));
    emit(format!(
        "  velocity: min={}, max={}, avg={:.2}",
        vel_min, vel_max, vel_avg
    ));
}
