use super::settings::{
    clear_loop_settings_frontmatter, loop_range_for_bar_indices, set_loop_range_frontmatter,
    toggle_loop_frontmatter,
};
use super::StudioApp;
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn apply_selected_loop_range(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "Loop range applies to bar selection only".into();
            return Ok(());
        }

        let Some(start_index) = selected_bars.iter().map(|bar| bar.index).min() else {
            return Ok(());
        };
        let Some(end_index) = selected_bars.iter().map(|bar| bar.index).max() else {
            return Ok(());
        };

        let source = self.source();
        let loop_range = match loop_range_for_bar_indices(&source, start_index, end_index) {
            Ok(loop_range) => loop_range,
            Err(message) => {
                self.status_message = message;
                return Ok(());
            }
        };
        match set_loop_range_frontmatter(&source, &loop_range) {
            Ok(source) => {
                let selected_positions: Vec<(usize, usize)> = selected_bars
                    .iter()
                    .map(|bar| (bar.row, bar.index))
                    .collect();
                self.push_source_undo();
                self.replace_source(source);
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

    pub(super) fn toggle_loop(&mut self) -> Result<()> {
        match toggle_loop_frontmatter(&self.source()) {
            Ok((source, enabled)) => {
                self.push_source_undo();
                self.replace_source(source);
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = if enabled {
                    "Loop: on".into()
                } else {
                    "Loop: off".into()
                };
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }

    pub(super) fn clear_loop_settings(&mut self) -> Result<()> {
        match clear_loop_settings_frontmatter(&self.source()) {
            Ok(Some(source)) => {
                let cursor = self.textarea.cursor();
                self.push_source_undo();
                self.replace_source(source);
                self.textarea
                    .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = "Loop cleared".into();
            }
            Ok(None) => {
                self.status_message = "No loop settings to clear".into();
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }
}
