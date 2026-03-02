use super::model::ParsedEvent;
use miette::{miette, Result};

#[derive(Default, Debug, Clone)]
pub(super) struct ParseFilter {
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

pub(super) fn parse_filters(inputs: &[String]) -> Result<ParseFilter> {
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

pub(super) fn apply_filters(events: Vec<ParsedEvent>, f: &ParseFilter) -> Vec<ParsedEvent> {
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
