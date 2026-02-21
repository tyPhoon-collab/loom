use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Block;
use std::collections::HashMap;
use std::fmt::Write;

struct EqualBlockInfo {
    bar_width: usize,
    max_tokens: usize,
    token_widths: Vec<usize>,
}

struct EqualContext {
    blocks: Vec<EqualBlockInfo>,
}

struct EqualFormatter;

impl PatternFormatter for EqualFormatter {
    type Context = EqualContext;

    fn prepare_context(
        &self,
        patterns: &[&ParsedLine],
        min_slot_widths: &[usize],
    ) -> Self::Context {
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
                            token_widths[0] =
                                token_widths[0].max(block.tokens[0].to_string().len());
                        } else {
                            for (t_idx, t) in block.tokens.iter().enumerate() {
                                let slot = ((t_idx as f64 * (m - 1) as f64) / ((k - 1) as f64))
                                    .round() as usize;
                                if slot < max_tokens {
                                    token_widths[slot] =
                                        token_widths[slot].max(t.to_string().len());
                                }
                            }
                        }
                    }
                }
            }

            // Apply modifier min slot widths only to non-Group positions
            for (slot_idx, tw) in token_widths.iter_mut().enumerate() {
                // Check if this slot is a Group in any pattern
                let is_group_slot = patterns.iter().any(|p| {
                    if let ParsedLine::Pattern { blocks, .. } = p {
                        if let Some(block) = blocks.get(i) {
                            let k = block.tokens.len();
                            if k == 0 {
                                return false;
                            }
                            let m = max_tokens;
                            // Check which token maps to this slot
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

            block_infos.push(EqualBlockInfo {
                bar_width: max_bar_width,
                max_tokens,
                token_widths,
            });
        }

        EqualContext {
            blocks: block_infos,
        }
    }

    fn format_block(
        &self,
        buf: &mut String,
        block: &Block,
        context: &Self::Context,
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
                    let slot =
                        ((t_idx as f64 * (m - 1) as f64) / ((k - 1) as f64)).round() as usize;
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

    fn slot_widths(
        &self,
        context: &Self::Context,
        block_index: usize,
        _token_count: usize,
    ) -> Vec<usize> {
        context
            .blocks
            .get(block_index)
            .map(|info| info.token_widths.clone())
            .unwrap_or_default()
    }

    fn slot_count(&self, context: &Self::Context, block_index: usize) -> usize {
        context
            .blocks
            .get(block_index)
            .map(|info| info.max_tokens)
            .unwrap_or(0)
    }

    fn bar_width(&self, context: &Self::Context, block_index: usize) -> usize {
        context
            .blocks
            .get(block_index)
            .map(|info| info.bar_width)
            .unwrap_or(1)
    }
}

pub fn format_patterns_equal(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, EqualFormatter)
}
