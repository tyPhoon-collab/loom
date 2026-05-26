use super::input::PendingInput;
use super::keystroke::{
    key_stroke_matches, lookup_key_action, normalized_key_stroke, KeyBinding, KeyStroke,
};
use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, insert_at_col, is_seq_line, replace_char_range,
    unit_at_or_near_col, unit_spans_in_line, UnitSpan,
};
use super::settings::{parse_track_header, parse_track_header_channel};
use super::{StudioApp, StudioMode};
use crate::dsl::token::TrackInitLabel;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;

struct PlacedSlot {
    index_on_line: usize,
}

#[derive(Clone, Copy, Debug)]
enum AddKeyAction {
    Cancel,
    AddSeqLine,
    AddNoteHeadLine,
    AddTrack,
    AddPianoRollTrack,
    AddSeparator,
    AddDefaultDrumLanes,
    AddVelocityModifier,
    AddPitchModifier,
    BeginTemplateMacro,
    BeginTrackInitAdd,
    AddTemplateDefinition,
    AddBar,
    AddNearbyNote,
    AddRest,
    AddSustain,
}

const ADD_KEY_BINDINGS: &[KeyBinding<AddKeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: AddKeyAction::Cancel,
    },
    KeyBinding {
        stroke: KeyStroke::Char('s'),
        action: AddKeyAction::AddSeqLine,
    },
    KeyBinding {
        stroke: KeyStroke::Char('l'),
        action: AddKeyAction::AddNoteHeadLine,
    },
    KeyBinding {
        stroke: KeyStroke::Char('t'),
        action: AddKeyAction::AddTrack,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('p'),
        action: AddKeyAction::AddPianoRollTrack,
    },
    KeyBinding {
        stroke: KeyStroke::Char('h'),
        action: AddKeyAction::AddSeparator,
    },
    KeyBinding {
        stroke: KeyStroke::Char('d'),
        action: AddKeyAction::AddDefaultDrumLanes,
    },
    KeyBinding {
        stroke: KeyStroke::Char('v'),
        action: AddKeyAction::AddVelocityModifier,
    },
    KeyBinding {
        stroke: KeyStroke::Char('p'),
        action: AddKeyAction::AddPitchModifier,
    },
    KeyBinding {
        stroke: KeyStroke::Char('m'),
        action: AddKeyAction::BeginTemplateMacro,
    },
    KeyBinding {
        stroke: KeyStroke::Char('i'),
        action: AddKeyAction::BeginTrackInitAdd,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('t'),
        action: AddKeyAction::AddTemplateDefinition,
    },
    KeyBinding {
        stroke: KeyStroke::Char('b'),
        action: AddKeyAction::AddBar,
    },
    KeyBinding {
        stroke: KeyStroke::Char('n'),
        action: AddKeyAction::AddNearbyNote,
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('.'),
        action: AddKeyAction::AddRest,
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('-'),
        action: AddKeyAction::AddSustain,
    },
];

impl StudioApp {
    pub(super) fn handle_add_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(action) = lookup_key_action(ADD_KEY_BINDINGS, &key) else {
            self.reject_pending_input(PendingInput::Add);
            return Ok(());
        };

        match action {
            AddKeyAction::Cancel => self.cancel_pending_input(PendingInput::Add),
            AddKeyAction::AddSeqLine => self.add_seq_line()?,
            AddKeyAction::AddNoteHeadLine => self.add_note_head_line()?,
            AddKeyAction::AddTrack => self.add_track()?,
            AddKeyAction::AddPianoRollTrack => self.add_piano_roll_track()?,
            AddKeyAction::AddSeparator => self.add_separator()?,
            AddKeyAction::AddDefaultDrumLanes => self.add_default_drum_lanes()?,
            AddKeyAction::AddVelocityModifier => self.add_modifier_line("v")?,
            AddKeyAction::AddPitchModifier => self.add_modifier_line("p")?,
            AddKeyAction::BeginTemplateMacro => {
                if self.current_template_call_at_cursor().is_some() {
                    self.begin_pending_input(PendingInput::TemplateMacro);
                } else {
                    self.status_message =
                        "Template macro add needs the cursor on a template call".into();
                }
            }
            AddKeyAction::BeginTrackInitAdd => {
                self.begin_pending_input(PendingInput::TrackInitAdd);
            }
            AddKeyAction::AddTemplateDefinition => self.add_template_definition()?,
            AddKeyAction::AddBar => self.add_bar()?,
            AddKeyAction::AddNearbyNote => {
                let token = self.note_token_for_add();
                self.place_token_at_current_slot(&token)?;
            }
            AddKeyAction::AddRest => {
                self.place_token_at_current_slot(".")?;
            }
            AddKeyAction::AddSustain => {
                self.place_token_at_current_slot("-")?;
            }
        }
        Ok(())
    }

    pub(super) fn handle_track_init_add_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(spec) = parse_track_init_key(key) else {
            self.reject_pending_input(PendingInput::TrackInitAdd);
            return Ok(());
        };

        if matches!(spec, TrackInitKeySpec::Cancel) {
            self.cancel_pending_input(PendingInput::TrackInitAdd);
            return Ok(());
        }

        self.add_track_init_line(spec.init_template())
    }

    pub(super) fn add_default_drum_lanes(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let inserted = vec![
            String::new(),
            "# Drums: 10".to_string(),
            "kick  | . . . . |".to_string(),
            "snare | . . . . |".to_string(),
            "hh    | . . . . |".to_string(),
            "oh    | . . . . |".to_string(),
        ];
        lines.splice(insert_row..insert_row, inserted);
        self.apply_cursor_source_update(
            lines,
            (insert_row + 2, 8),
            "Added default drum lanes".into(),
            None,
        )
    }

    pub(super) fn add_piano_roll_track(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let (track_name, channel) = next_track_header(&lines);
        let inserted = piano_roll_track_lines(&track_name, channel);
        lines.splice(insert_row..insert_row, inserted);
        self.apply_cursor_source_update(
            lines,
            (insert_row + 9, 0),
            format!(
                "Added piano-roll track: {} on channel {}",
                track_name, channel
            ),
            Some((insert_row + 9, "G4".to_string())),
        )
    }

    pub(super) fn add_seq_line(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        lines.insert(insert_row, "seq | . . . . |".to_string());
        self.apply_cursor_source_update(lines, (insert_row, 6), "Added seq line".into(), None)
    }

    pub(super) fn add_note_head_line(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let note = self.note_token_for_add();
        lines.insert(insert_row, format!("{} | ^ . . . |", note));
        self.apply_cursor_source_update(
            lines,
            (insert_row, 0),
            format!("Added note-head line: {}", note),
            Some((insert_row, note)),
        )
    }

    pub(super) fn add_track(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let (track_name, channel) = next_track_header(&lines);
        let inserted = vec![
            String::new(),
            format!("# {}: {}", track_name, channel),
            "seq | . . . . |".to_string(),
        ];
        lines.splice(insert_row..insert_row, inserted);
        self.apply_cursor_source_update(
            lines,
            (insert_row + 2, 6),
            format!("Added track: {} on channel {}", track_name, channel),
            None,
        )
    }

    pub(super) fn add_separator(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        insert_separator_at_row(&mut lines, insert_row);
        self.apply_cursor_source_update(lines, (insert_row, 0), "Added separator".into(), None)
    }

    pub(super) fn add_template_definition(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let template_name = next_empty_template_name(&lines, cursor.0);

        let inserted = vec![
            String::new(),
            format!("# @{}", template_name),
            "seq | . . . . |".to_string(),
        ];
        lines.splice(insert_row..insert_row, inserted);
        self.apply_cursor_source_update(
            lines,
            (insert_row + 2, 6),
            format!("Added template definition: @{}", template_name),
            None,
        )
    }

    pub(super) fn add_bar(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let Ok(new_cursor_col) = add_rest_bar_to_line(line) else {
            self.status_message = "Add bar needs a line with bars".into();
            return Ok(());
        };
        self.apply_cursor_source_update(lines, (cursor.0, new_cursor_col), "Added bar".into(), None)
    }

    pub(super) fn add_modifier_line(&mut self, label: &str) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(insert_row) = modifier_insert_row(&lines, cursor.0) else {
            self.status_message =
                "Modifier line add needs a seq, note-head, or drum lane block".into();
            return Ok(());
        };

        let template = format!("{:<2} | . . . . |", label);
        lines.insert(insert_row, template);
        self.apply_cursor_source_update(
            lines,
            (insert_row, 5),
            format!("Added {} modifier line", modifier_label_name(label)),
            None,
        )
    }

    pub(super) fn place_token_at_current_slot(&mut self, token: &str) -> Result<bool> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(false);
        };

        let Ok(slot) = place_seq_token_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Place token currently supports seq lines only".into();
            return Ok(false);
        };

        let cursor_col = unit_spans_in_line(cursor.0, line)
            .get(slot.index_on_line)
            .map(|note| note.start_col)
            .unwrap_or(cursor.1);
        let audition = (token != "." && token != "-").then(|| (cursor.0, token.to_string()));
        self.apply_cursor_source_update(
            lines,
            (cursor.0, cursor_col),
            format!("Placed {}", token),
            audition,
        )?;
        Ok(true)
    }

    pub(super) fn note_token_for_add(&self) -> String {
        let cursor = self.textarea.cursor();
        if let Some(note) = unit_at_or_near_col(
            self.auditionable_spans_in_line(self.textarea.lines(), cursor.0),
            cursor.1,
        ) {
            return note.token;
        }

        self.textarea
            .lines()
            .iter()
            .enumerate()
            .take(cursor.0 + 1)
            .rev()
            .find_map(|(row, _)| {
                self.auditionable_spans_in_line(self.textarea.lines(), row)
                    .into_iter()
                    .next_back()
            })
            .map(|note| note.token)
            .unwrap_or_else(|| "C4".to_string())
    }

    fn add_track_init_line(&mut self, template: TrackInitTemplate) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some((header_row, insert_row)) = track_init_insert_row(&lines, cursor.0) else {
            self.status_message = "Track init add needs the cursor inside a track".into();
            return Ok(());
        };

        let line = template.line();
        let cursor_col = template.value_col();
        lines.insert(insert_row, line);
        self.apply_cursor_source_update(
            lines,
            (insert_row, cursor_col),
            format!(
                "Added track init {} on track line {}",
                template.label,
                header_row + 1
            ),
            None,
        )?;
        self.mode = StudioMode::Insert;
        self.status_message = format!("Added track init {} | Insert mode", template.label);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct TrackInitTemplate {
    label: TrackInitLabel,
    text: &'static str,
    value_col: usize,
}

impl TrackInitTemplate {
    fn line(self) -> String {
        format!("## {}", self.text)
    }

    fn value_col(self) -> usize {
        self.value_col
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrackInitKeySpec {
    Cancel,
    Pc,
    Bank,
    Cc,
    Pan,
    Volume,
    Expression,
    Mod,
    Sustain,
}

impl TrackInitKeySpec {
    fn init_template(self) -> TrackInitTemplate {
        match self {
            TrackInitKeySpec::Cancel => unreachable!(),
            TrackInitKeySpec::Pc => TrackInitTemplate {
                label: TrackInitLabel::Pc,
                text: "pc 0",
                value_col: 6,
            },
            TrackInitKeySpec::Bank => TrackInitTemplate {
                label: TrackInitLabel::Bank,
                text: "bank 0/0",
                value_col: 8,
            },
            TrackInitKeySpec::Cc => TrackInitTemplate {
                label: TrackInitLabel::Cc,
                text: "cc 1 0",
                value_col: 6,
            },
            TrackInitKeySpec::Pan => TrackInitTemplate {
                label: TrackInitLabel::Pan,
                text: "pan 64",
                value_col: 7,
            },
            TrackInitKeySpec::Volume => TrackInitTemplate {
                label: TrackInitLabel::Volume,
                text: "volume 100",
                value_col: 10,
            },
            TrackInitKeySpec::Expression => TrackInitTemplate {
                label: TrackInitLabel::Expression,
                text: "expression 100",
                value_col: 14,
            },
            TrackInitKeySpec::Mod => TrackInitTemplate {
                label: TrackInitLabel::Mod,
                text: "mod 0",
                value_col: 7,
            },
            TrackInitKeySpec::Sustain => TrackInitTemplate {
                label: TrackInitLabel::Sustain,
                text: "sustain 0",
                value_col: 11,
            },
        }
    }
}

fn parse_track_init_key(key: KeyEvent) -> Option<TrackInitKeySpec> {
    if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), &key) {
        return Some(TrackInitKeySpec::Cancel);
    }

    match normalized_key_stroke(&key) {
        Some(KeyStroke::Char(ch)) | Some(KeyStroke::ShiftChar(ch)) => match ch {
            'p' => Some(TrackInitKeySpec::Pc),
            'b' => Some(TrackInitKeySpec::Bank),
            'c' => Some(TrackInitKeySpec::Cc),
            'n' => Some(TrackInitKeySpec::Pan),
            'v' => Some(TrackInitKeySpec::Volume),
            'e' => Some(TrackInitKeySpec::Expression),
            'm' => Some(TrackInitKeySpec::Mod),
            's' => Some(TrackInitKeySpec::Sustain),
            _ => None,
        },
        _ => None,
    }
}

fn track_init_insert_row(lines: &[String], cursor_row: usize) -> Option<(usize, usize)> {
    let header_row = current_track_header_row(lines, cursor_row)?;
    let mut insert_row = header_row + 1;
    while insert_row < lines.len() && is_track_init_line(&lines[insert_row]) {
        insert_row += 1;
    }
    Some((header_row, insert_row))
}

fn current_track_header_row(lines: &[String], cursor_row: usize) -> Option<usize> {
    (0..=cursor_row)
        .rev()
        .find(|&row| {
            lines
                .get(row)
                .and_then(|line| parse_track_header(line))
                .is_some()
        })
        .or_else(|| {
            lines
                .iter()
                .position(|line| parse_track_header(line).is_some())
        })
}

fn is_track_init_line(line: &str) -> bool {
    line.trim_start().starts_with("## ")
}

fn insert_row_after_cursor(lines: &[String], cursor_row: usize) -> usize {
    if lines.is_empty() {
        0
    } else {
        (cursor_row + 1).min(lines.len())
    }
}

fn insert_separator_at_row(lines: &mut Vec<String>, row: usize) {
    lines.insert(row, "---".to_string());
}

fn modifier_insert_row(lines: &[String], cursor_row: usize) -> Option<usize> {
    let anchor_row = if lines
        .get(cursor_row)
        .is_some_and(|line| is_pattern_line(line) || is_modifier_line(line))
    {
        cursor_row
    } else {
        (0..=cursor_row).rev().find(|&row| {
            lines
                .get(row)
                .is_some_and(|line| is_pattern_line(line) || is_modifier_line(line))
        })?
    };

    if lines
        .get(anchor_row)
        .is_some_and(|line| is_modifier_line(line))
    {
        let pattern_row = (0..anchor_row)
            .rev()
            .find(|&row| lines.get(row).is_some_and(|line| is_pattern_line(line)))?;
        return Some(first_non_modifier_row_after(lines, pattern_row));
    }

    Some(first_non_modifier_row_after(lines, anchor_row))
}

fn first_non_modifier_row_after(lines: &[String], row: usize) -> usize {
    let mut insert_row = row + 1;
    while insert_row < lines.len() && is_modifier_line(&lines[insert_row]) {
        insert_row += 1;
    }
    insert_row
}

fn is_pattern_line(line: &str) -> bool {
    line.contains('|') && !is_modifier_line(line) && !line.trim().starts_with("[@")
}

fn is_modifier_line(line: &str) -> bool {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return false;
    };
    matches!(
        line.chars().take(pipe_col).collect::<String>().trim(),
        "v" | "p"
    )
}

fn modifier_label_name(label: &str) -> &'static str {
    match label {
        "v" => "velocity",
        "p" => "pitch",
        _ => "unknown",
    }
}

fn piano_roll_track_lines(track_name: &str, channel: u8) -> Vec<String> {
    let mut lines = vec![String::new(), format!("# {}: {}", track_name, channel)];
    lines.extend(
        [
            "C4", "C#4", "D4", "D#4", "E4", "F4", "F#4", "G4", "G#4", "A4", "A#4", "B4",
        ]
        .into_iter()
        .map(|note| format!("{:<4}| . . . . |", note)),
    );
    lines
}

fn next_track_header(lines: &[String]) -> (String, u8) {
    let track_count = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('#') && !trimmed.starts_with("##")
        })
        .count();
    let next_channel = lines
        .iter()
        .filter_map(|line| parse_track_header_channel(line))
        .map(|channel| channel + 2)
        .max()
        .unwrap_or(1)
        .min(16);
    (format!("Track {}", track_count + 1), next_channel)
}

fn next_empty_template_name(lines: &[String], cursor_row: usize) -> String {
    let existing: std::collections::HashSet<String> = lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix("# @"))
        .map(|name| name.trim().to_string())
        .collect();

    let base = (0..=cursor_row)
        .rev()
        .find_map(|row| lines.get(row).and_then(|line| parse_track_header(line)))
        .map(|header| slugify_template_name(&header.name))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| "template".to_string());

    if !existing.contains(&base) {
        return base;
    }

    let mut index = 1usize;
    loop {
        let candidate = format!("{}{}", base, index);
        if !existing.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn slugify_template_name(input: &str) -> String {
    let mut slug = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        }
    }
    slug
}

fn add_rest_bar_to_line(line: &mut String) -> std::result::Result<usize, ()> {
    if !line.contains('|') {
        return Err(());
    }
    let trimmed_len = line.trim_end().chars().count();
    replace_char_range(line, trimmed_len, line.chars().count(), "");
    let cursor_col = line.chars().count() + 1;
    line.push_str(" . . . . |");
    Ok(cursor_col)
}

fn place_seq_token_at_slot(
    row: usize,
    line: &mut String,
    col: usize,
    token: &str,
) -> std::result::Result<PlacedSlot, ()> {
    if !is_seq_line(line) {
        return Err(());
    }

    let notes = unit_spans_in_line(row, line);
    if let Some((index, note)) = unit_at_or_near_col_with_index(&notes, col) {
        replace_char_range(line, note.start_col, note.end_col, token);
        return Ok(PlacedSlot {
            index_on_line: index,
        });
    }

    let Some(bar) = bar_at_or_near_col(bar_spans_in_line(row, line), col) else {
        return Err(());
    };
    let insertion_col = bar.start_col + 1;
    let chars: Vec<char> = line.chars().collect();
    let insertion = if chars
        .get(insertion_col)
        .is_some_and(|ch| ch.is_whitespace())
    {
        format!(" {}", token)
    } else {
        format!(" {} ", token)
    };
    insert_at_col(line, insertion_col, &insertion);
    Ok(PlacedSlot { index_on_line: 0 })
}

fn unit_at_or_near_col_with_index(notes: &[UnitSpan], col: usize) -> Option<(usize, UnitSpan)> {
    notes
        .iter()
        .enumerate()
        .find(|(_, note)| col >= note.start_col && col < note.end_col)
        .map(|(index, note)| (index, note.clone()))
        .or_else(|| {
            notes
                .iter()
                .enumerate()
                .find(|(_, note)| note.start_col >= col)
                .map(|(index, note)| (index, note.clone()))
        })
        .or_else(|| {
            notes
                .iter()
                .enumerate()
                .next_back()
                .map(|(index, note)| (index, note.clone()))
        })
}

#[cfg(test)]
mod tests {
    use super::{
        add_rest_bar_to_line, current_track_header_row, insert_separator_at_row,
        next_empty_template_name, next_track_header, parse_track_init_key, piano_roll_track_lines,
        place_seq_token_at_slot, track_init_insert_row, TrackInitKeySpec,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    #[test]
    pub(super) fn add_rest_bar_appends_grid_bar() {
        let mut line = "seq | C4 . |".to_string();
        let cursor_col = add_rest_bar_to_line(&mut line).unwrap();
        assert_eq!(line, "seq | C4 . | . . . . |");
        assert_eq!(cursor_col, 13);
    }

    #[test]
    pub(super) fn place_seq_token_replaces_current_slot() {
        let mut line = "seq | C4 . E4 |".to_string();
        let placed = place_seq_token_at_slot(0, &mut line, 9, "D4").unwrap();
        assert_eq!(line, "seq | C4 D4 E4 |");
        assert_eq!(placed.index_on_line, 1);
    }

    #[test]
    pub(super) fn place_seq_token_in_empty_bar() {
        let mut line = "seq | |".to_string();
        let placed = place_seq_token_at_slot(0, &mut line, 5, "C4").unwrap();
        assert_eq!(line, "seq | C4 |");
        assert_eq!(placed.index_on_line, 0);
    }

    #[test]
    pub(super) fn next_track_header_uses_next_track_number_and_channel() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "# Bass: 3".to_string(),
        ];
        assert_eq!(next_track_header(&lines), ("Track 3".to_string(), 4));
    }

    #[test]
    pub(super) fn insert_separator_at_row_inserts_rule() {
        let mut lines = vec!["# Piano: 1".to_string(), "seq | C4 |".to_string()];
        insert_separator_at_row(&mut lines, 1);
        assert_eq!(
            lines,
            vec![
                "# Piano: 1".to_string(),
                "---".to_string(),
                "seq | C4 |".to_string()
            ]
        );
    }

    #[test]
    pub(super) fn next_empty_template_name_prefers_track_name() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "# @piano".to_string(),
        ];
        assert_eq!(next_empty_template_name(&lines, 1), "piano1");
    }

    #[test]
    fn current_track_header_row_finds_enclosing_track() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "## pc 4".to_string(),
            "seq | C4 |".to_string(),
            "# Bass: 2".to_string(),
        ];
        assert_eq!(current_track_header_row(&lines, 2), Some(0));
        assert_eq!(current_track_header_row(&lines, 3), Some(3));
    }

    #[test]
    fn track_init_insert_row_skips_existing_init_lines() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "## bank 0/32".to_string(),
            "## pc 40".to_string(),
            "seq | C4 |".to_string(),
        ];
        assert_eq!(track_init_insert_row(&lines, 3), Some((0, 3)));
    }

    #[test]
    fn parse_track_init_key_maps_supported_bindings() {
        let key = |code, modifiers| KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('p'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Pc)
        );
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Pan)
        );
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('s'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Sustain)
        );
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Volume)
        );
    }

    #[test]
    fn piano_roll_track_lines_create_header_and_chromatic_lanes() {
        let lines = piano_roll_track_lines("Track 2", 3);
        assert_eq!(lines.first().map(String::as_str), Some(""));
        assert_eq!(lines.get(1).map(String::as_str), Some("# Track 2: 3"));
        assert_eq!(lines.get(2).map(String::as_str), Some("C4  | . . . . |"));
        assert_eq!(lines.get(3).map(String::as_str), Some("C#4 | . . . . |"));
        assert_eq!(lines.get(9).map(String::as_str), Some("G4  | . . . . |"));
        assert_eq!(lines.last().map(String::as_str), Some("B4  | . . . . |"));
        assert_eq!(lines.len(), 14);
    }
}
