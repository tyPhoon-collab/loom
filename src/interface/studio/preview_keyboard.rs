use super::note_entry::KeyboardVisualLayout;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

pub(super) fn preview_keyboard_deck_text(
    layout: &KeyboardVisualLayout,
    active_keys: &std::collections::HashSet<char>,
    octave: i32,
) -> Text<'static> {
    let chassis = Style::default().fg(Color::Gray).bg(Color::Rgb(16, 17, 20));
    let white_keys: Vec<_> = layout
        .pitch_keys
        .iter()
        .filter(|key| !key.is_black)
        .collect();
    let mut black_keys = Vec::new();
    let mut white_index = 0usize;
    for key in &layout.pitch_keys {
        if key.is_black {
            if white_index > 0 {
                black_keys.push((white_index - 1, key));
            }
        } else {
            white_index += 1;
        }
    }

    let mut canvas = KeyboardCanvas::new(
        white_keys.len() * WHITE_KEY_WIDTH + 1,
        KEYBOARD_HEIGHT,
        chassis,
    );
    draw_white_keybed(&mut canvas, &white_keys, active_keys, octave);
    draw_black_keys(&mut canvas, &black_keys, active_keys);

    Text::from(canvas.into_lines())
}

fn white_key_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Rgb(6, 20, 22))
            .bg(Color::Rgb(109, 232, 218))
    } else {
        Style::default()
            .fg(Color::Rgb(22, 24, 28))
            .bg(Color::Rgb(236, 230, 214))
    }
}

fn black_key_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Rgb(26, 18, 0))
            .bg(Color::Rgb(255, 206, 82))
    } else {
        Style::default()
            .fg(Color::Rgb(226, 230, 235))
            .bg(Color::Rgb(8, 9, 12))
    }
}

#[derive(Clone)]
struct KeyboardCanvas {
    cells: Vec<Vec<(char, Style)>>,
}

const WHITE_KEY_WIDTH: usize = 6;
const BLACK_KEY_WIDTH: usize = 4;
const KEYBOARD_HEIGHT: usize = 10;
const WHITE_KEY_BOTTOM: usize = 8;
const BLACK_KEY_BOTTOM: usize = 5;

impl KeyboardCanvas {
    fn new(width: usize, height: usize, style: Style) -> Self {
        Self {
            cells: vec![vec![(' ', style); width]; height],
        }
    }

    fn put(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if let Some(row) = self.cells.get_mut(y) {
            if let Some(cell) = row.get_mut(x) {
                *cell = (ch, style);
            }
        }
    }

    fn put_text_centered(&mut self, x: usize, y: usize, width: usize, text: &str, style: Style) {
        let chars: Vec<char> = text.chars().collect();
        let start = x + (width.saturating_sub(chars.len())) / 2;
        for (i, ch) in chars.into_iter().enumerate() {
            self.put(start + i, y, ch, style);
        }
    }

    fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, style: Style) {
        for dy in 0..height {
            for dx in 0..width {
                self.put(x + dx, y + dy, ' ', style);
            }
        }
    }

    fn into_lines(self) -> Vec<Line<'static>> {
        self.cells.into_iter().map(styled_cells_to_line).collect()
    }
}

fn draw_white_keybed(
    canvas: &mut KeyboardCanvas,
    white_keys: &[&super::note_entry::KeyboardVisualKey],
    active_keys: &std::collections::HashSet<char>,
    octave: i32,
) {
    for (i, key) in white_keys.iter().enumerate() {
        let style = white_key_style(active_keys.contains(&key.physical_key));
        let x = i * WHITE_KEY_WIDTH;
        let note = format!("{}{}", key.note_name, octave + key.octave_offset);
        let right = x + WHITE_KEY_WIDTH;

        canvas.fill_rect(x, 0, WHITE_KEY_WIDTH + 1, WHITE_KEY_BOTTOM + 1, style);
        canvas.put(x, 0, if i == 0 { '┌' } else { '┬' }, style);
        for dx in 1..WHITE_KEY_WIDTH {
            canvas.put(x + dx, 0, '─', style);
        }
        canvas.put(
            right,
            0,
            if i + 1 == white_keys.len() {
                '┐'
            } else {
                '┬'
            },
            style,
        );

        for y in 1..WHITE_KEY_BOTTOM {
            canvas.put(x, y, '│', style);
            canvas.put(right, y, '│', style);
        }
        canvas.put_text_centered(
            x + 1,
            6,
            WHITE_KEY_WIDTH - 1,
            &key.physical_key.to_ascii_uppercase().to_string(),
            style,
        );
        canvas.put_text_centered(x + 1, 7, WHITE_KEY_WIDTH - 1, &note, style);

        canvas.put(x, WHITE_KEY_BOTTOM, if i == 0 { '└' } else { '┴' }, style);
        for dx in 1..WHITE_KEY_WIDTH {
            canvas.put(x + dx, WHITE_KEY_BOTTOM, '─', style);
        }
        canvas.put(
            right,
            WHITE_KEY_BOTTOM,
            if i + 1 == white_keys.len() {
                '┘'
            } else {
                '┴'
            },
            style,
        );
    }
}

fn draw_black_keys(
    canvas: &mut KeyboardCanvas,
    black_keys: &[(usize, &super::note_entry::KeyboardVisualKey)],
    active_keys: &std::collections::HashSet<char>,
) {
    for (anchor, key) in black_keys {
        let style = black_key_style(active_keys.contains(&key.physical_key));
        let x = (anchor + 1) * WHITE_KEY_WIDTH - (BLACK_KEY_WIDTH / 2);
        canvas.fill_rect(x, 0, BLACK_KEY_WIDTH, BLACK_KEY_BOTTOM + 1, style);
        canvas.put(x, 0, '┌', style);
        canvas.put(x + 1, 0, '─', style);
        canvas.put(x + 2, 0, '─', style);
        canvas.put(x + 3, 0, '┐', style);
        for y in 1..BLACK_KEY_BOTTOM {
            canvas.put(x, y, '│', style);
            canvas.put(x + BLACK_KEY_WIDTH - 1, y, '│', style);
        }
        canvas.put_text_centered(
            x + 1,
            3,
            BLACK_KEY_WIDTH - 2,
            &key.physical_key.to_ascii_uppercase().to_string(),
            style,
        );
        canvas.put(x, BLACK_KEY_BOTTOM, '└', style);
        canvas.put(x + 1, BLACK_KEY_BOTTOM, '─', style);
        canvas.put(x + 2, BLACK_KEY_BOTTOM, '─', style);
        canvas.put(x + 3, BLACK_KEY_BOTTOM, '┘', style);
    }
}

fn styled_cells_to_line(cells: Vec<(char, Style)>) -> Line<'static> {
    let mut spans = Vec::new();
    let mut current_style = None;
    let mut current = String::new();

    for (ch, style) in cells {
        match current_style {
            Some(existing) if existing == style => current.push(ch),
            Some(existing) => {
                spans.push(Span::styled(std::mem::take(&mut current), existing));
                current.push(ch);
                current_style = Some(style);
            }
            None => {
                current.push(ch);
                current_style = Some(style);
            }
        }
    }

    if let Some(style) = current_style {
        spans.push(Span::styled(current, style));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::preview_keyboard_deck_text;
    use crate::interface::studio::note_entry::NoteKeyboard;
    use std::collections::HashSet;

    #[test]
    fn preview_keyboard_deck_uses_visual_layout_keys() {
        let layout = NoteKeyboard::default().visual_layout();
        let text = preview_keyboard_deck_text(&layout, &HashSet::new(), 4);
        let rendered = text
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("W"));
        assert!(rendered.contains("└──┘"));
        assert!(rendered.contains("A"));
        assert!(rendered.contains("C4"));
        assert!(rendered.contains("E5"));
    }
}
