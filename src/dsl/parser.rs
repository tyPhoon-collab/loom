use super::error::ParseError;
use super::syntax::Symbol;
use super::token::{
    Bar, Block, Frontmatter, Line, ModifierBlock, ModifierKind, ModifierLine, ModifierValue, Note,
    Song, Token, Track,
};
#[derive(Debug, Clone)]
pub enum ParsedLine {
    Frontmatter(String),
    TrackHeader {
        name: String,
        channel: u8,
        muted: bool,
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
}
use miette::NamedSource;
use nom::{
    branch::alt,
    bytes::complete::{take_until, take_while1},
    character::complete::{digit1, line_ending, not_line_ending, space0},
    combinator::{eof, map, opt, value},
    multi::many0,
    sequence::{delimited, preceded, terminated},
    IResult,
};
use std::str::FromStr;

// Convert nom error to Miette Diagnostic
// This requires access to the full original source string to create NamedSource and spans.
// Since parsers return IResult, we handle conversion at the top level.

// --- Token Parsers ---

fn parse_token_note(input: &str) -> IResult<&str, Token> {
    value(Token::Note, Symbol::Note.char())(input)
}

fn parse_token_rest(input: &str) -> IResult<&str, Token> {
    value(Token::Rest, Symbol::Rest.char())(input)
}

fn parse_token_sustain(input: &str) -> IResult<&str, Token> {
    value(Token::Sustain, Symbol::Sustain.char())(input)
}

fn parse_token_group(input: &str) -> IResult<&str, Token> {
    map(
        delimited(
            Symbol::GroupStart.char(),
            delimited(space0, many0(parse_token), space0),
            Symbol::GroupEnd.char(),
        ),
        Token::Group,
    )(input)
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
    )(input)
}

// --- Block Parser ---
// function removed

fn parse_block_tokens_only(input: &str) -> IResult<&str, Vec<Token>> {
    terminated(many0(parse_token), space0)(input)
}

// --- Bar Parser ---
fn parse_bar(input: &str) -> IResult<&str, Bar> {
    alt((
        value(Bar::Double, Symbol::BarDouble.tag()),
        value(Bar::RepeatEnd, Symbol::BarRepeatEnd.tag()),
        value(Bar::RepeatStart, Symbol::BarRepeatStart.tag()),
        value(Bar::Standard, Symbol::BarStandard.char()),
    ))(input)
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
                // dbg!(&next_bar);
                blocks.push(Block {
                    start_bar: current_bar,
                    tokens,
                });
                current_input = rest;
                current_bar = next_bar;
            }
            Err(_) => {
                // In strict mode, if we have content, we MUST have a closing bar.
                // If tokens are not empty and we fail to parse a bar, it's an error.
                if !tokens.is_empty() {
                    use nom::error::{Error, ErrorKind};
                    return Err(nom::Err::Failure(Error::new(
                        input_after_tokens,
                        ErrorKind::Tag,
                    )));
                }
                // If tokens are empty, it means we are at the end of the line (or just whitespace).
                // The `current_bar` is the "end bar" of the previous block,
                // OR it is the final bar of the sequence.
                // E.g. `| A |`:
                // 1. `|` (Start)
                // 2. `A` -> `|` (Next). push Block(|, A). current=|.
                // 3. `Empty` -> Error(EOF). Break.
                // Return blocks=[Block(|, A)], end_bar=|.

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

    let channel = channel_str.parse::<u8>().unwrap_or(0); // 0 is invalid anyway
    if !(1..=16).contains(&channel) {
        use nom::error::{Error, ErrorKind};
        return Err(nom::Err::Failure(Error::new(input, ErrorKind::Verify)));
    }

    let (input, _) = space0(input)?;
    let (input, muted_flag) = opt(Symbol::TrackHeaderMute.char())(input)?;

    Ok((
        input,
        ParsedLine::TrackHeader {
            name: name.trim().to_string(),
            channel,
            muted: muted_flag.is_some(),
        },
    ))
}

pub(crate) fn parse_key(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c != Symbol::BarStandard.as_char() && c != '\n' && c != '\r')(input)
}

pub(crate) fn parse_pattern_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, key_raw) = parse_key(input)?;
    let (input, (blocks, end_bar)) = parse_line_blocks(input)?;

    // Check for trailing comment
    let (input, _) = space0(input)?;
    let (input, trailing_comment) = opt(preceded(Symbol::Comment.char(), not_line_ending))(input)?;

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
                trailing_comment: trailing_comment.map(|s| s.trim().to_string()),
            },
        )),
        false => {
            // For strict mode, we return an error here.
            use nom::error::{Error, ErrorKind};
            Err(nom::Err::Failure(Error::new(input, ErrorKind::Verify)))
        }
    }
}

fn parse_empty_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = space0(input)?;
    let (input, _) = eof(input)?;
    Ok((input, ParsedLine::Empty))
}

fn parse_track_wrap(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = Symbol::TrackWrap.tag()(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = eof(input)?;
    Ok((input, ParsedLine::TrackWrap))
}

fn parse_modifier_value(input: &str) -> IResult<&str, ModifierValue> {
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

    let (input, is_latch) = opt(Symbol::ModLatch.char())(input)?;
    let (input, sign) = opt(alt((Symbol::ModPositive.char(), Symbol::Sustain.char())))(input)?;
    let (input, digits) = digit1(input)?;

    let val: i32 = digits.parse().unwrap_or(0);
    let val = match sign {
        Some(s) if s == Symbol::Sustain.as_char() => -val,
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

fn parse_modifier_block_values(input: &str) -> IResult<&str, Vec<ModifierValue>> {
    let mut values = Vec::new();
    let mut current = input;

    loop {
        let (rest, _) = space0(current)?;
        // Check if we hit a bar or end
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
    // Parse modifier kind label: "v" or "p" followed by space and bar
    let (input, kind_char) = alt((Symbol::ModVelocity.char(), Symbol::ModPitch.char()))(input)?;
    let (input, _) = space0(input)?;

    // Must be followed by a bar
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

    // Check for trailing comment
    let (input, _) = space0(input)?;
    let (input, trailing_comment) = opt(preceded(Symbol::Comment.char(), not_line_ending))(input)?;

    Ok((
        input,
        ParsedLine::Modifier {
            kind,
            blocks,
            end_bar,
            trailing_comment: trailing_comment.map(|s| s.trim().to_string()),
        },
    ))
}

pub fn parse_line_entry(input: &str) -> IResult<&str, ParsedLine> {
    alt((
        parse_comment,
        parse_track_wrap,
        parse_track_header,
        parse_empty_line,
        parse_modifier_line,
        parse_pattern_line,
    ))(input)
}

// --- High Level ---

fn parse_frontmatter(input: &str) -> IResult<&str, Frontmatter> {
    let (input, _) = Symbol::TrackWrap.tag()(input)?;
    let (input, _) = line_ending(input)?;
    let (input, yaml_content) = take_until(Symbol::TrackWrap.as_str())(input)?;
    let (input, _) = Symbol::TrackWrap.tag()(input)?;
    let (input, _) = opt(line_ending)(input)?;

    let fm: Frontmatter = serde_yaml::from_str(yaml_content).map_err(|_| {
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Fail))
    })?;

    Ok((input, fm))
}

pub fn parse_song(source: String) -> Result<Song, ParseError> {
    let input = source.as_str();

    // Frontmatter
    let (input, metadata) = if input.starts_with("---") {
        match parse_frontmatter(input) {
            Ok(res) => res,
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let offset = e.input.as_ptr() as usize - source.as_ptr() as usize;
                return Err(ParseError::YamlError {
                    src: NamedSource::new("input", source.clone()), // Clone here
                    span: (offset, 10).into(),
                    msg: "Invalid Frontmatter YAML".to_string(),
                });
            }
            Err(_) => panic!("Incomplete input"),
        }
    } else {
        (input, Frontmatter::default())
    };

    let mut tracks: Vec<Track> = Vec::new();
    let mut current_track: Option<Track> = None;

    // Line by line parsing

    for line in input.lines() {
        let trimmed = line.trim();

        match parse_line_entry(trimmed) {
            Ok((_, parsed)) => match parsed {
                ParsedLine::TrackHeader {
                    name,
                    channel,
                    muted,
                } => {
                    if let Some(t) = current_track.take() {
                        tracks.push(t);
                    }
                    current_track = Some(Track {
                        name,
                        channel,
                        muted,
                        sections: vec![crate::dsl::token::Section {
                            label: None,
                            lines: Vec::new(),
                        }],
                    });
                }
                ParsedLine::TrackWrap => {
                    // Start a new section in the current track
                    if let Some(ref mut track) = current_track {
                        track.sections.push(crate::dsl::token::Section {
                            label: None,
                            lines: Vec::new(),
                        });
                    }
                }
                ParsedLine::Pattern {
                    notes,
                    blocks,
                    end_bar,
                    ..
                } => {
                    if let Some(ref mut track) = current_track {
                        if track.sections.is_empty() {
                            track.sections.push(crate::dsl::token::Section {
                                label: None,
                                lines: Vec::new(),
                            });
                        }
                        track.sections.last_mut().unwrap().lines.push(Line {
                            notes,
                            blocks,
                            end_bar,
                            modifiers: Vec::new(),
                        });
                    }
                }
                ParsedLine::Modifier {
                    kind,
                    blocks,
                    end_bar,
                    trailing_comment,
                } => {
                    // Bind to the last line in the current section
                    if let Some(ref mut track) = current_track {
                        if let Some(section) = track.sections.last_mut() {
                            if let Some(line) = section.lines.last_mut() {
                                line.modifiers.push(ModifierLine {
                                    kind,
                                    blocks,
                                    end_bar,
                                    trailing_comment,
                                });
                            }
                        }
                    }
                }
                ParsedLine::Comment(_) | ParsedLine::Empty | ParsedLine::Frontmatter(_) => {}
            },
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let offset = line.as_ptr() as usize - source.as_ptr() as usize;

                return Err(ParseError::NomError {
                    src: NamedSource::new("input", source.clone()),
                    span: (offset, line.len()).into(),
                    kind: format!("{:?}", e.code),
                });
            }
            _ => {}
        }
    }

    if let Some(t) = current_track {
        tracks.push(t);
    }

    Ok(Song { metadata, tracks })
}
