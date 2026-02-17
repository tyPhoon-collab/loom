use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Note,               // ^
    Rest,               // .
    Sustain,            // -
    Group(Vec<Token>),  // [...]
}

#[derive(Debug, Clone)]
pub struct Block {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub note: String, // e.g. "c3", "kick"
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub lines: Vec<Line>,
}

#[derive(Debug, Deserialize, Default)]
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
}

fn default_signature() -> String { "4/4".to_string() }
fn default_unit() -> String { "bar".to_string() }

#[derive(Debug)]
pub struct Song {
    pub metadata: Frontmatter,
    pub tracks: Vec<Track>,
}
