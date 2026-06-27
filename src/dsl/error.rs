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

    pub fn with_source_name(self, name: impl AsRef<str>) -> Self {
        match self {
            Self::NomError { src, span, kind } => Self::NomError {
                src: NamedSource::new(name.as_ref(), src.inner().clone()),
                span,
                kind,
            },
            Self::YamlError { src, span, msg } => Self::YamlError {
                src: NamedSource::new(name.as_ref(), src.inner().clone()),
                span,
                msg,
            },
            Self::ContextError(data) => {
                let data = *data;
                Self::ContextError(Box::new(ContextErrorData {
                    src: NamedSource::new(name.as_ref(), data.src.inner().clone()),
                    span: data.span,
                    msg: data.msg,
                }))
            }
            Self::ValidationError(data) => {
                let data = *data;
                Self::ValidationError(Box::new(ValidationErrorData {
                    src: NamedSource::new(name.as_ref(), data.src.inner().clone()),
                    span: data.span,
                    msg: data.msg,
                    help: data.help,
                }))
            }
        }
    }

    pub fn with_source_name_if_default(self, name: impl AsRef<str>) -> Self {
        if self.source_name() == "input" {
            self.with_source_name(name)
        } else {
            self
        }
    }

    pub fn source_name(&self) -> &str {
        match self {
            Self::NomError { src, .. } | Self::YamlError { src, .. } => src.name(),
            Self::ContextError(data) => data.src.name(),
            Self::ValidationError(data) => data.src.name(),
        }
    }

    pub fn source_text(&self) -> &str {
        match self {
            Self::NomError { src, .. } | Self::YamlError { src, .. } => src.inner(),
            Self::ContextError(data) => data.src.inner(),
            Self::ValidationError(data) => data.src.inner(),
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            Self::NomError { span, .. } | Self::YamlError { span, .. } => *span,
            Self::ContextError(data) => data.span,
            Self::ValidationError(data) => data.span,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::NomError { kind, .. } => format!("Parse error: {}", kind),
            Self::YamlError { msg, .. } => format!("YAML Frontmatter error: {}", msg),
            Self::ContextError(data) => data.msg.clone(),
            Self::ValidationError(data) => data.msg.clone(),
        }
    }

    pub fn help(&self) -> Option<&str> {
        match self {
            Self::ValidationError(data) => data.help.as_deref(),
            _ => None,
        }
    }
}
