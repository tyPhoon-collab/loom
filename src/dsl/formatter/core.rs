use crate::dsl::error::ParseError;
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::{Block, ModifierValue};
use std::collections::HashMap;
use std::fmt::Write;
use std::str::FromStr;

// --- Loom Grid Context ---

struct BlockGridInfo {
    grid_size: usize,
    column_widths: Vec<usize>,
    bar_width: usize,
}

struct FormatContext {
    blocks: Vec<BlockGridInfo>,
}

// --- Helpers ---

fn modifier_value_width(val: &ModifierValue) -> usize {
    match val {
        ModifierValue::Set(v) => format!("{}", v).len(),
        ModifierValue::Latch(v) => format!("!{}", v).len(),
        ModifierValue::Empty => 1,
        ModifierValue::NoteList(vals) => vals
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
            .len(),
        ModifierValue::Group(vals) => {
            // [v1 v2 ...] — brackets + spaces + each value
            if vals.is_empty() {
                return 2; // []
            }
            let inner: usize = vals.iter().map(|v| modifier_value_width(v).max(1)).sum();
            let spaces = vals.len() - 1;
            2 + inner + spaces // [ + values + spaces + ]
        }
    }
}

fn modifier_value_str(val: &ModifierValue) -> String {
    match val {
        ModifierValue::Set(v) => format!("{}", v),
        ModifierValue::Latch(v) => format!("!{}", v),
        ModifierValue::Empty => ".".to_string(),
        ModifierValue::NoteList(vals) => vals
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        ModifierValue::Group(vals) => {
            let inner: Vec<String> = vals
                .iter()
                .map(|v| {
                    let s = modifier_value_str(v);
                    if s.is_empty() {
                        ".".to_string()
                    } else {
                        s
                    }
                })
                .collect();
            format!("[{}]", inner.join(" "))
        }
    }
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        a %= b;
        std::mem::swap(&mut a, &mut b);
    }
    a
}

fn lcm(a: usize, b: usize) -> usize {
    if a == 0 || b == 0 {
        return 0;
    }
    (a * b) / gcd(a, b)
}

// --- Core Formatting ---

fn prepare_context(patterns: &[&ParsedLine]) -> FormatContext {
    let mut patterns_with_mods: Vec<(&ParsedLine, Vec<&ParsedLine>)> = Vec::new();

    for p in patterns {
        match p {
            ParsedLine::Pattern { .. } => {
                patterns_with_mods.push((p, Vec::new()));
            }
            ParsedLine::Modifier { .. } => {
                if let Some(last) = patterns_with_mods.last_mut() {
                    last.1.push(p);
                }
            }
            _ => {}
        }
    }

    let max_blocks = patterns
        .iter()
        .map(|p| match p {
            ParsedLine::Pattern { blocks, .. } => blocks.len(),
            ParsedLine::Modifier { blocks, .. } => blocks.len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    let mut blocks = Vec::new();

    for b_idx in 0..max_blocks {
        let mut token_counts = Vec::new();
        let mut max_bar_width = 0;

        for (p, mods) in &patterns_with_mods {
            if let ParsedLine::Pattern {
                blocks: pblocks, ..
            } = p
            {
                if let Some(block) = pblocks.get(b_idx) {
                    token_counts.push(block.tokens.len());
                    max_bar_width = max_bar_width.max(block.start_bar.to_string().len());
                }
            }
            for m in mods {
                if let ParsedLine::Modifier {
                    blocks: mblocks, ..
                } = m
                {
                    if let Some(block) = mblocks.get(b_idx) {
                        // Modifier values do NOT participate in LCM grid calculation.
                        // They follow the pattern's grid layout.
                        // Only contribute to column widths (done below).
                        let _ = block;
                    }
                }
            }
        }

        let k_max = token_counts.iter().copied().max().unwrap_or(0);

        let mut l = 1;
        for &k in &token_counts {
            if k > 0 {
                l = lcm(l, k);
            }
        }

        let mut g = l;
        if g > 0 {
            // Scale up to ensure at least some space between tokens, but only if k_max > 1
            if k_max > 1 {
                while g < k_max * 2 && g + l <= 24 {
                    g += l;
                }
            }
            if g > 24 {
                g = k_max.max(24);
            }
        } else {
            g = 1;
        }

        let num_cols = g + 2;
        let mut column_widths = vec![1; num_cols];

        for (p, mods) in &patterns_with_mods {
            if let ParsedLine::Pattern {
                blocks: pblocks, ..
            } = p
            {
                if let Some(block) = pblocks.get(b_idx) {
                    let k = block.tokens.len();
                    if k > 0 {
                        for (j, token) in block.tokens.iter().enumerate() {
                            let col = 1 + (j * g) / k;
                            if col < num_cols {
                                column_widths[col] =
                                    column_widths[col].max(token.to_string().len());
                            }
                        }
                    }
                }
            }
            for m in mods {
                if let ParsedLine::Modifier {
                    blocks: mblocks, ..
                } = m
                {
                    if let Some(block) = mblocks.get(b_idx) {
                        let k = block.values.len();
                        if k > 0 {
                            for (j, val) in block.values.iter().enumerate() {
                                let col = 1 + (j * g) / k;
                                if col < num_cols {
                                    let v_width = modifier_value_width(val);
                                    column_widths[col] = column_widths[col].max(v_width);
                                }
                            }
                        }
                    }
                }
            }
        }

        blocks.push(BlockGridInfo {
            grid_size: g,
            column_widths,
            bar_width: max_bar_width,
        });
    }

    FormatContext { blocks }
}

fn format_block(
    buf: &mut String,
    block: &Block,
    context: &FormatContext,
    block_index: usize,
) -> std::fmt::Result {
    let info = &context.blocks[block_index];
    let g = info.grid_size;
    let k = block.tokens.len();
    let num_cols = info.column_widths.len();

    write!(
        buf,
        "{:width$}",
        block.start_bar.to_string(),
        width = info.bar_width
    )?;

    let mut token_pos = HashMap::new();
    if k > 0 {
        for (j, t) in block.tokens.iter().enumerate() {
            token_pos.insert(1 + (j * g) / k, t);
        }
    }

    for col in 0..num_cols {
        let width = info.column_widths[col];
        if let Some(token) = token_pos.get(&col) {
            write!(buf, "{:width$}", token.to_string(), width = width)?;
        } else {
            write!(buf, "{:width$}", "", width = width)?;
        }
    }

    Ok(())
}

fn format_modifier_block(
    buf: &mut String,
    mblock: &crate::dsl::token::ModifierBlock,
    context: &FormatContext,
    block_index: usize,
) -> std::fmt::Result {
    let info = &context.blocks[block_index];
    let g = info.grid_size;
    let k = mblock.values.len();
    let num_cols = info.column_widths.len();

    write!(
        buf,
        "{:width$}",
        mblock.start_bar.to_string(),
        width = info.bar_width
    )?;

    let mut val_pos = HashMap::new();
    if k > 0 {
        for (j, v) in mblock.values.iter().enumerate() {
            val_pos.insert(1 + (j * g) / k, v);
        }
    }

    for col in 0..num_cols {
        let width = info.column_widths[col];
        if let Some(val) = val_pos.get(&col) {
            let s = modifier_value_str(val);
            write!(buf, "{:width$}", s, width = width)?;
        } else {
            write!(buf, "{:width$}", "", width = width)?;
        }
    }

    Ok(())
}

pub fn format_patterns(patterns: &[&ParsedLine]) -> Result<String, ParseError> {
    if patterns.is_empty() {
        return Ok(String::new());
    }

    let context = prepare_context(patterns);

    let (sorted_patterns, sorted_keys) = sort_patterns_and_mods(patterns)?;
    let mut max_key_width = calculate_max_key_width(&sorted_keys);
    for p in patterns {
        if let ParsedLine::Modifier { kind, .. } = p {
            max_key_width = max_key_width.max(kind.to_string().len());
        }
    }

    let mut pattern_to_mods: HashMap<*const ParsedLine, Vec<&ParsedLine>> = HashMap::new();
    let mut current_pattern: Option<*const ParsedLine> = None;
    for p in patterns {
        match p {
            ParsedLine::Pattern { .. } => {
                current_pattern = Some(*p as *const ParsedLine);
                pattern_to_mods.insert(current_pattern.unwrap(), Vec::new());
            }
            ParsedLine::Modifier { .. } => {
                if let Some(cp) = current_pattern {
                    pattern_to_mods.get_mut(&cp).unwrap().push(p);
                }
            }
            _ => {}
        }
    }

    let mut out = String::new();

    for p in patterns {
        if let ParsedLine::TemplateCalls(calls) = p {
            let joined = calls
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(out, "{}", joined).unwrap();
        }
    }

    // Prepare line contents and their widths (excluding trailing comments)
    let mut line_infos = Vec::new();
    let mut max_line_width = 0;

    for (i, p) in sorted_patterns.iter().enumerate() {
        if let ParsedLine::Pattern {
            blocks,
            end_bar,
            trailing_comment,
            ..
        } = p
        {
            let mut line_buf = String::new();
            let sorted_key = &sorted_keys[i];
            write!(line_buf, "{:width$} ", sorted_key, width = max_key_width).unwrap();

            for (b_idx, block) in blocks.iter().enumerate() {
                format_block(&mut line_buf, block, &context, b_idx).unwrap();
            }
            write!(line_buf, "{}", end_bar).unwrap();

            let mut music_part_width = line_buf.len();
            let mut mods_rendered = Vec::new();

            if let Some(mods) = pattern_to_mods.get(&(*p as *const ParsedLine)) {
                for m in mods.iter() {
                    if let ParsedLine::Modifier {
                        kind,
                        blocks: mod_blocks,
                        end_bar: m_end_bar,
                        trailing_comment: m_comment,
                        ..
                    } = m
                    {
                        let mut m_line_buf = String::new();
                        write!(m_line_buf, "{:width$} ", kind, width = max_key_width).unwrap();
                        for (b_idx, mblock) in mod_blocks.iter().enumerate() {
                            format_modifier_block(&mut m_line_buf, mblock, &context, b_idx)
                                .unwrap();
                        }
                        write!(m_line_buf, "{}", m_end_bar).unwrap();

                        music_part_width = music_part_width.max(m_line_buf.len());
                        mods_rendered.push((m_line_buf, m_comment));
                    }
                }
            }

            max_line_width = max_line_width.max(music_part_width);
            line_infos.push((line_buf, trailing_comment, mods_rendered));
        }
    }

    // Now output everything with aligned comments
    for (line_buf, comment, mods) in line_infos {
        out.push_str(&line_buf);
        if let Some(c) = comment {
            let padding = max_line_width.saturating_sub(line_buf.len());
            write!(out, "{:padding$} > {}", "", c, padding = padding).unwrap();
        }
        out.push('\n');

        for (m_buf, m_comment) in mods {
            out.push_str(&m_buf);
            if let Some(mc) = m_comment {
                let padding = max_line_width.saturating_sub(m_buf.len());
                write!(out, "{:padding$} > {}", "", mc, padding = padding).unwrap();
            }
            out.push('\n');
        }
    }
    Ok(out)
}

fn parse_note_strict(note_text: &str, key: &str) -> Result<crate::dsl::note::Note, ParseError> {
    crate::dsl::note::Note::from_str(note_text.trim()).map_err(|e| {
        ParseError::from_validation(
            key,
            key,
            format!(
                "Invalid note `{}` in key `{}`: {}",
                note_text.trim(),
                key,
                e
            ),
            Some("Use valid pitch/drum/MIDI note syntax in key list".to_string()),
        )
    })
}

fn sort_patterns_and_mods<'a>(
    patterns: &[&'a ParsedLine],
) -> Result<(Vec<&'a ParsedLine>, Vec<String>), ParseError> {
    let mut pattern_list: Vec<&ParsedLine> = patterns
        .iter()
        .filter(|p| matches!(p, ParsedLine::Pattern { .. }))
        .copied()
        .collect();

    let mut max_by_ptr: HashMap<*const ParsedLine, u8> = HashMap::new();
    let mut canonical_key_by_ptr: HashMap<*const ParsedLine, String> = HashMap::new();

    for p in &pattern_list {
        if let ParsedLine::Pattern { key, .. } = p {
            if key == "seq" {
                canonical_key_by_ptr.insert(*p as *const ParsedLine, "seq".to_string());
                continue;
            }

            let mut notes = Vec::new();
            for raw in key.split(',') {
                notes.push(parse_note_strict(raw, key)?);
            }

            if let Some(max_midi) = notes.iter().map(|n| n.to_midi()).max() {
                max_by_ptr.insert(*p as *const ParsedLine, max_midi);
            }

            let mut sorted_notes = notes;
            sorted_notes.sort_by_key(|n| n.to_midi());
            let canonical_key = sorted_notes
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",");
            canonical_key_by_ptr.insert(*p as *const ParsedLine, canonical_key);
        }
    }

    pattern_list.sort_by(|a, b| {
        let max_a = max_by_ptr.get(&(*a as *const ParsedLine));
        let max_b = max_by_ptr.get(&(*b as *const ParsedLine));
        match (max_a, max_b) {
            (Some(a), Some(b)) => b.cmp(a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    let mut sorted_keys = Vec::new();
    for p in &pattern_list {
        if let ParsedLine::Pattern { key, .. } = p {
            let ptr = *p as *const ParsedLine;
            let canonical = canonical_key_by_ptr
                .get(&ptr)
                .cloned()
                .unwrap_or_else(|| key.clone());
            sorted_keys.push(canonical);
        }
    }
    Ok((pattern_list, sorted_keys))
}

fn calculate_max_key_width(keys: &[String]) -> usize {
    keys.iter().map(|k| k.len()).max().unwrap_or(0)
}
