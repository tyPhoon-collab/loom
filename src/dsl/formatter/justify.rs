use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Block;
use std::fmt::Write;

struct JustifyContext {
    target_widths: Vec<usize>,
    bar_widths: Vec<usize>,
}

struct JustifyFormatter;

impl PatternFormatter for JustifyFormatter {
    type Context = JustifyContext;

    fn prepare_context(&self, patterns: &[&ParsedLine]) -> Self::Context {
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

        for i in 0..max_blocks {
            let mut max_min_width = 0;
            let mut max_bar_w = 0;

            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        max_bar_w = max_bar_w.max(block.start_bar.to_string().len());

                        if !block.tokens.is_empty() {
                            let total_token_len: usize =
                                block.tokens.iter().map(|t| t.to_string().len()).sum();
                            let gaps = block.tokens.len() - 1;
                            // Minimum width requires at least 1 space between tokens
                            let min_w = total_token_len + gaps;
                            max_min_width = max_min_width.max(min_w);
                        }
                    }
                }
            }
            block_target_widths[i] = max_min_width;
            max_bar_widths[i] = max_bar_w;
        }

        JustifyContext {
            target_widths: block_target_widths,
            bar_widths: max_bar_widths,
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

        // Print Bar
        write!(
            buf,
            "{:width$} ",
            block.start_bar.to_string(),
            width = bar_w
        )?;

        // Justify Content
        if block.tokens.is_empty() {
            // If this block is empty (0 tokens), just pad spaces.
            write!(buf, "{:width$}", "", width = target_w)?;
        } else {
            let total_token_len: usize = block.tokens.iter().map(|t| t.to_string().len()).sum();
            let num_tokens = block.tokens.len();

            if num_tokens == 1 {
                // Single token: Align Left
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
                        // Print Gap
                        let gap = base_gap + if remainder > 0 { 1 } else { 0 };
                        remainder = remainder.saturating_sub(1);
                        write!(buf, "{:width$}", "", width = gap)?;
                    }
                    write!(buf, "{}", token)?;
                }
            }
        }
        write!(buf, " ")?; // Space after block content
        Ok(())
    }
}

pub fn format_patterns_justify(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, JustifyFormatter)
}
