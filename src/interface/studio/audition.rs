use super::selection::{
    note_at_or_near_col, note_spans_in_line, NoteTokenSpan, SelectableTokenKind,
};
use super::settings::parse_track_header_channel;
use super::StudioApp;
use crate::dsl::note::Note;
use crate::dsl::parser;
use std::time::Duration;

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

        note_at_or_near_col(
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

    pub(super) fn auditionable_spans_in_line(
        &self,
        lines: &[String],
        row: usize,
    ) -> Vec<NoteTokenSpan> {
        lines
            .get(row)
            .map(|line| {
                note_spans_in_line(row, line)
                    .into_iter()
                    .filter(|note| note.kind == SelectableTokenKind::Note)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn audition_candidate_from_indices(
        &self,
        indices: &[usize],
    ) -> Option<(usize, String)> {
        let notes = self.note_token_spans();
        indices.iter().find_map(|index| {
            notes
                .get(*index)
                .filter(|note| note.kind == SelectableTokenKind::Note)
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
        let note = token.parse::<Note>().ok()?;
        let midi = note.to_midi_checked().ok()?;
        let channel = match note {
            Note::Drum(_) => 9,
            _ => self.track_channel_for_row(row)?,
        };
        self.player
            .preview_note(channel, midi, 96, Duration::from_millis(180));
        Some(())
    }

    pub(super) fn track_channel_for_row(&self, row: usize) -> Option<u8> {
        self.textarea
            .lines()
            .iter()
            .take(row + 1)
            .rev()
            .find_map(|line| parse_track_header_channel(line))
    }
}
