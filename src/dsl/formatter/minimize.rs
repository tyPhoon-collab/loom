use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Block;
use std::fmt::Write;

struct MinimizeContext {
    block_widths: Vec<usize>,
    bar_widths: Vec<usize>,
    max_tokens: Vec<usize>,
    min_slot_widths: Vec<usize>,
}

struct MinimizeFormatter;

impl PatternFormatter for MinimizeFormatter {
    type Context = MinimizeContext;

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

        let mut block_widths = vec![0; max_blocks];
        let mut bar_widths = vec![0usize; max_blocks];
        let mut max_tokens_per_block = vec![0usize; max_blocks];
        let mut stored_min_slot_widths = vec![1usize; max_blocks];

        for (i, msw) in stored_min_slot_widths.iter_mut().enumerate() {
            *msw = min_slot_widths.get(i).copied().unwrap_or(1);
        }

        for p in patterns {
            if let ParsedLine::Pattern { blocks, .. } = p {
                for (i, block) in blocks.iter().enumerate() {
                    bar_widths[i] = bar_widths[i].max(block.start_bar.to_string().len());
                    max_tokens_per_block[i] = max_tokens_per_block[i].max(block.tokens.len());

                    // Compute block content width
                    let mut w = block.start_bar.to_string().len();
                    w += 1; // initial space

                    let msw = stored_min_slot_widths[i];
                    let mut content_len = 0;
                    for (j, token) in block.tokens.iter().enumerate() {
                        if j > 0 {
                            content_len += 1; // separator
                        }
                        if token.is_group() {
                            // Groups always use their own width
                            content_len += token.to_string().len();
                        } else {
                            content_len += token.to_string().len().max(msw);
                        }
                    }
                    w += content_len;
                    w += 1; // trailing space

                    block_widths[i] = block_widths[i].max(w);
                }
            }
        }

        MinimizeContext {
            block_widths,
            bar_widths,
            max_tokens: max_tokens_per_block,
            min_slot_widths: stored_min_slot_widths,
        }
    }

    fn format_block(
        &self,
        buf: &mut String,
        block: &Block,
        context: &Self::Context,
        block_index: usize,
    ) -> std::fmt::Result {
        let target_width = context.block_widths[block_index];
        let msw = context.min_slot_widths[block_index];

        let mut block_str = String::new();
        write!(block_str, "{} ", block.start_bar)?;
        for (j, token) in block.tokens.iter().enumerate() {
            if j > 0 {
                block_str.push(' ');
            }
            if token.is_group() {
                // Groups use their own width
                block_str.push_str(&token.to_string());
            } else if msw > 1 {
                write!(block_str, "{:width$}", token.to_string(), width = msw)?;
            } else {
                block_str.push_str(&token.to_string());
            }
        }
        block_str.push(' ');

        write!(buf, "{:width$}", block_str, width = target_width)?;
        Ok(())
    }

    fn slot_widths(
        &self,
        context: &Self::Context,
        block_index: usize,
        _token_count: usize,
    ) -> Vec<usize> {
        // Caller needs to know per-slot: for modifier output we need
        // unified width for non-Group, group width for Group
        // But we don't store which slots are Groups in context.
        // Return uniform msw for all slots; the modifier output handles groups separately
        let n = self.slot_count(context, block_index);
        let sw = context
            .min_slot_widths
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

pub fn format_patterns_minimize(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, MinimizeFormatter)
}
