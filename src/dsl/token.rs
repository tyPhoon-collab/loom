pub use super::note::Note;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Note,              // ^
    Rest,              // .
    Sustain,           // -
    Group(Vec<Token>), // [...]
}

impl Token {
    pub fn is_group(&self) -> bool {
        matches!(self, Token::Group(_))
    }
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

#[derive(Debug, Clone, PartialEq)]
pub enum Bar {
    Standard,    // |
    RepeatStart, // |:
    RepeatEnd,   // :|
    Double,      // :|:
}

impl std::fmt::Display for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Bar::Standard => "|",
            Bar::RepeatStart => "|:",
            Bar::RepeatEnd => ":|",
            Bar::Double => ":|:",
        };
        write!(f, "{}", s)
    }
}

/// モディファイアの値（ラッチ or ワンショット）
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierValue {
    Set(i32),   // ワンショット: 100, +2, -1
    Latch(i32), // ラッチ: !80, !+2
}

/// モディファイアの種類
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModifierKind {
    Velocity, // v
    Pitch,    // p
}

impl std::fmt::Display for ModifierKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ModifierKind::Velocity => "v",
            ModifierKind::Pitch => "p",
        };
        f.pad(s)
    }
}

impl ModifierKind {
    pub fn default_value(&self) -> i32 {
        match self {
            ModifierKind::Velocity => 100,
            ModifierKind::Pitch => 0,
        }
    }
}

/// 1ブロック分のモディファイア値リスト
#[derive(Debug, Clone)]
pub struct ModifierBlock {
    pub start_bar: Bar,
    pub values: Vec<Option<ModifierValue>>,
}

/// 1行分のモディファイア
#[derive(Debug, Clone)]
pub struct ModifierLine {
    pub kind: ModifierKind,
    pub blocks: Vec<ModifierBlock>,
    pub end_bar: Bar,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub start_bar: Bar,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub notes: Vec<Note>,
    pub blocks: Vec<Block>,
    pub end_bar: Bar,
    pub modifiers: Vec<ModifierLine>,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub muted: bool,
    pub sections: Vec<Section>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Frontmatter {
    #[allow(dead_code)]
    #[serde(default = "default_bpm")]
    pub bpm: u32,
    #[serde(default)]
    pub pitch: i32,
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

fn default_bpm() -> u32 {
    120
}

fn default_signature() -> String {
    "4/4".to_string()
}
fn default_unit() -> String {
    "bar".to_string()
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            bpm: default_bpm(),
            pitch: 0,
            signature: default_signature(),
            unit: default_unit(),
            title: None,
            author: None,
            r#loop: false,
            loop_range: None,
        }
    }
}

#[derive(Debug)]
pub struct Song {
    pub metadata: Frontmatter,
    pub tracks: Vec<Track>,
}
