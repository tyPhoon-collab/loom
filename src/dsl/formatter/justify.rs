use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Block;
use std::fmt::Write;

struct JustifyContext {
    target_widths: Vec<usize>,
    bar_widths: Vec<usize>,
    max_tokens: Vec<usize>,
    slot_widths_per_block: Vec<usize>,
}

struct JustifyFormatter;

impl PatternFormatter for JustifyFormatter {
    type Context = JustifyContext;

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

        let mut block_target_widths = vec![0; max_blocks];
        let mut max_bar_widths = vec![0; max_blocks];
        let mut max_tokens = vec![0usize; max_blocks];
        let mut slot_widths = vec![1usize; max_blocks];

        for i in 0..max_blocks {
            let mut max_min_width = 0;
            let mut max_bar_w = 0;
            let min_sw = min_slot_widths.get(i).copied().unwrap_or(1);

            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        max_bar_w = max_bar_w.max(block.start_bar.to_string().len());
                        max_tokens[i] = max_tokens[i].max(block.tokens.len());

                        if !block.tokens.is_empty() {
                            let total_token_len: usize =
                                block.tokens.iter().map(|t| t.to_string().len()).sum();
                            let gaps = block.tokens.len() - 1;
                            let min_w = total_token_len + gaps;
                            max_min_width = max_min_width.max(min_w);
                        }
                    }
                }
            }

            slot_widths[i] = slot_widths[i].max(min_sw);

            // If modifier values are wider, ensure target width accommodates them
            if min_sw > 1 && max_tokens[i] > 0 {
                let mod_min_width = max_tokens[i] * min_sw + (max_tokens[i] - 1);
                max_min_width = max_min_width.max(mod_min_width);
            }

            block_target_widths[i] = max_min_width;
            max_bar_widths[i] = max_bar_w;
        }

        JustifyContext {
            target_widths: block_target_widths,
            bar_widths: max_bar_widths,
            max_tokens,
            slot_widths_per_block: slot_widths,
        }
    }

    fn format_block(
        &self,
        buf: &mut String,
        block: &Block,
        context: &Self::Context,
        block_index: usize,
    ) -> std::fmt::Result {
        let target_w = context.target_widths[block_index];
        let bar_w = context.bar_widths[block_index];
        let sw = context.slot_widths_per_block[block_index];

        write!(
            buf,
            "{:width$} ",
            block.start_bar.to_string(),
            width = bar_w
        )?;

        let mut needs_trailing_space = true;

        if block.tokens.is_empty() {
            write!(buf, "{:width$}", "", width = target_w)?;
        } else if sw > 1 {
            // Slot-based formatting when modifier values require wider slots
            for token in &block.tokens {
                if token.is_group() {
                    write!(buf, "{}", token)?;
                } else {
                    write!(buf, "{:width$}", token.to_string(), width = sw)?;
                }
                write!(buf, " ")?;
            }
            // Pad remaining to match target width if needed
            let used: usize = block
                .tokens
                .iter()
                .map(|t| {
                    if t.is_group() {
                        t.to_string().len()
                    } else {
                        sw
                    }
                })
                .sum::<usize>()
                + block.tokens.len(); // +len for spaces
            if used < target_w + 1 {
                write!(buf, "{:width$}", "", width = target_w + 1 - used)?;
            }
            needs_trailing_space = false;
        } else {
            // Original justify: distribute tokens evenly
            let total_token_len: usize = block.tokens.iter().map(|t| t.to_string().len()).sum();
            let num_tokens = block.tokens.len();

            if num_tokens == 1 {
                write!(buf, "{}", block.tokens[0])?;
                if target_w > total_token_len {
                    write!(buf, "{:width$}", "", width = target_w - total_token_len)?;
                }
            } else {
                let available_space = target_w.saturating_sub(total_token_len);
                let num_gaps = num_tokens - 1;
                let base_gap = available_space / num_gaps;
                let mut remainder = available_space % num_gaps;

                for (t_idx, token) in block.tokens.iter().enumerate() {
                    if t_idx > 0 {
                        let gap = base_gap + if remainder > 0 { 1 } else { 0 };
                        remainder = remainder.saturating_sub(1);
                        write!(buf, "{:width$}", "", width = gap)?;
                    }
                    write!(buf, "{}", token)?;
                }
            }
        }
        if needs_trailing_space {
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
        let n = self.slot_count(context, block_index);
        let sw = context
            .slot_widths_per_block
            .get(block_index)
            .copied()
            .unwrap_or(1);
        vec![sw; n]
    }

    fn slot_count(&self, context: &Self::Context, block_index: usize) -> usize {
        context.max_tokens.get(block_index).copied().unwrap_or(0)
    }

    fn bar_width(&self, context: &Self::Context, block_index: usize) -> usize {
        context.bar_widths.get(block_index).copied().unwrap_or(1)
    }
}

pub fn format_patterns_justify(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, JustifyFormatter)
}
