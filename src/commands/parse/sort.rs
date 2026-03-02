use super::model::ParsedEvent;
use crate::cli::ParseSortKey;

pub(super) fn sort_events(events: &mut [ParsedEvent], key: ParseSortKey) {
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
