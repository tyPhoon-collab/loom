use crate::dsl::syntax::Symbol;
use crate::dsl::token::{
    Bar, Block, Frontmatter, ModifierBlock, ModifierKind, ModifierValue, Note, SwingConfig,
    TemplateCallTarget, TemplateMacro, Token, TrackInitEvent, TrackInitLabel,
};
use nom::error::{Error, ErrorKind};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{digit1, line_ending, not_line_ending, space0, space1},
    combinator::{eof, map, opt, recognize, value},
    multi::many0,
    sequence::{delimited, pair, preceded, terminated},
    IResult, Parser,
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum ParsedLine {
    Frontmatter(String),
    TrackHeader {
        name: String,
        channel: u8,
        solo: bool,
        muted: bool,
    },
    TrackReference {
        channel: u8,
    },
    FragmentCall {
        name: String,
    },
    TrackInit {
        event: TrackInitEvent,
        label: TrackInitLabel,
    },
    Pattern {
        key: String,
        notes: Vec<Note>,
        blocks: Vec<Block>,
        end_bar: Bar,
        trailing_comment: Option<String>,
    },
    Modifier {
        kind: ModifierKind,
        blocks: Vec<ModifierBlock>,
        end_bar: Bar,
        trailing_comment: Option<String>,
    },
    Comment(String),
    Empty,
    TrackWrap,
    TemplateHeader {
        name: String,
    },
    TemplateCalls(Vec<crate::dsl::token::TemplateCall>),
}

// --- Token Parsers ---

fn parse_token_note(input: &str) -> IResult<&str, Token> {
    value(Token::Note, Symbol::Note.char()).parse(input)
}

fn parse_token_rest(input: &str) -> IResult<&str, Token> {
    value(Token::Rest, Symbol::Rest.char()).parse(input)
}

fn parse_token_sustain(input: &str) -> IResult<&str, Token> {
    value(Token::Sustain, Symbol::Sustain.char()).parse(input)
}

fn parse_token_group(input: &str) -> IResult<&str, Token> {
    map(
        delimited(
            Symbol::GroupStart.char(),
            delimited(space0, many0(parse_token), space0),
            Symbol::GroupEnd.char(),
        ),
        Token::Group,
    )
    .parse(input)
}

fn parse_token(input: &str) -> IResult<&str, Token> {
    preceded(
        space0,
        alt((
            parse_token_note,
            parse_token_rest,
            parse_token_sustain,
            parse_token_group,
        )),
    )
    .parse(input)
}

fn parse_seq_note_literal(input: &str) -> IResult<&str, Token> {
    let (input, raw) = take_while1(|c: char| {
        !c.is_whitespace()
            && c != Symbol::GroupEnd.as_char()
            && c != Symbol::BarStandard.as_char()
            && c != ':'
    })
    .parse(input)?;

    let mut notes = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Err(nom::Err::Failure(Error::new(input, ErrorKind::Verify)));
        }
        let note = Note::from_str(trimmed)
            .map_err(|_| nom::Err::Failure(Error::new(input, ErrorKind::Verify)))?;
        notes.push(note);
    }

    if notes.is_empty() {
        return Err(nom::Err::Failure(Error::new(input, ErrorKind::Verify)));
    }

    Ok((input, Token::NoteLiteral(notes)))
}

fn parse_seq_token_group(input: &str) -> IResult<&str, Token> {
    map(
        delimited(
            Symbol::GroupStart.char(),
            delimited(space0, many0(parse_seq_token), space0),
            Symbol::GroupEnd.char(),
        ),
        Token::Group,
    )
    .parse(input)
}

fn parse_seq_token(input: &str) -> IResult<&str, Token> {
    preceded(
        space0,
        alt((
            parse_token_rest,
            parse_token_sustain,
            parse_seq_token_group,
            parse_seq_note_literal,
        )),
    )
    .parse(input)
}

fn parse_block_tokens_only(input: &str) -> IResult<&str, Vec<Token>> {
    terminated(many0(parse_token), space0).parse(input)
}

fn parse_seq_block_tokens_only(input: &str) -> IResult<&str, Vec<Token>> {
    terminated(many0(parse_seq_token), space0).parse(input)
}

// --- Bar Parser ---
fn parse_bar(input: &str) -> IResult<&str, Bar> {
    alt((
        value(Bar::Double, Symbol::BarDouble.tag()),
        value(Bar::RepeatEnd, Symbol::BarRepeatEnd.tag()),
        value(Bar::RepeatStart, Symbol::BarRepeatStart.tag()),
        value(Bar::Standard, Symbol::BarStandard.char()),
    ))
    .parse(input)
}

pub(crate) fn parse_line_blocks(input: &str) -> IResult<&str, (Vec<Block>, Bar)> {
    let (input, first_bar) = parse_bar(input)?;

    let mut blocks = Vec::new();
    let mut current_input = input;
    let mut current_bar = first_bar;

    loop {
        // Parse content (tokens)
        let (input_after_tokens, tokens) = parse_block_tokens_only(current_input)?;

        // Look for the next bar
        match parse_bar(input_after_tokens) {
            Ok((rest, next_bar)) => {
                blocks.push(Block {
                    start_bar: current_bar,
                    tokens,
                });
                current_input = rest;
                current_bar = next_bar;
            }
            Err(_) => {
                if !tokens.is_empty() {
                    use nom::error::{Error, ErrorKind};
                    return Err(nom::Err::Failure(Error::new(
                        input_after_tokens,
                        ErrorKind::Tag,
                    )));
                }
                break;
            }
        }
    }

    Ok((current_input, (blocks, current_bar)))
}

pub(crate) fn parse_seq_line_blocks(input: &str) -> IResult<&str, (Vec<Block>, Bar)> {
    let (input, first_bar) = parse_bar(input)?;

    let mut blocks = Vec::new();
    let mut current_input = input;
    let mut current_bar = first_bar;

    loop {
        let (input_after_tokens, tokens) = parse_seq_block_tokens_only(current_input)?;

        match parse_bar(input_after_tokens) {
            Ok((rest, next_bar)) => {
                blocks.push(Block {
                    start_bar: current_bar,
                    tokens,
                });
                current_input = rest;
                current_bar = next_bar;
            }
            Err(_) => {
                if !tokens.is_empty() {
                    return Err(nom::Err::Failure(Error::new(
                        input_after_tokens,
                        ErrorKind::Tag,
                    )));
                }
                break;
            }
        }
    }

    Ok((current_input, (blocks, current_bar)))
}

// --- Line Types ---

fn parse_comment(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = Symbol::Comment.char()(input)?;
    let (input, content) = not_line_ending(input)?;
    Ok((input, ParsedLine::Comment(content.trim().to_string())))
}

fn parse_track_header(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = Symbol::TrackHeader.char()(input)?;
    let (input, _) = space0(input)?;
    let (input, name) = take_while1(|c: char| c != Symbol::TrackHeaderSeparator.as_char())(input)?;
    let (input, _) = Symbol::TrackHeaderSeparator.char()(input)?;
    let (input, _) = space0(input)?;
    let (input, channel_str) = digit1(input)?;

    let channel = channel_str
        .parse::<u8>()
        .map_err(|_| nom::Err::Failure(Error::new(channel_str, ErrorKind::Digit)))?;
    let (input, flags) = many0(preceded(
        space1,
        alt((
            Symbol::TrackHeaderSolo.char(),
            Symbol::TrackHeaderMute.char(),
        )),
    ))
    .parse(input)?;
    let (input, _) = space0.parse(input)?;
    let (input, _) = eof.parse(input)?;
    let solo = flags.contains(&Symbol::TrackHeaderSolo.as_char());
    let muted = flags.contains(&Symbol::TrackHeaderMute.as_char());

    Ok((
        input,
        ParsedLine::TrackHeader {
            name: name.trim().to_string(),
            channel,
            solo,
            muted,
        },
    ))
}

fn parse_track_reference(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = Symbol::TrackHeader.char()(input)?;
    let (input, _) = space1(input)?;
    let (input, channel_str) = digit1(input)?;
    let channel = channel_str
        .parse::<u8>()
        .map_err(|_| nom::Err::Failure(Error::new(channel_str, ErrorKind::Digit)))?;
    let (input, _) = space0.parse(input)?;
    let (input, _) = eof.parse(input)?;
    Ok((input, ParsedLine::TrackReference { channel }))
}

fn parse_fragment_name(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while1(|c: char| c.is_ascii_alphanumeric()),
        many0(alt((
            take_while1(|c: char| c.is_ascii_alphanumeric()),
            tag("_"),
            tag("-"),
        ))),
    ))
    .parse(input)
}

fn parse_fragment_call(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = tag("[[")(input)?;
    let (input, name) = parse_fragment_name(input)?;
    let (input, _) = tag("]]")(input)?;
    let (input, _) = space0.parse(input)?;
    let (input, _) = eof.parse(input)?;
    Ok((
        input,
        ParsedLine::FragmentCall {
            name: name.to_string(),
        },
    ))
}

pub fn parse_track_init_command(
    command: &str,
) -> std::result::Result<(TrackInitEvent, TrackInitLabel), String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Missing init command after '##'".to_string());
    }
    let head = parts[0].to_ascii_lowercase();
    match head.as_str() {
        "pc" | "sound" => {
            if parts.len() != 2 {
                Err(format!("Usage: ## {} <0..127>", head))
            } else {
                let label = if head == "sound" {
                    TrackInitLabel::Sound
                } else {
                    TrackInitLabel::Pc
                };
                crate::validation::parse_u7(parts[1])
                    .map(|program| (TrackInitEvent::ProgramChange { program }, label))
            }
        }
        "bank" => {
            if parts.len() != 2 {
                Err("Usage: ## bank <msb>/<lsb>".to_string())
            } else if let Some((msb, lsb)) = parts[1].split_once('/') {
                match (
                    crate::validation::parse_u7(msb),
                    crate::validation::parse_u7(lsb),
                ) {
                    (Ok(msb), Ok(lsb)) => Ok((
                        TrackInitEvent::BankSelect { msb, lsb },
                        TrackInitLabel::Bank,
                    )),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            } else {
                Err("Usage: ## bank <msb>/<lsb>".to_string())
            }
        }
        "cc" => {
            if parts.len() != 3 {
                Err("Usage: ## cc <controller 0..127> <value 0..127>".to_string())
            } else {
                match (
                    crate::validation::parse_u7(parts[1]),
                    crate::validation::parse_u7(parts[2]),
                ) {
                    (Ok(cc), Ok(value)) => Ok((
                        TrackInitEvent::ControlChange { cc, value },
                        TrackInitLabel::Cc,
                    )),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            }
        }
        "pan" | "volume" | "expression" | "mod" | "sustain" => {
            if parts.len() != 2 {
                Err(format!("Usage: ## {} <0..127>", head))
            } else {
                let cc = match head.as_str() {
                    "pan" => 10,
                    "volume" => 7,
                    "expression" => 11,
                    "mod" => 1,
                    "sustain" => 64,
                    _ => unreachable!(),
                };
                let label = match head.as_str() {
                    "pan" => TrackInitLabel::Pan,
                    "volume" => TrackInitLabel::Volume,
                    "expression" => TrackInitLabel::Expression,
                    "mod" => TrackInitLabel::Mod,
                    "sustain" => TrackInitLabel::Sustain,
                    _ => unreachable!(),
                };
                crate::validation::parse_u7(parts[1])
                    .map(|value| (TrackInitEvent::ControlChange { cc, value }, label))
            }
        }
        _ => {
            let suggestions = [
                "bank",
                "pc",
                "sound",
                "cc",
                "pan",
                "volume",
                "expression",
                "mod",
                "sustain",
            ];
            let hint = suggestions
                .iter()
                .find(|s| s.starts_with(&head))
                .copied()
                .or_else(|| {
                    suggestions
                        .iter()
                        .find(|s| s.contains(&head) || head.contains(**s))
                        .copied()
                });
            match hint {
                Some(h) => Err(format!(
                    "Unknown init command '{}'. Did you mean '{}'?",
                    head, h
                )),
                None => Err(format!("Unknown init command '{}'", head)),
            }
        }
    }
}

fn parse_template_macro(param: &str) -> std::result::Result<TemplateMacro, String> {
    match param {
        "rev" => Ok(TemplateMacro::Rev),
        "arp" => Ok(TemplateMacro::Arp),
        "strum" => Ok(TemplateMacro::Strum),
        _ => {
            if let Some(raw) = param.strip_prefix("vel:") {
                let v = crate::validation::parse_u7(raw)?;
                return Ok(TemplateMacro::Vel(v));
            }
            if let Some(raw) = param.strip_prefix("pan:") {
                let v = crate::validation::parse_u7(raw)?;
                return Ok(TemplateMacro::Pan(v));
            }
            Err(format!("Unknown template macro '{}'", param))
        }
    }
}

fn parse_track_init_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = nom::bytes::complete::tag("##")(input)?;
    let (input, _) = space0(input)?;
    let (input, rest) = not_line_ending(input)?;
    let command = rest.trim();

    match parse_track_init_command(command) {
        Ok((event, label)) => Ok((input, ParsedLine::TrackInit { event, label })),
        Err(_) => Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        ))),
    }
}

fn parse_template_header(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = Symbol::TrackHeader.char()(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = Symbol::Template.char()(input)?;
    let (input, name) =
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')(input)?;

    Ok((
        input,
        ParsedLine::TemplateHeader {
            name: name.to_string(),
        },
    ))
}

fn parse_template(input: &str) -> IResult<&str, crate::dsl::token::TemplateCall> {
    let (input, _) = space0.parse(input)?;
    let (input, _) = Symbol::GroupStart.char()(input)?;
    let (input, _) = Symbol::Template.char()(input)?;
    let (input, first) =
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')(input)?;
    let (input, target) = if input.starts_with('.') {
        let (input, _) = nom::character::complete::char('.')(input)?;
        let (input, name) =
            take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')(input)?;
        if input.starts_with('.') {
            return Err(nom::Err::Failure(Error::new(input, ErrorKind::Verify)));
        }
        (
            input,
            TemplateCallTarget::Library {
                alias: first.to_string(),
                name: name.to_string(),
            },
        )
    } else {
        (
            input,
            TemplateCallTarget::Local {
                name: first.to_string(),
            },
        )
    };

    let mut params = Vec::new();
    let mut current_input = input;

    if !current_input.starts_with(Symbol::GroupEnd.as_char()) {
        let (rest, _) = space1.parse(current_input)?;
        current_input = rest;
    }

    while !current_input.starts_with(Symbol::GroupEnd.as_char()) {
        let (next_input, param_str) =
            take_while1(|c: char| c != Symbol::GroupEnd.as_char() && !c.is_whitespace())(
                current_input,
            )?;

        if param_str.starts_with(Symbol::Positive.as_char())
            || param_str.starts_with(Symbol::Negative.as_char())
        {
            let val = param_str
                .parse::<i32>()
                .map_err(|_| nom::Err::Failure(Error::new(next_input, ErrorKind::Digit)))?;
            params.push(crate::dsl::token::TemplateParam::Transpose(val));
        } else if let Some(stripped) = param_str.strip_prefix('x') {
            let val = stripped
                .parse::<u32>()
                .map_err(|_| nom::Err::Failure(Error::new(next_input, ErrorKind::Digit)))?;
            if val == 0 {
                return Err(nom::Err::Failure(Error::new(next_input, ErrorKind::Verify)));
            }
            params.push(crate::dsl::token::TemplateParam::StructuralRepeat(val));
        } else if let Some(stripped) = param_str.strip_prefix('/') {
            let val = stripped
                .parse::<u32>()
                .map_err(|_| nom::Err::Failure(Error::new(next_input, ErrorKind::Digit)))?;
            if val == 0 {
                return Err(nom::Err::Failure(Error::new(next_input, ErrorKind::Verify)));
            }
            params.push(crate::dsl::token::TemplateParam::TimeScale(val));
        } else {
            match parse_template_macro(param_str) {
                Ok(macro_kind) => {
                    params.push(crate::dsl::token::TemplateParam::Macro(macro_kind));
                }
                Err(_) => {
                    return Err(nom::Err::Failure(Error::new(next_input, ErrorKind::Verify)));
                }
            }
        }

        current_input = next_input;
        if current_input.starts_with(Symbol::GroupEnd.as_char()) {
            break;
        }
        let (rest, _) = space1.parse(current_input)?;
        current_input = rest;
    }

    let (input, _) = Symbol::GroupEnd.char()(current_input)?;
    let (input, _) = space0.parse(input)?;

    let mut final_input = input;
    let mut repeat = 1;

    let (input_after_space, _) = space0.parse(final_input)?;
    if input_after_space.starts_with('*') {
        let (rest, _) = nom::character::complete::char('*')(input_after_space)?;
        let (rest, digits) = digit1(rest)?;
        repeat = digits
            .parse::<u32>()
            .map_err(|_| nom::Err::Failure(Error::new(rest, ErrorKind::Digit)))?;
        if repeat == 0 {
            return Err(nom::Err::Failure(Error::new(rest, ErrorKind::Verify)));
        }
        final_input = rest;
    }

    Ok((
        final_input,
        crate::dsl::token::TemplateCall {
            target,
            params,
            repeat,
        },
    ))
}

fn parse_template_list(input: &str) -> IResult<&str, ParsedLine> {
    let (input, expansions) = nom::multi::many1(parse_template).parse(input)?;
    Ok((input, ParsedLine::TemplateCalls(expansions)))
}

pub fn parse_key(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c != Symbol::BarStandard.as_char() && c != '\n' && c != '\r').parse(input)
}

pub(crate) fn parse_pattern_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, key_raw) = parse_key(input)?;
    let (input, (blocks, end_bar)) = parse_line_blocks(input)?;

    // Check for trailing comment
    let (input, _) = space0(input)?;
    let (input, trailing_comment) =
        opt(preceded(Symbol::Comment.char(), not_line_ending)).parse(input)?;

    let mut notes = Vec::new();
    let mut valid_notes = true;
    for part in key_raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        match Note::from_str(trimmed) {
            Ok(note) => notes.push(note),
            Err(_) => {
                valid_notes = false;
                break;
            }
        }
    }

    if notes.is_empty() {
        valid_notes = false;
    }

    match valid_notes {
        true => Ok((
            input,
            ParsedLine::Pattern {
                key: key_raw.trim().to_string(),
                notes,
                blocks,
                end_bar,
                trailing_comment: trailing_comment.map(|s: &str| s.trim().to_string()),
            },
        )),
        false => {
            use nom::error::{Error, ErrorKind};
            Err(nom::Err::Failure(Error::new(input, ErrorKind::Verify)))
        }
    }
}

fn parse_seq_pattern_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = nom::bytes::complete::tag("seq")(input)?;
    let (input, _) = space1(input)?;
    let (input, (blocks, end_bar)) = parse_seq_line_blocks(input)?;

    let (input, _) = space0(input)?;
    let (input, trailing_comment) =
        opt(preceded(Symbol::Comment.char(), not_line_ending)).parse(input)?;

    Ok((
        input,
        ParsedLine::Pattern {
            key: "seq".to_string(),
            notes: Vec::new(),
            blocks,
            end_bar,
            trailing_comment: trailing_comment.map(|s: &str| s.trim().to_string()),
        },
    ))
}

fn parse_empty_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = space0.parse(input)?;
    let (input, _) = eof.parse(input)?;
    Ok((input, ParsedLine::Empty))
}

fn parse_track_wrap(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = Symbol::TrackWrap.tag().parse(input)?;
    let (input, _) = space0.parse(input)?;
    let (input, _) = eof.parse(input)?;
    Ok((input, ParsedLine::TrackWrap))
}

fn parse_modifier_group(input: &str) -> IResult<&str, ModifierValue> {
    let (input, _) = space0(input)?;
    let (input, _) = Symbol::GroupStart.char()(input)?;
    let (input, values) = parse_modifier_block_values(input)?;
    let (input, _) = Symbol::GroupEnd.char()(input)?;
    Ok((input, ModifierValue::Group(values)))
}

fn parse_modifier_empty(input: &str) -> IResult<&str, ModifierValue> {
    let (input, _) = space0(input)?;
    let (input, _) = Symbol::Rest.char()(input)?;
    Ok((input, ModifierValue::Empty))
}

fn parse_modifier_scalar(input: &str) -> IResult<&str, ModifierValue> {
    let (input, _) = space0(input)?;
    let input_trimmed = input.trim_start();
    if input_trimmed.is_empty()
        || input_trimmed.starts_with(Symbol::BarStandard.as_char())
        || input_trimmed.starts_with(Symbol::BarRepeatEnd.as_str())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (input, is_latch) = opt(Symbol::ModLatch.char()).parse(input)?;
    let (input, sign) =
        opt(alt((Symbol::Positive.char(), Symbol::Negative.char()))).parse(input)?;
    let (input, digits) = digit1.parse(input)?;

    let val: i32 = digits
        .parse()
        .map_err(|_| nom::Err::Failure(Error::new(input, ErrorKind::Digit)))?;
    let val = match sign {
        Some(s) if s == Symbol::Negative.as_char() => -val,
        _ => val,
    };

    Ok((
        input,
        if is_latch.is_some() {
            ModifierValue::Latch(val)
        } else {
            ModifierValue::Set(val)
        },
    ))
}

fn parse_signed_i32_no_latch(input: &str) -> IResult<&str, i32> {
    let (input, sign) =
        opt(alt((Symbol::Positive.char(), Symbol::Negative.char()))).parse(input)?;
    let (input, digits) = digit1.parse(input)?;
    let val: i32 = digits
        .parse()
        .map_err(|_| nom::Err::Failure(Error::new(input, ErrorKind::Digit)))?;
    let val = match sign {
        Some(s) if s == Symbol::Negative.as_char() => -val,
        _ => val,
    };
    Ok((input, val))
}

fn parse_modifier_note_list(input: &str) -> IResult<&str, ModifierValue> {
    let (mut input, _) = space0(input)?;
    let (rest, first) = parse_signed_i32_no_latch(input)?;
    input = rest;
    let (rest, _) = space0(input)?;
    input = rest;

    if !input.starts_with(',') {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let mut values = vec![first];
    while input.starts_with(',') {
        let (rest, _) = nom::character::complete::char(',')(input)?;
        let (rest, _) = space0(rest)?;
        let (rest, value) = parse_signed_i32_no_latch(rest)?;
        let (rest, _) = space0(rest)?;
        values.push(value);
        input = rest;
    }

    Ok((input, ModifierValue::NoteList(values)))
}

fn parse_modifier_value(input: &str) -> IResult<&str, ModifierValue> {
    alt((
        parse_modifier_group,
        parse_modifier_empty,
        parse_modifier_note_list,
        parse_modifier_scalar,
    ))
    .parse(input)
}

fn parse_modifier_block_values(input: &str) -> IResult<&str, Vec<ModifierValue>> {
    let mut values = Vec::new();
    let mut current = input;

    loop {
        let (rest, _) = space0.parse(current)?;
        if rest.is_empty()
            || rest.starts_with(Symbol::BarStandard.as_char())
            || rest.starts_with(Symbol::TrackHeaderSeparator.as_char())
        {
            current = rest;
            break;
        }
        match parse_modifier_value(rest) {
            Ok((rest2, val)) => {
                values.push(val);
                current = rest2;
            }
            Err(_) => {
                current = rest;
                break;
            }
        }
    }

    Ok((current, values))
}

fn parse_modifier_line_blocks(input: &str) -> IResult<&str, (Vec<ModifierBlock>, Bar)> {
    let (input, first_bar) = parse_bar(input)?;
    let mut blocks = Vec::new();
    let mut current_input = input;
    let mut current_bar = first_bar;

    loop {
        let (rest, values) = parse_modifier_block_values(current_input)?;

        match parse_bar(rest) {
            Ok((rest2, next_bar)) => {
                blocks.push(ModifierBlock {
                    start_bar: current_bar,
                    values,
                });
                current_input = rest2;
                current_bar = next_bar;
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok((current_input, (blocks, current_bar)))
}

fn parse_modifier_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, kind_char) =
        alt((Symbol::ModVelocity.char(), Symbol::ModPitch.char())).parse(input)?;
    let (input, _) = space0.parse(input)?;

    if !input.starts_with(Symbol::BarStandard.as_char())
        && !input.starts_with(Symbol::BarRepeatEnd.as_str())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let kind = if kind_char == Symbol::ModVelocity.as_char() {
        ModifierKind::Velocity
    } else {
        ModifierKind::Pitch
    };

    let (input, (blocks, end_bar)) = parse_modifier_line_blocks(input)?;

    let (input, _) = space0(input)?;
    let (input, trailing_comment) =
        opt(preceded(Symbol::Comment.char(), not_line_ending)).parse(input)?;

    Ok((
        input,
        ParsedLine::Modifier {
            kind,
            blocks,
            end_bar,
            trailing_comment: trailing_comment.map(|s: &str| s.trim().to_string()),
        },
    ))
}

pub fn parse_line_entry(input: &str) -> IResult<&str, ParsedLine> {
    alt((
        parse_comment,
        parse_fragment_call,
        parse_track_wrap,
        parse_track_init_line,
        parse_track_reference,
        parse_track_header,
        parse_template_header,
        parse_template_list,
        parse_empty_line,
        parse_modifier_line,
        parse_seq_pattern_line,
        parse_pattern_line,
    ))
    .parse(input)
}

pub(crate) fn parse_frontmatter(input: &str) -> IResult<&str, Frontmatter> {
    let (input, _) = Symbol::TrackWrap.tag().parse(input)?;
    let (input, _) = line_ending.parse(input)?;
    let (input, yaml_content) = take_until(Symbol::TrackWrap.as_str()).parse(input)?;
    let (input, _) = Symbol::TrackWrap.tag().parse(input)?;
    let (input, _) = opt(line_ending).parse(input)?;

    let fm: Frontmatter = serde_yaml::from_str(yaml_content).map_err(|_| {
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Fail))
    })?;

    Ok((input, fm))
}

pub(crate) fn validate_swing_config(swing: &SwingConfig) -> std::result::Result<(), String> {
    match swing {
        SwingConfig::Detailed { grid, amount } => {
            if *grid == 0 {
                return Err("Invalid swing.grid: expected > 0".to_string());
            }
            if !grid.is_power_of_two() {
                return Err(format!(
                    "Invalid swing.grid: {} (expected power of two like 8 or 16)",
                    grid
                ));
            }
            if !(1..=99).contains(amount) {
                return Err(format!("Invalid swing.amount: {} (expected 1..99)", amount));
            }
            Ok(())
        }
        SwingConfig::Numeric(grid) => {
            if *grid == 0 {
                return Ok(());
            }
            if !grid.is_power_of_two() {
                return Err(format!(
                    "Invalid swing value: {} (expected power of two like 8 or 16)",
                    grid
                ));
            }
            Ok(())
        }
        SwingConfig::Boolean(_) => Ok(()),
    }
}
