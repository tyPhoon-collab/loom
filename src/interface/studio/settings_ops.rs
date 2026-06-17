use super::settings::{
    loop_range_for_bar_indices, score_body_start_row, set_loop_range_frontmatter,
    track_bar_index_at,
};
use super::StudioApp;
use miette::Result;

fn shifted_score_row(row: usize, before_source: &str, after_source: &str) -> usize {
    let Ok(before_start) = score_body_start_row(before_source) else {
        return row;
    };
    let Ok(after_start) = score_body_start_row(after_source) else {
        return row;
    };
    let delta = after_start as isize - before_start as isize;

    if row < before_start {
        row
    } else {
        row.saturating_add_signed(delta)
    }
}

impl StudioApp {
    pub(super) fn apply_selected_loop_range(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "Loop range applies to bar selection only".into();
            return Ok(());
        }

        let source = self.source();
        let mut selected_indices = Vec::with_capacity(selected_bars.len());
        for bar in &selected_bars {
            match track_bar_index_at(&source, bar.row, bar.index) {
                Ok(index) => selected_indices.push(index),
                Err(message) => {
                    self.status_message = message;
                    return Ok(());
                }
            }
        }
        let Some(start_index) = selected_indices.iter().min().copied() else {
            return Ok(());
        };
        let Some(end_index) = selected_indices.iter().max().copied() else {
            return Ok(());
        };
        let loop_range = match loop_range_for_bar_indices(&source, start_index, end_index) {
            Ok(loop_range) => loop_range,
            Err(message) => {
                self.status_message = message;
                return Ok(());
            }
        };
        match set_loop_range_frontmatter(&source, &loop_range) {
            Ok(updated_source) => {
                let selected_positions: Vec<(usize, usize)> = selected_bars
                    .iter()
                    .map(|bar| {
                        (
                            shifted_score_row(bar.row, &source, &updated_source),
                            bar.index,
                        )
                    })
                    .collect();
                self.push_source_undo();
                self.replace_source(updated_source);
                self.restore_bar_selection_from_positions(&selected_positions);
                self.sync_selection_visual();
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = format!("Loop range: {}", loop_range);
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }
}
