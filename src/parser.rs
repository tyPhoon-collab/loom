use crate::token::{Block, Frontmatter, Line, Song, Token, Track};
use miette::{Diagnostic, NamedSource, SourceSpan};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until, take_while, take_while1},
    character::complete::{alpha1, alphanumeric1, char, digit1, line_ending, multispace0, not_line_ending, space0, space1},
    combinator::{eof, map, map_res, opt, recognize, value, all_consuming},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};
use thiserror::Error;

// --- Errors ---

#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    #[error("Parse error: {kind}")]
    #[diagnostic(code(loom::parser::base))]
    NomError {
        #[source_code]
        src: NamedSource,
        #[label("Here")]
        span: SourceSpan,
        kind: String, // Debug string of nom error kind
    },

    #[error("YAML Frontmatter error")]
    #[diagnostic(code(loom::parser::frontmatter))]
    YamlError {
        #[source_code]
        src: NamedSource,
        #[label("YAML content")]
        span: SourceSpan,
        #[help]
        msg: String,
    },
}

// Convert nom error to Miette Diagnostic
// This requires access to the full original source string to create NamedSource and spans.
// Since parsers return IResult, we handle conversion at the top level.

// --- Token Parsers ---

fn parse_token_note(input: &str) -> IResult<&str, Token> {
    value(Token::Note, char('^'))(input)
}

fn parse_token_rest(input: &str) -> IResult<&str, Token> {
    value(Token::Rest, char('.'))(input)
}

fn parse_token_sustain(input: &str) -> IResult<&str, Token> {
    value(Token::Sustain, char('-'))(input)
}

fn parse_token_group(input: &str) -> IResult<&str, Token> {
    map(
        delimited(
            char('['),
            delimited(space0, many0(parse_token), space0),
            char(']'),
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
pub fn parse_block_content(input: &str) -> IResult<&str, Block> {
    map(
        terminated(
            many0(parse_token),
            space0 // Consume trailing
        ),
        |tokens| Block { tokens },
    )(input)
}

pub fn parse_line_blocks(input: &str) -> IResult<&str, Vec<Block>> {
    let (input, _) = char('|')(input)?;
    many1(
        terminated(
            parse_block_content,
            char('|')
        )
    )(input)
}

// --- Line Types ---

pub(crate) enum ParsedLine {
    TrackHeader { name: String, channel: u8 },
    Pattern { key: String, blocks: Vec<Block> },
    Comment,
    Empty,
}

fn parse_comment(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = char('>')(input)?;
    let (input, _) = not_line_ending(input)?; // consumes rest of line
    Ok((input, ParsedLine::Comment))
}

fn parse_track_header(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = char('#')(input)?;
    let (input, _) = space0(input)?;
    let (input, name) = take_while1(|c: char| c != ':')(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, channel_str) = digit1(input)?;

    let channel = channel_str.parse::<u8>().unwrap_or(1);

    Ok((input, ParsedLine::TrackHeader {
        name: name.trim().to_string(),
        channel
    }))
}

pub fn parse_key(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c != '|' && c != '\n' && c != '\r')(input)
}

pub fn parse_pattern_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, key_raw) = parse_key(input)?;
    let (input, blocks) = parse_line_blocks(input)?;

    Ok((input, ParsedLine::Pattern {
        key: key_raw.trim().to_string(),
        blocks
    }))
}

fn parse_empty_line(input: &str) -> IResult<&str, ParsedLine> {
    let (input, _) = space0(input)?;
    let (input, _) = eof(input)?;
    Ok((input, ParsedLine::Empty))
}

fn parse_line_entry(input: &str) -> IResult<&str, ParsedLine> {
    alt((
        parse_comment,
        parse_track_header,
        parse_empty_line,
        parse_pattern_line
    ))(input)
}

// --- High Level ---

fn parse_frontmatter(input: &str) -> IResult<&str, Frontmatter> {
    let (input, _) = tag("---")(input)?;
    let (input, _) = line_ending(input)?;
    let (input, yaml_content) = take_until("---")(input)?;
    let (input, _) = tag("---")(input)?;
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

    for (_line_idx, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        match parse_line_entry(trimmed) {
            Ok((_, parsed)) => {
                match parsed {
                   ParsedLine::TrackHeader { name, channel } => {
                       if let Some(t) = current_track.take() { tracks.push(t); }
                       current_track = Some(Track { name, channel, lines: Vec::new() });
                   }
                   ParsedLine::Pattern { key, blocks } => {
                       if let Some(ref mut t) = current_track {
                           t.lines.push(Line { note: key, blocks });
                       }
                   }
                   ParsedLine::Comment | ParsedLine::Empty => {}
                }
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let offset = line.as_ptr() as usize - source.as_ptr() as usize;

                return Err(ParseError::NomError {
                    src: NamedSource::new("input", source.clone()), // Clone here too
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
