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

    #[error(transparent)]
    #[diagnostic(transparent)]
    ContextError(Box<ContextErrorData>),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ValidationError(Box<ValidationErrorData>),
}

#[derive(Error, Debug, Diagnostic)]
#[error("Context error: {msg}")]
#[diagnostic(code(loom::parser::context))]
pub struct ContextErrorData {
    #[source_code]
    pub src: NamedSource<String>,
    #[label("Here")]
    pub span: SourceSpan,
    pub msg: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Validation error: {msg}")]
#[diagnostic(code(loom::parser::validation))]
pub struct ValidationErrorData {
    #[source_code]
    pub src: NamedSource<String>,
    #[label("Invalid value")]
    pub span: SourceSpan,
    pub msg: String,
    #[help]
    pub help: Option<String>,
}

impl ParseError {
    pub fn from_nom(line: &str, full_source: &str, kind: String) -> Self {
        let offset = line.as_ptr() as usize - full_source.as_ptr() as usize;
        Self::NomError {
            src: NamedSource::new("input", full_source.to_string()),
            span: (offset, line.len()).into(),
            kind,
        }
    }

    pub fn from_yaml(error_input: &str, full_source: &str, msg: String) -> Self {
        let offset = error_input.as_ptr() as usize - full_source.as_ptr() as usize;
        Self::YamlError {
            src: NamedSource::new("input", full_source.to_string()),
            span: (offset, error_input.find('\n').unwrap_or(error_input.len())).into(),
            msg,
        }
    }

    pub fn from_context(line: &str, full_source: &str, msg: String) -> Self {
        let offset = line.as_ptr() as usize - full_source.as_ptr() as usize;
        Self::ContextError(Box::new(ContextErrorData {
            src: NamedSource::new("input", full_source.to_string()),
            span: (offset, line.len()).into(),
            msg,
        }))
    }

    pub fn from_validation(
        line: &str,
        full_source: &str,
        msg: String,
        help: Option<String>,
    ) -> Self {
        let offset = line.as_ptr() as usize - full_source.as_ptr() as usize;
        Self::ValidationError(Box::new(ValidationErrorData {
            src: NamedSource::new("input", full_source.to_string()),
            span: (offset, line.len()).into(),
            msg,
            help,
        }))
    }
}
