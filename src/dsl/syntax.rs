use std::fmt;

macro_rules! define_symbols {
    (
        $(
            #[doc = $doc:expr]
            $name:ident => $val:expr
        ),* $(,)?
    ) => {
        /// Loom DSL を構成する記号・キーワードの列挙型
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Symbol {
            $($name),*
        }

        impl Symbol {
            /// 対応する文字列を返す
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$name => $val),*
                }
            }

            /// 対応する一文字を返す（一文字でない場合は panic）
            pub const fn as_char(&self) -> char {
                let s = self.as_str();
                if s.len() == 1 {
                    s.as_bytes()[0] as char
                } else {
                    panic!("Symbol is not a single character")
                }
            }
        }

        impl fmt::Display for Symbol {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl Symbol {
            /// nom の char パーサーを生成するヘルパー
            pub fn char<'a, E>(self) -> impl FnMut(&'a str) -> nom::IResult<&'a str, char, E>
            where
                E: nom::error::ParseError<&'a str>,
            {
                nom::character::complete::char(self.as_char())
            }

            /// nom の tag パーサーを生成するヘルパー
            pub fn tag<'a, E>(self) -> impl FnMut(&'a str) -> nom::IResult<&'a str, &'a str, E>
            where
                E: nom::error::ParseError<&'a str>,
            {
                nom::bytes::complete::tag(self.as_str())
            }
        }
    };
}

define_symbols! {
    #[doc = "Note glyph"]
    Note => "^",
    #[doc = "Rest glyph"]
    Rest => ".",
    #[doc = "Sustain (tie) glyph"]
    Sustain => "-",

    #[doc = "Standard bar line"]
    BarStandard => "|",
    #[doc = "Repeat start bar line"]
    BarRepeatStart => "|:",
    #[doc = "Repeat end bar line"]
    BarRepeatEnd => ":|",
    #[doc = "Double bar line / Section boundary"]
    BarDouble => ":|:",

    #[doc = "Track header start symbol"]
    TrackHeader => "#",
    #[doc = "Track header separator (name:channel)"]
    TrackHeaderSeparator => ":",
    #[doc = "Track header mute flag"]
    TrackHeaderMute => "x",

    #[doc = "Comment start symbol"]
    Comment => ">",
    #[doc = "Track wrap / Frontmatter boundary"]
    TrackWrap => "---",

    #[doc = "Group start"]
    GroupStart => "[",
    #[doc = "Group end"]
    GroupEnd => "]",

    #[doc = "Velocity modifier selector"]
    ModVelocity => "v",
    #[doc = "Pitch modifier selector"]
    ModPitch => "p",
    #[doc = "Modifier latch flag (!)"]
    ModLatch => "!",
    #[doc = "Modifier relative positive value (+)"]
    ModPositive => "+",
}
