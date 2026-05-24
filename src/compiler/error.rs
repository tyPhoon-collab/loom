use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CompileError {
    #[error("Invalid signature '{signature}': {reason}")]
    #[diagnostic(code(loom::compiler::invalid_signature))]
    InvalidSignature { signature: String, reason: String },

    #[error("Invalid MIDI channel {channel} in {context} (expected 1..16)")]
    #[diagnostic(code(loom::compiler::invalid_channel))]
    InvalidChannel { channel: u8, context: String },

    #[error("Template not found: '{template}' while compiling {context}")]
    #[diagnostic(code(loom::compiler::template_not_found))]
    TemplateNotFound { template: String, context: String },

    #[error("Circular template reference detected: {0}")]
    #[diagnostic(code(loom::compiler::circular_template_reference))]
    CircularTemplateReference(String),

    #[error(
        "Velocity out of range: {value} (expected 0..127) at track '{track}', {context}, block {block_index}, leaf {leaf_index}"
    )]
    #[diagnostic(code(loom::compiler::velocity_out_of_range))]
    VelocityOutOfRange {
        track: String,
        context: String,
        block_index: usize,
        leaf_index: usize,
        value: i32,
    },

    #[error(
        "MIDI note out of range: {value} (expected 0..127) for '{note}' at track '{track}', {context}, block {block_index}, leaf {leaf_index}"
    )]
    #[diagnostic(code(loom::compiler::note_out_of_range))]
    NoteOutOfRange {
        track: String,
        context: String,
        block_index: usize,
        leaf_index: usize,
        note: String,
        value: i32,
    },

    #[error(
        "Invalid note '{note}' at track '{track}', {context}, block {block_index}, leaf {leaf_index}: {reason}"
    )]
    #[diagnostic(code(loom::compiler::invalid_note))]
    InvalidNote {
        track: String,
        context: String,
        block_index: usize,
        leaf_index: usize,
        note: String,
        reason: String,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidModifierStructure(Box<InvalidModifierStructureData>),

    #[error("while compiling {context}")]
    #[diagnostic(code(loom::compiler::context))]
    Context {
        context: String,
        #[source]
        source: Box<CompileError>,
    },
}

pub type CompileResult<T> = std::result::Result<T, CompileError>;

#[derive(Error, Debug, Diagnostic)]
#[error(
    "Invalid modifier structure for '{modifier}' at track '{track}', {context}, block {block_index}, value path {value_path}: {reason}"
)]
#[diagnostic(code(loom::compiler::invalid_modifier_structure))]
pub struct InvalidModifierStructureData {
    pub track: String,
    pub context: String,
    pub modifier: String,
    pub block_index: usize,
    pub value_path: String,
    pub reason: String,
}

pub trait CompileContextExt<T> {
    fn with_compile_context(self, context: impl Into<String>) -> CompileResult<T>;
}

impl<T> CompileContextExt<T> for CompileResult<T> {
    fn with_compile_context(self, context: impl Into<String>) -> CompileResult<T> {
        self.map_err(|source| CompileError::Context {
            context: context.into(),
            source: Box::new(source),
        })
    }
}
