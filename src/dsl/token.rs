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
            Token::Note => write!(f, "{}", Symbol::Note),
            Token::Rest => write!(f, "{}", Symbol::Rest),
            Token::Sustain => write!(f, "{}", Symbol::Sustain),
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
            Bar::Standard => Symbol::BarStandard.as_str(),
            Bar::RepeatStart => Symbol::BarRepeatStart.as_str(),
            Bar::RepeatEnd => Symbol::BarRepeatEnd.as_str(),
            Bar::Double => Symbol::BarDouble.as_str(),
        };
        write!(f, "{}", s)
    }
}

/// モディファイアのスロット（値、ラッチ、または空）
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierValue {
    Empty,
    Set(i32),
    Latch(i32),
    Group(Vec<ModifierValue>),
}

/// モディファイアの種類
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModifierKind {
    Velocity, // v
    Pitch,    // p
}

impl ModifierKind {
    pub fn as_char(&self) -> char {
        match self {
            ModifierKind::Velocity => Symbol::ModVelocity.as_char(),
            ModifierKind::Pitch => Symbol::ModPitch.as_char(),
        }
    }
}

impl std::fmt::Display for ModifierKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&self.as_char().to_string())
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
    pub values: Vec<ModifierValue>,
}

/// 1行分のモディファイア
#[derive(Debug, Clone)]
pub struct ModifierLine {
    pub kind: ModifierKind,
    pub blocks: Vec<ModifierBlock>,
    pub end_bar: Bar,
    pub trailing_comment: Option<String>,
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
    pub label: Option<String>,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateParam {
    Transpose(i32),        // +N / -N
    StructuralRepeat(u32), // xN
    TimeScale(u32),        // /N
    Macro(String),         // rev, etc.
}

impl std::fmt::Display for TemplateParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transpose(val) => {
                if *val >= 0 {
                    write!(f, "{}{}", Symbol::Positive, val)
                } else {
                    write!(f, "{}{}", Symbol::Negative, val.abs())
                }
            }
            Self::StructuralRepeat(val) => write!(f, "x{}", val),
            Self::TimeScale(val) => write!(f, "/{}", val),
            Self::Macro(m) => write!(f, "{}", m),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemplateCall {
    pub name: String,
    pub params: Vec<TemplateParam>,
    pub repeat: u32, // *N
}

impl std::fmt::Display for TemplateCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", Symbol::GroupStart, Symbol::Template, self.name)?;
        for param in &self.params {
            write!(f, "{}{}", Symbol::Separator, param)?;
        }
        write!(f, "{}", Symbol::GroupEnd)?;
        if self.repeat > 1 {
            write!(f, "*{}", self.repeat)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum LineEntry {
    Pattern(Line),
    TemplateCalls(Vec<TemplateCall>),
    TrackWrap,
}

#[derive(Debug, Clone)]
pub struct Sequence {
    pub entries: Vec<LineEntry>,
}

#[derive(Debug, Clone)]
pub struct TemplateDef {
    pub name: String,
    pub sequence: Sequence,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub muted: bool,
    pub init_events: Vec<TrackInitEvent>,
    pub sequence: Sequence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackInitEvent {
    ProgramChange { program: u8 },
    BankSelect { msb: u8, lsb: u8 },
    ControlChange { cc: u8, value: u8 },
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum SwingConfig {
    Detailed {
        grid: u8,
        #[serde(default = "default_swing_amount")]
        amount: u8,
    },
    Numeric(u8),
    Boolean(bool),
}

fn default_swing_amount() -> u8 {
    66
}

impl Default for SwingConfig {
    fn default() -> Self {
        SwingConfig::Numeric(0)
    }
}

impl SwingConfig {
    pub fn values(&self) -> Option<(u8, u8)> {
        match self {
            SwingConfig::Detailed { grid, amount } if *grid > 0 => Some((*grid, *amount)),
            SwingConfig::Numeric(grid) if *grid > 0 => Some((*grid, 66)),
            SwingConfig::Boolean(true) => Some((8, 66)),
            _ => None,
        }
    }
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
    #[serde(default)]
    pub swing: SwingConfig,
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
            swing: SwingConfig::default(),
            r#loop: false,
            loop_range: None,
        }
    }
}

#[derive(Debug)]
pub struct Song {
    pub metadata: Frontmatter,
    pub tracks: Vec<Track>,
    pub templates: std::collections::HashMap<String, TemplateDef>,
}

use super::syntax::Symbol;
