#![allow(unused_assignments)]
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    #[error("Parse error: {kind}")]
    #[diagnostic(code(loom::parser::base))]
    NomError {
        #[source_code]
        src: NamedSource<String>,
        #[label("Here")]
        span: SourceSpan,
        kind: String,
    },

    #[error("YAML Frontmatter error")]
    #[diagnostic(code(loom::parser::frontmatter))]
    YamlError {
        #[source_code]
        src: NamedSource<String>,
        #[label("YAML content")]
        span: SourceSpan,
        #[help]
        msg: String,
    },
}
