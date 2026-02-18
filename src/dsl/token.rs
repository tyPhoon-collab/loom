pub use super::note::Note;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Note,              // ^
    Rest,              // .
    Sustain,           // -
    Group(Vec<Token>), // [...]
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Note => write!(f, "^"),
            Token::Rest => write!(f, "."),
            Token::Sustain => write!(f, "-"),
            Token::Group(tokens) => {
                write!(f, "[")?;
                for (i, t) in tokens.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, "]")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub notes: Vec<Note>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub lines: Vec<Line>,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct Frontmatter {
    #[allow(dead_code)]
    pub bpm: u32,
    #[serde(default = "default_signature")]
    #[allow(dead_code)]
    pub signature: String,
    #[serde(default = "default_unit")]
    #[allow(dead_code)]
    pub unit: String,
    #[allow(dead_code)]
    pub title: Option<String>,
    #[allow(dead_code)]
    pub author: Option<String>,
    #[serde(default, rename = "loop")]
    pub r#loop: bool,
    #[allow(dead_code)]
    pub loop_range: Option<String>,
}

fn default_signature() -> String {
    "4/4".to_string()
}
fn default_unit() -> String {
    "bar".to_string()
}

#[derive(Debug)]
pub struct Song {
    pub metadata: Frontmatter,
    pub tracks: Vec<Track>,
}
