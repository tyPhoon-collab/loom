use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Block;
use std::fmt::Write;

struct MinimizeFormatter;

impl PatternFormatter for MinimizeFormatter {
    type Context = Vec<usize>; // block_widths

    fn prepare_context(&self, patterns: &[&ParsedLine]) -> Self::Context {
        let max_blocks = patterns
            .iter()
            .map(|p| match p {
                ParsedLine::Pattern { blocks, .. } => blocks.len(),
                _ => 0,
            })
            .max()
            .unwrap_or(0);

        let mut block_widths = vec![0; max_blocks];
        for p in patterns {
            if let ParsedLine::Pattern { blocks, .. } = p {
                for (i, block) in blocks.iter().enumerate() {
                    let mut w = block.start_bar.to_string().len(); // Start bar length
                    w += 1; // initial space

                    let mut content_len = 0;
                    for (j, token) in block.tokens.iter().enumerate() {
                        if j > 0 {
                            content_len += 1;
                        }
                        content_len += token.to_string().len();
                    }
                    w += content_len;
                    w += 1; // trailing space

                    block_widths[i] = block_widths[i].max(w);
                }
            }
        }
        block_widths
    }

    fn format_block(
        &self,
        buf: &mut String,
        block: &Block,
        context: &Self::Context,
        block_index: usize,
    ) -> std::fmt::Result {
        let target_width = context[block_index];

        let mut block_str = String::new();
        write!(block_str, "{} ", block.start_bar)?;
        for (j, token) in block.tokens.iter().enumerate() {
            if j > 0 {
                block_str.push(' ');
            }
            block_str.push_str(&token.to_string());
        }
        block_str.push(' ');

        write!(buf, "{:width$}", block_str, width = target_width)?;
        Ok(())
    }
}

pub fn format_patterns_minimize(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, MinimizeFormatter)
}
