use super::selection::BarSpan;
use super::StudioApp;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use ratatui_textarea::WrapMode;

const BAR_SELECTION_STYLE: Style = Style::new().bg(Color::Blue);
const BAR_FOCUS_STYLE: Style = Style::new().bg(Color::Cyan).fg(Color::Black);

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TextAreaViewport {
    pub(super) top_row: u16,
    pub(super) top_col: u16,
}

impl StudioApp {
    pub(super) fn update_textarea_scroll_top(&mut self, area: Rect) {
        let width = area.width;
        let height = area.height;
        let screen_cursor = self.textarea.screen_cursor();
        let top_row = next_scroll_top(
            self.textarea_viewport.top_row,
            screen_cursor.row as u16,
            height,
        );
        let top_col = if self.textarea.wrap_mode() == WrapMode::None {
            let mut cursor = screen_cursor.col as u16;
            if self.textarea.line_number_style().is_some() {
                let lnum = line_number_width(self.textarea.lines().len());
                if cursor <= lnum {
                    cursor *= 2;
                } else {
                    cursor += lnum;
                }
            }
            next_scroll_top(self.textarea_viewport.top_col, cursor, width)
        } else {
            0
        };
        self.textarea_viewport = TextAreaViewport { top_row, top_col };
    }

    pub(super) fn render_selection_overlay(&self, area: Rect, buf: &mut Buffer) {
        let Some((selected_bars, focus_bar)) = self.bar_overlay_spans() else {
            return;
        };
        let line_number_width = if self.textarea.line_number_style().is_some() {
            line_number_width(self.textarea.lines().len())
        } else {
            0
        };

        for bar in selected_bars {
            let screen_row = bar.row as i32 - self.textarea_viewport.top_row as i32;
            if screen_row < 0 || screen_row >= area.height as i32 {
                continue;
            }

            let start_col = bar.start_col as i32 + line_number_width as i32
                - self.textarea_viewport.top_col as i32;
            let end_col = bar.end_col as i32 + line_number_width as i32
                - self.textarea_viewport.top_col as i32;
            let visible_start = start_col.max(0);
            let visible_end = end_col.min(area.width as i32);
            if visible_start >= visible_end {
                continue;
            }

            let style = if Some(&bar) == focus_bar.as_ref() {
                BAR_FOCUS_STYLE
            } else {
                BAR_SELECTION_STYLE
            };
            buf.set_style(
                Rect {
                    x: area.x + visible_start as u16,
                    y: area.y + screen_row as u16,
                    width: (visible_end - visible_start) as u16,
                    height: 1,
                },
                style,
            );
        }
    }

    fn bar_overlay_spans(&self) -> Option<(Vec<BarSpan>, Option<BarSpan>)> {
        match &self.selection {
            Some(super::selection::StudioSelection::BarRange { anchor, focus })
                if anchor.row != focus.row =>
            {
                Some((self.selected_bar_spans(), self.focus_bar()))
            }
            _ => None,
        }
    }
}

fn next_scroll_top(prev_top: u16, cursor: u16, len: u16) -> u16 {
    if cursor < prev_top {
        cursor
    } else if prev_top + len <= cursor {
        cursor + 1 - len
    } else {
        prev_top
    }
}

fn line_number_width(line_count: usize) -> u16 {
    line_count.max(1).to_string().len() as u16 + 2
}
