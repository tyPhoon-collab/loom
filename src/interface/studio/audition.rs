use super::selection::{unit_at_or_near_col, unit_spans_in_line, UnitSpan};
use super::settings::parse_track_header;
use super::StudioApp;
use crate::dsl::note::Note;
use crate::dsl::parser::{self, parse_track_init_command};
use crate::dsl::token::TrackInitEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PreviewTrackContext {
    pub(super) track_name: String,
    pub(super) channel: u8,
    pub(super) source_program: Option<u8>,
}

impl StudioApp {
    pub(super) fn current_loop_range(&self) -> Option<String> {
        parser::parse_song(self.source())
            .ok()
            .and_then(|song| song.metadata.loop_range)
    }

    pub(super) fn audition_candidate_from_lines(
        &self,
        lines: &[String],
        start_row: usize,
        end_row: usize,
        cursor: (usize, usize),
    ) -> Option<(usize, String)> {
        let preferred_row = if (start_row..=end_row).contains(&cursor.0) {
            cursor.0
        } else {
            start_row
        };

        unit_at_or_near_col(
            self.auditionable_spans_in_line(lines, preferred_row),
            cursor.1,
        )
        .or_else(|| {
            (start_row..=end_row).find_map(|row| {
                self.auditionable_spans_in_line(lines, row)
                    .into_iter()
                    .next()
            })
        })
        .map(|note| (note.row, note.token))
    }

    pub(super) fn auditionable_spans_in_line(&self, lines: &[String], row: usize) -> Vec<UnitSpan> {
        lines
            .get(row)
            .map(|line| {
                unit_spans_in_line(row, line)
                    .into_iter()
                    .filter(|note| note.kind.is_pitch())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn audition_candidate_from_indices(
        &self,
        indices: &[usize],
    ) -> Option<(usize, String)> {
        let notes = self.unit_spans();
        indices.iter().find_map(|index| {
            notes
                .get(*index)
                .filter(|note| note.kind.is_pitch())
                .map(|note| (note.row, note.token.clone()))
        })
    }

    pub(super) fn audition_candidate(&mut self, candidate: Option<(usize, String)>) {
        let Some((row, token)) = candidate else {
            return;
        };
        self.sync_playback_state();
        if self.is_playing {
            return;
        }
        if self.preview_token(row, &token).is_some() {
            self.status_message
                .push_str(&format!(" | Audition: {}", token));
        }
    }

    pub(super) fn preview_token(&self, row: usize, token: &str) -> Option<()> {
        let (channel, midi) = self.preview_target(row, token)?;
        self.player
            .preview_note(channel, midi, 96, std::time::Duration::from_millis(180));
        Some(())
    }

    pub(super) fn preview_target(&self, row: usize, token: &str) -> Option<(u8, u8)> {
        let note = token.parse::<Note>().ok()?;
        let midi = note.to_midi_checked().ok()?;
        let channel = match note {
            Note::Drum(_) => 9,
            _ => self.track_channel_for_row(row)?,
        };
        Some((channel, midi))
    }

    pub(super) fn preview_track_context_for_row(&self, row: usize) -> Option<PreviewTrackContext> {
        preview_track_context(self.textarea.lines(), row)
    }

    pub(super) fn clear_active_preview_notes(&mut self) {
        if self.active_preview_keys.is_empty() {
            return;
        }
        self.preview_panel.active_keys.clear();
        self.active_preview_keys.clear();
        self.player.preview_silence_all();
    }

    pub(super) fn track_channel_for_row(&self, row: usize) -> Option<u8> {
        self.preview_track_context_for_row(row)
            .map(|context| context.channel)
    }
}

fn preview_track_context(lines: &[String], row: usize) -> Option<PreviewTrackContext> {
    let mut current: Option<(usize, PreviewTrackContext)> = None;

    for (index, line) in lines.iter().enumerate().take(row + 1) {
        if let Some(header) = parse_track_header(line) {
            current = Some((
                index,
                PreviewTrackContext {
                    track_name: header.name,
                    channel: crate::validation::to_zero_based_channel(header.channel).ok()?,
                    source_program: None,
                },
            ));
        } else if line.trim().starts_with("# @") {
            current = None;
        }
    }

    let (header_row, mut context) = current?;
    for line in lines.iter().skip(header_row + 1) {
        if parse_track_header(line).is_some() || line.trim().starts_with("# @") {
            break;
        }

        let trimmed = line.trim();
        if let Some(command) = trimmed.strip_prefix("## ") {
            if let Ok((TrackInitEvent::ProgramChange { program }, _)) =
                parse_track_init_command(command)
            {
                context.source_program.get_or_insert(program);
                break;
            }
        }
    }

    Some(context)
}

#[cfg(test)]
mod tests {
    use super::{preview_track_context, PreviewTrackContext};

    #[test]
    fn preview_track_context_reads_track_channel_and_program() {
        let lines = vec![
            "# Piano: 2".to_string(),
            "## pc 41".to_string(),
            "C4 | ^ |".to_string(),
        ];

        assert_eq!(
            preview_track_context(&lines, 2),
            Some(PreviewTrackContext {
                track_name: "Piano".to_string(),
                channel: 1,
                source_program: Some(41),
            })
        );
    }

    #[test]
    fn preview_track_context_accepts_sound_alias() {
        let lines = vec![
            "# Lead: 1".to_string(),
            "## sound 81".to_string(),
            "C5 | ^ |".to_string(),
        ];

        assert_eq!(
            preview_track_context(&lines, 2),
            Some(PreviewTrackContext {
                track_name: "Lead".to_string(),
                channel: 0,
                source_program: Some(81),
            })
        );
    }

    #[test]
    fn preview_track_context_reads_program_from_header_row() {
        let lines = vec![
            "# Pad: 3".to_string(),
            "## pc 89".to_string(),
            "C4 | ^ |".to_string(),
        ];

        assert_eq!(
            preview_track_context(&lines, 0),
            Some(PreviewTrackContext {
                track_name: "Pad".to_string(),
                channel: 2,
                source_program: Some(89),
            })
        );
    }

    #[test]
    fn preview_track_context_stops_at_template_section() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "## pc 1".to_string(),
            "# @arp(x)".to_string(),
            "C4 | ^ |".to_string(),
        ];

        assert_eq!(preview_track_context(&lines, 3), None);
    }
}
