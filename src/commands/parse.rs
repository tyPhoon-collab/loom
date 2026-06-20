mod filters;
mod mapper;
mod model;
mod render;
mod sort;

use crate::cli::{ParseFormat, ParseSortKey};
use filters::{apply_filters, parse_filters};
use loom::dsl::parser;
use loom::inspect::collect_track_events;
use mapper::to_parsed_events;
use miette::Result;
use render::{print_events, print_summary};
use sort::sort_events;
use std::path::PathBuf;

pub fn handle_parse(
    input: PathBuf,
    format: ParseFormat,
    sort: ParseSortKey,
    filters: &[String],
    summary: bool,
) -> Result<()> {
    let song = parser::parse_song_from_path(&input)?;
    let track_events = collect_track_events(&song)?;

    let mut events = to_parsed_events(track_events);
    let parsed_filter = parse_filters(filters)?;
    events = apply_filters(events, &parsed_filter);
    sort_events(&mut events, sort);

    print_events(&events, format)?;
    if summary {
        print_summary(&events, format);
    }
    Ok(())
}
