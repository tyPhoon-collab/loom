pub use super::note::Note;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Note,                   // ^
    Rest,                   // .
    Sustain,                // -
    Group(Vec<Token>),      // [...]
    NoteLiteral(Vec<Note>), // C4 or C4,E4
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
            Token::NoteLiteral(notes) => {
                let joined = notes
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                write!(f, "{}", joined)
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
    NoteList(Vec<i32>), // 100,80 (per-note values for a single token)
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
pub enum TemplateMacro {
    Rev,
    Arp,
    Strum,
    Vel(u8),
    Pan(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateParam {
    Transpose(i32),        // +N / -N
    StructuralRepeat(u32), // xN
    TimeScale(u32),        // /N
    Macro(TemplateMacro),
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
            Self::Macro(m) => match m {
                TemplateMacro::Rev => write!(f, "rev"),
                TemplateMacro::Arp => write!(f, "arp"),
                TemplateMacro::Strum => write!(f, "strum"),
                TemplateMacro::Vel(v) => write!(f, "vel:{}", v),
                TemplateMacro::Pan(v) => write!(f, "pan:{}", v),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum TemplateCallTarget {
    Local { name: String },
    Library { alias: String, name: String },
}

impl TemplateCallTarget {
    pub fn display_name(&self) -> String {
        match self {
            Self::Local { name } => name.clone(),
            Self::Library { alias, name } => format!("{}.{}", alias, name),
        }
    }
}

impl std::fmt::Display for TemplateCallTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone)]
pub struct TemplateCall {
    pub target: TemplateCallTarget,
    pub params: Vec<TemplateParam>,
    pub repeat: u32, // *N
}

impl std::fmt::Display for TemplateCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            Symbol::GroupStart,
            Symbol::Template,
            self.target
        )?;
        for param in &self.params {
            write!(f, " {}", param)?;
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
pub struct TemplateLibrary {
    pub source: String,
    pub templates: HashMap<String, TemplateDef>,
    pub libraries: HashMap<String, TemplateLibrary>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub solo: bool,
    pub muted: bool,
    pub init_events: Vec<TrackInitEvent>,
    pub sequence: Sequence,
}

#[derive(Debug, Clone)]
pub struct FragmentBlock {
    pub name: String,
    pub tracks: Vec<Track>,
    pub templates: HashMap<String, TemplateDef>,
    pub libraries: HashMap<String, TemplateLibrary>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackInitEvent {
    ProgramChange { program: u8 },
    BankSelect { msb: u8, lsb: u8 },
    ControlChange { cc: u8, value: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackInitLabel {
    Pc,
    Sound,
    Bank,
    Cc,
    Pan,
    Volume,
    Expression,
    Mod,
    Sustain,
}

impl std::fmt::Display for TrackInitLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TrackInitLabel::Pc => "pc",
            TrackInitLabel::Sound => "sound",
            TrackInitLabel::Bank => "bank",
            TrackInitLabel::Cc => "cc",
            TrackInitLabel::Pan => "pan",
            TrackInitLabel::Volume => "volume",
            TrackInitLabel::Expression => "expression",
            TrackInitLabel::Mod => "mod",
            TrackInitLabel::Sustain => "sustain",
        };
        write!(f, "{}", s)
    }
}

impl TrackInitEvent {
    pub fn format_with_label(&self, label: TrackInitLabel) -> String {
        match (label, self) {
            (TrackInitLabel::Pc, TrackInitEvent::ProgramChange { program })
            | (TrackInitLabel::Sound, TrackInitEvent::ProgramChange { program }) => {
                format!("{} {}", label, program)
            }
            (TrackInitLabel::Bank, TrackInitEvent::BankSelect { msb, lsb }) => {
                format!("{} {}/{}", label, msb, lsb)
            }
            (TrackInitLabel::Cc, TrackInitEvent::ControlChange { cc, value }) => {
                format!("{} {} {}", label, cc, value)
            }
            (TrackInitLabel::Pan, TrackInitEvent::ControlChange { cc, value }) => {
                debug_assert_eq!(*cc, 10);
                format!("{} {}", label, value)
            }
            (TrackInitLabel::Volume, TrackInitEvent::ControlChange { cc, value }) => {
                debug_assert_eq!(*cc, 7);
                format!("{} {}", label, value)
            }
            (TrackInitLabel::Expression, TrackInitEvent::ControlChange { cc, value }) => {
                debug_assert_eq!(*cc, 11);
                format!("{} {}", label, value)
            }
            (TrackInitLabel::Mod, TrackInitEvent::ControlChange { cc, value }) => {
                debug_assert_eq!(*cc, 1);
                format!("{} {}", label, value)
            }
            (TrackInitLabel::Sustain, TrackInitEvent::ControlChange { cc, value }) => {
                debug_assert_eq!(*cc, 64);
                format!("{} {}", label, value)
            }
            (_, TrackInitEvent::ProgramChange { program }) => format!("pc {}", program),
            (_, TrackInitEvent::BankSelect { msb, lsb }) => format!("bank {}/{}", msb, lsb),
            (_, TrackInitEvent::ControlChange { cc, value }) => format!("cc {} {}", cc, value),
        }
    }
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
    #[serde(default)]
    pub humanize: Humanize,
    #[serde(default)]
    pub fragments: HashMap<String, String>,
    #[serde(default)]
    pub templates: HashMap<String, String>,
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

fn default_humanize_timing() -> f64 {
    0.015
}

fn default_humanize_velocity() -> u16 {
    5
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Humanize {
    Bool(bool),
    Config(HumanizeConfig),
}

impl Default for Humanize {
    fn default() -> Self {
        Self::Bool(false)
    }
}

impl Humanize {
    pub fn values(&self) -> Option<HumanizeConfig> {
        match self {
            Self::Bool(false) => None,
            Self::Bool(true) => Some(HumanizeConfig::default()),
            Self::Config(config) => Some(config.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct HumanizeConfig {
    #[serde(default = "default_humanize_timing")]
    pub timing: f64,
    #[serde(default = "default_humanize_velocity")]
    pub velocity: u16,
    #[serde(default)]
    pub seed: u64,
}

impl Default for HumanizeConfig {
    fn default() -> Self {
        Self {
            timing: default_humanize_timing(),
            velocity: default_humanize_velocity(),
            seed: 0,
        }
    }
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
            humanize: Humanize::default(),
            fragments: HashMap::new(),
            templates: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Song {
    pub metadata: Frontmatter,
    pub tracks: Vec<Track>,
    pub templates: HashMap<String, TemplateDef>,
    pub libraries: HashMap<String, TemplateLibrary>,
    pub fragment_blocks: Vec<FragmentBlock>,
}

use super::syntax::Symbol;
