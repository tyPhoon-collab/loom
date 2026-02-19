use crate::dsl::parser::ParsedLine;
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

    fn prepare_context(&self, patterns: &[&ParsedLine]) -> Self::Context;

    fn format_block(
        &self,
        buf: &mut String,
        block: &crate::dsl::token::Block,
        context: &Self::Context,
        block_index: usize,
    ) -> std::fmt::Result;
}

pub fn format_patterns_generic<F: PatternFormatter>(
    patterns: &[&ParsedLine],
    formatter: F,
) -> String {
    if patterns.is_empty() {
        return String::new();
    }
    let (patterns, sorted_keys) = sort_patterns(patterns);
    let max_key_width = calculate_max_key_width(&sorted_keys);
    let context = formatter.prepare_context(&patterns);

    let mut out = String::new();

    // Loop through sorted patterns
    for (i, p) in patterns.iter().enumerate() {
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
                // Call specific formatter for content
                formatter
                    .format_block(&mut out, block, &context, b_idx)
                    .unwrap();
            }

            write!(out, "{}", end_bar).unwrap();
            if let Some(comment) = trailing_comment {
                write!(out, " > {}", comment).unwrap();
            }
            writeln!(out).unwrap();
        }
    }
    out
}

// --- Common Helpers exposed to submodules via super:: ---

pub(crate) fn sort_patterns<'a>(patterns: &[&'a ParsedLine]) -> (Vec<&'a ParsedLine>, Vec<String>) {
    let mut patterns = patterns.to_vec();

    // Sort by pitch
    patterns.sort_by(|a, b| {
        let (key_a, key_b) = match (a, b) {
            (ParsedLine::Pattern { key: k1, .. }, ParsedLine::Pattern { key: k2, .. }) => (k1, k2),
            _ => return std::cmp::Ordering::Equal,
        };

        // Cache optimization could be done here, but let's stick to logic first.
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
