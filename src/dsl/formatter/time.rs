use super::core::{format_patterns_generic, PatternFormatter};
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Block;
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

struct TimeBlockInfo {
    bar_width: usize,
    grid_size: usize,
    slot_widths: Vec<usize>,
}

struct TimeContext {
    blocks: Vec<TimeBlockInfo>,
}

struct TimeFormatter;

impl PatternFormatter for TimeFormatter {
    type Context = TimeContext;

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
            let mut counts = Vec::new();

            // 1. Collect Token Counts and Max Bar Width
            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        max_bar_width = max_bar_width.max(block.start_bar.to_string().len());
                        let k = block.tokens.len();
                        if k > 0 {
                            counts.push(k);
                        }
                    }
                }
            }

            // 2. Calculate LCM (Grid Size)
            let mut grid_size = 1;
            for &c in &counts {
                grid_size = lcm(grid_size, c);
            }
            if grid_size == 0 {
                grid_size = 1;
            }

            // 3. Determine Width of each Slot
            let mut slot_widths = vec![0; grid_size];

            for p in patterns {
                if let ParsedLine::Pattern { blocks, .. } = p {
                    if let Some(block) = blocks.get(i) {
                        let k = block.tokens.len();
                        if k == 0 {
                            continue;
                        }

                        let step = grid_size / k;
                        for (t_idx, token) in block.tokens.iter().enumerate() {
                            let slot = t_idx * step;
                            if slot < grid_size {
                                slot_widths[slot] = slot_widths[slot].max(token.to_string().len());
                            }
                        }
                    }
                }
            }

            // Enforce min width
            for w in &mut slot_widths {
                if *w == 0 {
                    *w = 1;
                }
            }

            block_infos.push(TimeBlockInfo {
                bar_width: max_bar_width,
                grid_size,
                slot_widths,
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

        // Print Bar
        write!(
            buf,
            "{:width$} ",
            block.start_bar.to_string(),
            width = info.bar_width
        )?;

        let k = block.tokens.len();
        let step = if k > 0 { info.grid_size / k } else { 0 };

        for slot in 0..info.grid_size {
            let mut printed = false;
            if k > 0 && slot % step == 0 {
                let t_idx = slot / step;
                if t_idx < k {
                    write!(
                        buf,
                        "{:width$}",
                        block.tokens[t_idx].to_string(),
                        width = info.slot_widths[slot]
                    )?;
                    printed = true;
                }
            }

            if !printed {
                write!(buf, "{:width$}", "", width = info.slot_widths[slot])?;
            }
            write!(buf, " ")?;
        }
        Ok(())
    }
}

pub fn format_patterns_time(patterns: &[&ParsedLine]) -> String {
    format_patterns_generic(patterns, TimeFormatter)
}
