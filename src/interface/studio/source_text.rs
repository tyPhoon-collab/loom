pub(super) fn char_to_byte_index(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

pub(super) fn shifted_score_row(row: usize, before_start: usize, after_start: usize) -> usize {
    let delta = after_start as isize - before_start as isize;

    if row < before_start {
        row
    } else {
        row.saturating_add_signed(delta)
    }
}

pub(super) fn shifted_score_row_for_sources(
    row: usize,
    before_source: &str,
    after_source: &str,
    score_body_start_row: impl Fn(&str) -> Result<usize, String>,
) -> usize {
    let Ok(before_start) = score_body_start_row(before_source) else {
        return row;
    };
    let Ok(after_start) = score_body_start_row(after_source) else {
        return row;
    };
    shifted_score_row(row, before_start, after_start)
}

pub(super) fn slugify_template_name(input: &str) -> String {
    let mut slug = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        }
    }
    slug
}
