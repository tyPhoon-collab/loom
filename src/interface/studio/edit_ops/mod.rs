pub mod grid;
pub mod ops;
pub mod template;
pub mod transpose;

#[cfg(test)]
mod tests {
    use super::grid::{shrinkable_group_at_token, subdivided_unit};
    use super::template::{
        format_template_call_repeat, format_template_call_time_scale,
        format_template_call_transpose, parse_template_call_repeat, parse_template_call_time_scale,
        parse_template_call_transpose, template_call_text_with_repeat_delta,
        template_call_text_with_time_scale_delta, transposed_template_call_text,
    };
    use crate::interface::studio::selection::{replace_char_range, unit_spans_in_line};

    #[test]
    fn subdivided_unit_wraps_seq_note() {
        assert_eq!(subdivided_unit("C4"), "[C4 .]");
        assert_eq!(subdivided_unit("."), "[. .]");
        assert_eq!(subdivided_unit("-"), "[- .]");
    }

    #[test]
    fn subdivided_unit_replaces_seq_slot() {
        let mut line = "seq | C4 . |".to_string();
        let token = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        replace_char_range(
            &mut line,
            token.start_col,
            token.end_col,
            &subdivided_unit(&token.token),
        );
        assert_eq!(line, "seq | [C4 .] . |");
    }

    #[test]
    fn shrinkable_group_at_token_uses_selected_element() {
        let line = "seq | [C4 .] . |".to_string();
        let token = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        let group = shrinkable_group_at_token(&line, &token).unwrap();
        assert_eq!(group.selected_element, "C4");
    }

    #[test]
    fn shrinkable_group_at_token_uses_nearest_group_element() {
        let line = "seq | [[C4 .] [. .]] . |".to_string();
        let token = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        let group = shrinkable_group_at_token(&line, &token).unwrap();
        assert_eq!(group.selected_element, "C4");
        assert_eq!((group.start_col, group.end_col), (7, 13));
    }

    #[test]
    fn template_call_transpose_adds_new_param() {
        assert_eq!(
            transposed_template_call_text("[@riff arp]*2", 12).as_deref(),
            Some("[@riff +12 arp]*2")
        );
    }

    #[test]
    fn template_call_transpose_preserves_library_qualified_name() {
        assert_eq!(
            transposed_template_call_text("[@drums.fill arp]*2", 12).as_deref(),
            Some("[@drums.fill +12 arp]*2")
        );
    }

    #[test]
    fn template_call_transpose_updates_existing_param() {
        assert_eq!(
            transposed_template_call_text("[@riff +12 arp]", -1).as_deref(),
            Some("[@riff +11 arp]")
        );
    }

    #[test]
    fn template_call_transpose_removes_zero_param() {
        assert_eq!(
            transposed_template_call_text("[@riff +12 rev]", -12).as_deref(),
            Some("[@riff rev]")
        );
    }

    #[test]
    fn parse_and_format_template_call_transpose_roundtrip() {
        assert_eq!(parse_template_call_transpose("+12"), Some(12));
        assert_eq!(parse_template_call_transpose("-7"), Some(-7));
        assert_eq!(format_template_call_transpose(5), "+5");
        assert_eq!(format_template_call_transpose(-5), "-5");
    }

    #[test]
    fn template_call_repeat_adds_new_param() {
        assert_eq!(
            template_call_text_with_repeat_delta("[@riff arp]*2", 1).as_deref(),
            Some("[@riff x2 arp]*2")
        );
    }

    #[test]
    fn template_call_repeat_updates_existing_param() {
        assert_eq!(
            template_call_text_with_repeat_delta("[@riff +12 x2 arp]", 1).as_deref(),
            Some("[@riff +12 x3 arp]")
        );
    }

    #[test]
    fn template_call_repeat_removes_default_param() {
        assert_eq!(
            template_call_text_with_repeat_delta("[@riff x2 rev]", -1).as_deref(),
            Some("[@riff rev]")
        );
    }

    #[test]
    fn template_call_time_scale_adds_new_param() {
        assert_eq!(
            template_call_text_with_time_scale_delta("[@riff arp]*2", 1).as_deref(),
            Some("[@riff /2 arp]*2")
        );
    }

    #[test]
    fn template_call_time_scale_updates_existing_param() {
        assert_eq!(
            template_call_text_with_time_scale_delta("[@riff +12 /2 arp]", 1).as_deref(),
            Some("[@riff +12 /3 arp]")
        );
    }

    #[test]
    fn template_call_time_scale_removes_default_param() {
        assert_eq!(
            template_call_text_with_time_scale_delta("[@riff /2 rev]", -1).as_deref(),
            Some("[@riff rev]")
        );
    }

    #[test]
    fn parse_and_format_template_call_repeat_roundtrip() {
        assert_eq!(parse_template_call_repeat("x2"), Some(2));
        assert_eq!(format_template_call_repeat(3), "x3");
    }

    #[test]
    fn parse_and_format_template_call_time_scale_roundtrip() {
        assert_eq!(parse_template_call_time_scale("/2"), Some(2));
        assert_eq!(format_template_call_time_scale(3), "/3");
    }
}
