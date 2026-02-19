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

    fn prepare_context(&self, patterns: &[&ParsedLine]) -> Self::Context {
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

            // 1. Determine Max Tokens (Grid Size) and Bar Width
            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        max_bar_width = max_bar_width.max(block.start_bar.to_string().len());
                        max_tokens = max_tokens.max(block.tokens.len());
                    }
                }
            }

            // 2. Determine Width of each Slot
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
                            // If only 1 token, it goes to slot 0
                            token_widths[0] =
                                token_widths[0].max(block.tokens[0].to_string().len());
                        } else {
                            for (t_idx, t) in block.tokens.iter().enumerate() {
                                // Slot Calculation: round(t * (m-1) / (k-1))
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
        // If block index is out of bounds (e.g. pattern has fewer blocks than max), handle gracefully
        // The generic loop iterates `block.blocks`, so `block_index` is valid for `block`.
        // However, `prepare_context` calculated for MAX blocks.
        // If a line has FEWER blocks, we just process what we have.

        let info = &context.blocks[block_index];

        // Print Bar
        write!(
            buf,
            "{:width$} ",
            block.start_bar.to_string(),
            width = info.bar_width
        )?;

        // Map tokens to slots
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

        // Iterate Slots
        for (slot_idx, &slot_w) in info.token_widths.iter().enumerate() {
            if let Some(token) = token_map.get(&slot_idx) {
                write!(buf, "{:width$}", token.to_string(), width = slot_w)?;
            } else {
                write!(buf, "{:width$}", "", width = slot_w)?;
            }
            write!(buf, " ")?; // Space after slot
        }
        Ok(())
    }
}

pub fn format_patterns_equal(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, EqualFormatter)
}
