use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::{Block, Token};
use std::fmt::Write;

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn lcm(a: usize, b: usize) -> usize {
    if a == 0 || b == 0 {
        return 0;
    }
    (a * b) / gcd(a, b)
}

/// Count the max group size among tokens (1 for non-groups)
fn line_subdivision(tokens: &[Token]) -> usize {
    let mut sub = 1;
    for t in tokens {
        if let Token::Group(inner) = t {
            sub = lcm(sub, inner.len());
        }
    }
    sub
}

struct TimeBlockInfo {
    bar_width: usize,
    grid_size: usize,
    slot_width: usize, // uniform W for all grid slots
    max_tokens: usize,
}

struct TimeContext {
    blocks: Vec<TimeBlockInfo>,
}

struct TimeFormatter;

impl PatternFormatter for TimeFormatter {
    type Context = TimeContext;

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
            let mut effective_counts = Vec::new();
            let mut max_tokens = 0;
            let min_sw = min_slot_widths.get(i).copied().unwrap_or(1);

            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        max_bar_width = max_bar_width.max(block.start_bar.to_string().len());
                        let k = block.tokens.len();
                        max_tokens = max_tokens.max(k);
                        if k > 0 {
                            let sub = line_subdivision(&block.tokens);
                            effective_counts.push(k * sub);
                        }
                    }
                }
            }

            let mut grid_size = 1;
            for &c in &effective_counts {
                grid_size = lcm(grid_size, c);
            }
            if grid_size == 0 {
                grid_size = 1;
            }

            // Compute uniform slot width W
            // W must satisfy:
            // 1. W >= 1 (min for single-char tokens)
            // 2. W >= min_sw (from non-Group modifier values)
            // 3. For each Group spanning S grid slots: group_display_width <= S*(W+1)-1
            let mut w = 1usize;
            w = w.max(min_sw);

            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        let k = block.tokens.len();
                        if k == 0 {
                            continue;
                        }
                        let top_step = grid_size / k;
                        for token in &block.tokens {
                            if token.is_group() {
                                let display_w = token.to_string().len();
                                // display_w <= S*(W+1)-1 → W >= ceil((display_w+1)/S) - 1
                                let s = top_step;
                                if s > 0 {
                                    let needed = (display_w + 1).div_ceil(s); // ceil((d+1)/s)
                                    w = w.max(needed.saturating_sub(1).max(1));
                                }
                            } else {
                                w = w.max(token.to_string().len());
                            }
                        }
                    }
                }
            }

            block_infos.push(TimeBlockInfo {
                bar_width: max_bar_width,
                grid_size,
                slot_width: w,
                max_tokens,
            });
        }

        TimeContext {
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
        let w = info.slot_width;

        write!(
            buf,
            "{:width$} ",
            block.start_bar.to_string(),
            width = info.bar_width
        )?;

        let k = block.tokens.len();
        if k == 0 {
            // Empty block: fill all grid slots
            for _ in 0..info.grid_size {
                write!(buf, "{:width$} ", "", width = w)?;
            }
            return Ok(());
        }

        let top_step = info.grid_size / k;
        let mut grid_pos = 0;

        for (t_idx, token) in block.tokens.iter().enumerate() {
            let start = t_idx * top_step;
            // Fill gap before this token
            while grid_pos < start {
                write!(buf, "{:width$} ", "", width = w)?;
                grid_pos += 1;
            }

            if token.is_group() {
                // Group spans top_step grid slots
                let total_chars = top_step * (w + 1); // including trailing spaces
                let content_width = total_chars - 1; // last slot has no trailing sep
                write!(buf, "{:width$} ", token.to_string(), width = content_width)?;
                grid_pos = start + top_step;
            } else {
                // Non-group: render at start, fill remaining span slots
                write!(buf, "{:width$} ", token.to_string(), width = w)?;
                grid_pos = start + 1;
                // Fill remaining slots in span
                while grid_pos < start + top_step {
                    write!(buf, "{:width$} ", "", width = w)?;
                    grid_pos += 1;
                }
            }
        }

        // Fill remaining grid slots
        while grid_pos < info.grid_size {
            write!(buf, "{:width$} ", "", width = w)?;
            grid_pos += 1;
        }

        Ok(())
    }

    fn slot_widths(
        &self,
        context: &Self::Context,
        block_index: usize,
        token_count: usize,
    ) -> Vec<usize> {
        if let Some(info) = context.blocks.get(block_index) {
            if token_count == 0 {
                return vec![];
            }
            let top_step = info.grid_size / token_count;
            let span_width = top_step * (info.slot_width + 1) - 1;
            vec![span_width; token_count]
        } else {
            vec![1; token_count]
        }
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

pub fn format_patterns_time(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, TimeFormatter)
}
