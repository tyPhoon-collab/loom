use crate::compiler::{Compiler, MidiEvent};
use crate::dsl::error::ParseError;
use crate::dsl::parser;
use miette::{Report, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlaygroundWorkspace {
    pub entry_path: String,
    pub active_path: String,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaygroundDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaygroundDiagnostic {
    pub path: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub byte_offset: Option<usize>,
    pub length: usize,
    pub severity: PlaygroundDiagnosticSeverity,
    pub message: String,
    pub help: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaygroundMetadata {
    pub bpm: u32,
    pub signature: String,
    pub unit: String,
    #[serde(rename = "loop")]
    pub r#loop: bool,
    pub loop_range: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PlaygroundCompileOutput {
    Ok {
        events: Vec<MidiEvent>,
        metadata: PlaygroundMetadata,
    },
    Err {
        diagnostics: Vec<PlaygroundDiagnostic>,
    },
}

pub fn compile_workspace(workspace: &PlaygroundWorkspace) -> Result<Vec<MidiEvent>> {
    compile_workspace_result(workspace).map(|(events, _metadata)| events)
}

fn compile_workspace_result(
    workspace: &PlaygroundWorkspace,
) -> Result<(Vec<MidiEvent>, PlaygroundMetadata)> {
    let song = parser::parse_song_from_virtual_workspace(
        &workspace.entry_path,
        &workspace.active_path,
        &workspace.files,
    )?;
    let compiler = Compiler::new(&song)?;
    let events = compiler.compile(&song)?;
    let metadata = PlaygroundMetadata {
        bpm: song.metadata.bpm,
        signature: song.metadata.signature,
        unit: song.metadata.unit,
        r#loop: song.metadata.r#loop,
        loop_range: song.metadata.loop_range,
    };
    Ok((events, metadata))
}

pub fn compile_workspace_with_diagnostics(
    workspace: &PlaygroundWorkspace,
) -> PlaygroundCompileOutput {
    match compile_workspace_result(workspace) {
        Ok((events, metadata)) => PlaygroundCompileOutput::Ok { events, metadata },
        Err(err) => PlaygroundCompileOutput::Err {
            diagnostics: vec![diagnostic_from_report(&err)],
        },
    }
}

fn diagnostic_from_report(report: &Report) -> PlaygroundDiagnostic {
    if let Some(parse_error) = report.downcast_ref::<ParseError>() {
        return diagnostic_from_parse_error(parse_error);
    }

    PlaygroundDiagnostic {
        path: None,
        line: None,
        column: None,
        byte_offset: None,
        length: 0,
        severity: PlaygroundDiagnosticSeverity::Error,
        message: report.to_string(),
        help: None,
    }
}

fn diagnostic_from_parse_error(error: &ParseError) -> PlaygroundDiagnostic {
    let span = error.span();
    let (line, column) = line_column_for_offset(error.source_text(), span.offset());
    PlaygroundDiagnostic {
        path: Some(error.source_name().to_string()),
        line: Some(line),
        column: Some(column),
        byte_offset: Some(span.offset()),
        length: span.len(),
        severity: PlaygroundDiagnosticSeverity::Error,
        message: error.message(),
        help: error.help().map(str::to_string),
    }
}

fn line_column_for_offset(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut line_start = 0;
    for (index, byte) in source.bytes().enumerate() {
        if index >= offset {
            break;
        }
        if byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}
