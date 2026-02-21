use crate::dsl::parser::ParsedLine;
use crate::dsl::token::ModifierValue;
use std::fmt::Write;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormattingMode {
    Minimize,
    Justify,
    Equal,
    Time,
}

pub trait PatternFormatter {
    type Context;

    /// Prepare formatting context.
    /// `min_slot_widths[block_index]` is the minimum width each non-Group slot must have
    /// (driven by modifier value widths at non-Group positions).
    fn prepare_context(&self, patterns: &[&ParsedLine], min_slot_widths: &[usize])
        -> Self::Context;

    fn format_block(
        &self,
        buf: &mut String,
        block: &crate::dsl::token::Block,
        context: &Self::Context,
        block_index: usize,
    ) -> std::fmt::Result;

    /// Returns per-slot widths for modifier value output.
    /// `token_count` is the parent pattern's token count for this block.
    fn slot_widths(
        &self,
        context: &Self::Context,
        block_index: usize,
        token_count: usize,
    ) -> Vec<usize> {
        let _ = token_count;
        let n = self.slot_count(context, block_index);
        vec![1; n]
    }

    /// Returns the number of slots for the given block.
    fn slot_count(&self, context: &Self::Context, block_index: usize) -> usize {
        let _ = (context, block_index);
        0
    }

    /// Returns the bar width for the given block.
    fn bar_width(&self, context: &Self::Context, block_index: usize) -> usize {
        let _ = (context, block_index);
        1
    }
}

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

pub fn format_patterns_generic<F: PatternFormatter>(
    patterns: &[&ParsedLine],
    formatter: F,
) -> String {
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

    // Compute per-block-column min slot width from modifier values
    // EXCLUDING modifier values at Group token positions
    let max_blocks = sorted_patterns
        .iter()
        .map(|p| match p {
            ParsedLine::Pattern { blocks, .. } => blocks.len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    let mut min_slot_widths = vec![1usize; max_blocks];
    for (p, mods) in &groups {
        if let ParsedLine::Pattern {
            blocks: pattern_blocks,
            ..
        } = p
        {
            for m in mods {
                if let ParsedLine::Modifier { blocks, .. } = m {
                    for (b_idx, mblock) in blocks.iter().enumerate() {
                        if b_idx < max_blocks {
                            // Get pattern tokens for this block to check Group positions
                            let pattern_tokens = pattern_blocks
                                .get(b_idx)
                                .map(|b| &b.tokens[..])
                                .unwrap_or(&[]);

                            for (val_idx, val) in mblock.values.iter().enumerate() {
                                if let Some(v) = val {
                                    // Exclude Group positions from unified width
                                    let is_group = pattern_tokens
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
            }
        }
    }

    let context = formatter.prepare_context(&sorted_patterns, &min_slot_widths);

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
                formatter
                    .format_block(&mut out, block, &context, b_idx)
                    .unwrap();
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
                            let bw = formatter.bar_width(&context, b_idx);

                            // Get pattern block tokens for Group detection
                            let pattern_tokens =
                                blocks.get(b_idx).map(|b| &b.tokens[..]).unwrap_or(&[]);

                            let per_slot_widths =
                                formatter.slot_widths(&context, b_idx, pattern_tokens.len());
                            let num_slots = pattern_tokens.len();

                            write!(out, "{:width$} ", mblock.start_bar.to_string(), width = bw)
                                .unwrap();

                            for slot_idx in 0..num_slots {
                                // Use slot_widths from formatter, but for Group positions
                                // override with the Group's display width if it's larger
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
                                let val_s = mblock
                                    .values
                                    .get(slot_idx)
                                    .and_then(|v| v.as_ref())
                                    .map(modifier_value_str)
                                    .unwrap_or_default();
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

pub(crate) fn sort_patterns<'a>(patterns: &[&'a ParsedLine]) -> (Vec<&'a ParsedLine>, Vec<String>) {
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

pub(crate) fn calculate_max_key_width(keys: &[String]) -> usize {
    keys.iter().map(|k| k.len()).max().unwrap_or(0)
}
