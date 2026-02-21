use crate::dsl::parser::ParsedLine;
use crate::dsl::token::{Block, ModifierValue};
use std::collections::HashMap;
use std::fmt::Write;
use std::str::FromStr;

// --- Equal Formatter Context ---

struct BlockInfo {
    bar_width: usize,
    max_tokens: usize,
    token_widths: Vec<usize>,
}

struct FormatContext {
    blocks: Vec<BlockInfo>,
}

// --- Helpers ---

fn modifier_value_width(val: &ModifierValue) -> usize {
    match val {
        ModifierValue::Set(v) => format!("{}", v).len(),
        ModifierValue::Latch(v) => format!("!{}", v).len(),
    }
}

fn modifier_value_str(val: &ModifierValue) -> String {
    match val {
        ModifierValue::Set(v) => format!("{}", v),
        ModifierValue::Latch(v) => format!("!{}", v),
    }
}

// --- Core Formatting ---

fn prepare_context(patterns: &[&ParsedLine], min_slot_widths: &[usize]) -> FormatContext {
    let max_blocks = patterns
        .iter()
        .map(|p| match p {
            ParsedLine::Pattern { blocks, .. } => blocks.len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    let mut block_infos = Vec::new();

    for i in 0..max_blocks {
        let mut max_bar_width = 0;
        let mut max_tokens = 0;
        let min_sw = min_slot_widths.get(i).copied().unwrap_or(1);

        for p in patterns {
            if let ParsedLine::Pattern { blocks, .. } = p {
                if let Some(block) = blocks.get(i) {
                    max_bar_width = max_bar_width.max(block.start_bar.to_string().len());
                    max_tokens = max_tokens.max(block.tokens.len());
                }
            }
        }

        let mut token_widths = vec![0; max_tokens];
        for p in patterns {
            if let ParsedLine::Pattern { blocks, .. } = p {
                if let Some(block) = blocks.get(i) {
                    let k = block.tokens.len();
                    if k == 0 {
                        continue;
                    }
                    let m = max_tokens;

                    if k == 1 {
                        token_widths[0] = token_widths[0].max(block.tokens[0].to_string().len());
                    } else {
                        for (t_idx, t) in block.tokens.iter().enumerate() {
                            let slot = ((t_idx as f64 * (m - 1) as f64) / ((k - 1) as f64)).round()
                                as usize;
                            if slot < max_tokens {
                                token_widths[slot] = token_widths[slot].max(t.to_string().len());
                            }
                        }
                    }
                }
            }
        }

        // Apply modifier min slot widths only to non-Group positions
        for (slot_idx, tw) in token_widths.iter_mut().enumerate() {
            let is_group_slot = patterns.iter().any(|p| {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        let k = block.tokens.len();
                        if k == 0 {
                            return false;
                        }
                        let m = max_tokens;
                        for (t_idx, t) in block.tokens.iter().enumerate() {
                            let mapped_slot = if k == 1 {
                                0
                            } else {
                                ((t_idx as f64 * (m - 1) as f64) / ((k - 1) as f64)).round()
                                    as usize
                            };
                            if mapped_slot == slot_idx && t.is_group() {
                                return true;
                            }
                        }
                    }
                }
                false
            });

            if !is_group_slot {
                *tw = (*tw).max(min_sw);
            }
        }

        block_infos.push(BlockInfo {
            bar_width: max_bar_width,
            max_tokens,
            token_widths,
        });
    }

    FormatContext {
        blocks: block_infos,
    }
}

fn format_block(
    buf: &mut String,
    block: &Block,
    context: &FormatContext,
    block_index: usize,
) -> std::fmt::Result {
    let info = &context.blocks[block_index];

    write!(
        buf,
        "{:width$} ",
        block.start_bar.to_string(),
        width = info.bar_width
    )?;

    let k = block.tokens.len();
    let m = info.max_tokens;
    let mut token_map = HashMap::new();

    if k > 0 {
        if k == 1 {
            token_map.insert(0, &block.tokens[0]);
        } else {
            for (t_idx, t) in block.tokens.iter().enumerate() {
                let slot = ((t_idx as f64 * (m - 1) as f64) / ((k - 1) as f64)).round() as usize;
                token_map.insert(slot, t);
            }
        }
    }

    for (slot_idx, &slot_w) in info.token_widths.iter().enumerate() {
        if let Some(token) = token_map.get(&slot_idx) {
            write!(buf, "{:width$}", token.to_string(), width = slot_w)?;
        } else {
            write!(buf, "{:width$}", "", width = slot_w)?;
        }
        write!(buf, " ")?;
    }
    Ok(())
}

fn slot_widths(context: &FormatContext, block_index: usize, token_count: usize) -> Vec<usize> {
    if let Some(info) = context.blocks.get(block_index) {
        if token_count == 0 {
            return vec![];
        }
        let mut widths = vec![0; token_count];
        let m = info.max_tokens;

        // Pattern content width including spaces
        let total_content_width = info.token_widths.iter().map(|w| w + 1).sum::<usize>();

        if token_count == 1 {
            widths[0] = total_content_width.saturating_sub(1);
        } else if m <= 1 {
            widths.fill(info.token_widths.first().copied().unwrap_or(0));
        } else {
            let mut last_slot = 0;
            for t_idx in 0..token_count {
                let slot =
                    ((t_idx as f64 * (m - 1) as f64) / ((token_count - 1) as f64)).round() as usize;

                if t_idx > 0 {
                    let gap_width: usize = info.token_widths[last_slot + 1..slot]
                        .iter()
                        .map(|&w| w + 1)
                        .sum();
                    widths[t_idx - 1] += gap_width;
                }
                widths[t_idx] = info.token_widths[slot];
                last_slot = slot;
            }
            // Add remaining slots after the last token to the last token's width
            if last_slot < m - 1 {
                let gap_width: usize = info.token_widths[last_slot + 1..m]
                    .iter()
                    .map(|&w| w + 1)
                    .sum();
                widths[token_count - 1] += gap_width;
            }
        }
        widths
    } else {
        vec![1; token_count]
    }
}

// --- Main Entry Point ---

pub fn format_patterns(patterns: &[&ParsedLine]) -> String {
    if patterns.is_empty() {
        return String::new();
    }

    // Group: each Pattern with its following Modifier lines
    let mut groups: Vec<(&ParsedLine, Vec<&ParsedLine>)> = Vec::new();
    for p in patterns {
        match p {
            ParsedLine::Pattern { .. } => {
                groups.push((p, Vec::new()));
            }
            ParsedLine::Modifier { .. } => {
                if let Some(last) = groups.last_mut() {
                    last.1.push(p);
                }
            }
            _ => {}
        }
    }

    // Extract only patterns for sorting
    let pattern_only: Vec<&ParsedLine> = groups.iter().map(|(p, _)| *p).collect();
    let (sorted_patterns, sorted_keys) = sort_patterns(&pattern_only);
    let mut max_key_width = calculate_max_key_width(&sorted_keys);

    // Include modifier label widths in key width calculation
    for (_, mods) in &groups {
        for m in mods {
            if let ParsedLine::Modifier { kind, .. } = m {
                max_key_width = max_key_width.max(kind.to_string().len());
            }
        }
    }
    // Build pointer map from pattern to its modifier lines
    let group_map: std::collections::HashMap<*const ParsedLine, &Vec<&ParsedLine>> = groups
        .iter()
        .map(|(p, mods)| (*p as *const ParsedLine, mods))
        .collect();

    // --- Alignment Calculations ---

    let max_blocks = sorted_patterns
        .iter()
        .map(|p| match p {
            ParsedLine::Pattern { blocks, .. } => blocks.len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    let mut min_slot_widths = vec![1usize; max_blocks];
    let mut max_block_widths = vec![0usize; max_blocks];

    for (p, mods) in &groups {
        if let ParsedLine::Pattern {
            blocks: pattern_blocks,
            ..
        } = p
        {
            for (b_idx, pblock) in pattern_blocks.iter().enumerate() {
                if b_idx >= max_blocks {
                    continue;
                }

                // First, find msw for this block across all its modifiers
                for m in mods {
                    if let ParsedLine::Modifier { blocks, .. } = m {
                        if let Some(mblock) = blocks.get(b_idx) {
                            for (val_idx, val) in mblock.values.iter().enumerate() {
                                if let Some(v) = val {
                                    let is_group = pblock
                                        .tokens
                                        .get(val_idx)
                                        .map(|t| t.is_group())
                                        .unwrap_or(false);
                                    if !is_group {
                                        min_slot_widths[b_idx] =
                                            min_slot_widths[b_idx].max(modifier_value_width(v));
                                    }
                                }
                            }
                        }
                    }
                }

                // Now compute intrinsic widths using the final msw
                let msw = min_slot_widths[b_idx];
                let mut pw = 0;
                for token in &pblock.tokens {
                    pw += if token.is_group() {
                        token.to_string().len()
                    } else {
                        token.to_string().len().max(msw)
                    } + 1;
                }
                max_block_widths[b_idx] = max_block_widths[b_idx].max(pw);

                for m in mods {
                    if let ParsedLine::Modifier { blocks, .. } = m {
                        if let Some(mblock) = blocks.get(b_idx) {
                            let mut mw = 0;
                            for (val_idx, val) in mblock.values.iter().enumerate() {
                                let val_w = val.as_ref().map(modifier_value_width).unwrap_or(0);
                                let slot_w = if let Some(t) = pblock.tokens.get(val_idx) {
                                    if t.is_group() {
                                        t.to_string().len()
                                    } else {
                                        msw
                                    }
                                } else {
                                    msw
                                };
                                mw += slot_w.max(val_w) + 1;
                            }
                            max_block_widths[b_idx] = max_block_widths[b_idx].max(mw);
                        }
                    }
                }
            }
        }
    }

    let context = prepare_context(&sorted_patterns, &min_slot_widths);

    let mut out = String::new();

    for (i, p) in sorted_patterns.iter().enumerate() {
        if let ParsedLine::Pattern {
            blocks,
            end_bar,
            trailing_comment,
            ..
        } = p
        {
            let sorted_key = &sorted_keys[i];
            write!(out, "{:width$} ", sorted_key, width = max_key_width).unwrap();

            for (b_idx, block) in blocks.iter().enumerate() {
                format_block(&mut out, block, &context, b_idx).unwrap();
            }

            write!(out, "{}", end_bar).unwrap();
            if let Some(comment) = trailing_comment {
                write!(out, " > {}", comment).unwrap();
            }
            writeln!(out).unwrap();

            // Output modifier lines for this pattern
            if let Some(mods) = group_map.get(&(*p as *const ParsedLine)) {
                for m in mods.iter() {
                    if let ParsedLine::Modifier {
                        kind,
                        blocks: mod_blocks,
                        end_bar,
                    } = m
                    {
                        write!(out, "{:width$} ", kind, width = max_key_width).unwrap();

                        for (b_idx, mblock) in mod_blocks.iter().enumerate() {
                            let bw = context
                                .blocks
                                .get(b_idx)
                                .map(|info| info.bar_width)
                                .unwrap_or(1);

                            // Get pattern block tokens for Group detection
                            let pattern_tokens =
                                blocks.get(b_idx).map(|b| &b.tokens[..]).unwrap_or(&[]);

                            let num_slots = pattern_tokens.len().max(
                                context
                                    .blocks
                                    .get(b_idx)
                                    .map(|info| info.max_tokens)
                                    .unwrap_or(0),
                            );
                            let per_slot_widths = slot_widths(&context, b_idx, num_slots);

                            write!(out, "{:width$} ", mblock.start_bar.to_string(), width = bw)
                                .unwrap();

                            for slot_idx in 0..num_slots {
                                let base_sw = per_slot_widths.get(slot_idx).copied().unwrap_or(1);
                                let sw = if let Some(token) = pattern_tokens.get(slot_idx) {
                                    if token.is_group() {
                                        base_sw.max(token.to_string().len())
                                    } else {
                                        base_sw
                                    }
                                } else {
                                    base_sw
                                };
                                let val_s = if slot_idx < pattern_tokens.len() {
                                    mblock
                                        .values
                                        .get(slot_idx)
                                        .and_then(|v| v.as_ref())
                                        .map(modifier_value_str)
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                };
                                write!(out, "{:width$} ", val_s, width = sw).unwrap();
                            }
                        }

                        write!(out, "{}", end_bar).unwrap();
                        writeln!(out).unwrap();
                    }
                }
            }
        }
    }
    out
}

// --- Common Helpers ---

fn sort_patterns<'a>(patterns: &[&'a ParsedLine]) -> (Vec<&'a ParsedLine>, Vec<String>) {
    let mut patterns = patterns.to_vec();

    patterns.sort_by(|a, b| {
        let (key_a, key_b) = match (a, b) {
            (ParsedLine::Pattern { key: k1, .. }, ParsedLine::Pattern { key: k2, .. }) => (k1, k2),
            _ => return std::cmp::Ordering::Equal,
        };

        let parse_midi_max = |key: &str| -> Option<u8> {
            key.split(',')
                .filter_map(|s| crate::dsl::note::Note::from_str(s.trim()).ok())
                .map(|n| n.to_midi())
                .max()
        };

        match (parse_midi_max(key_a), parse_midi_max(key_b)) {
            (Some(max_a), Some(max_b)) => max_b.cmp(&max_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    let mut sorted_keys = Vec::new();
    for p in &patterns {
        if let ParsedLine::Pattern { key, .. } = p {
            let mut ns = key
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    crate::dsl::note::Note::from_str(trimmed)
                        .ok()
                        .map(|n| (n, trimmed.to_string()))
                })
                .collect::<Vec<_>>();

            if ns.is_empty() {
                sorted_keys.push(key.clone());
            } else {
                ns.sort_by(|(n1, _), (n2, _)| n1.to_midi().cmp(&n2.to_midi()));
                let sk = ns
                    .iter()
                    .map(|(n, _)| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                sorted_keys.push(sk);
            }
        }
    }
    (patterns, sorted_keys)
}

fn calculate_max_key_width(keys: &[String]) -> usize {
    keys.iter().map(|k| k.len()).max().unwrap_or(0)
}
