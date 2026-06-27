use crate::dsl::formatter;
use crate::playground::{
    compile_workspace_with_diagnostics, PlaygroundCompileOutput, PlaygroundDiagnostic,
    PlaygroundDiagnosticSeverity, PlaygroundWorkspace,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum WasmFormatOutput {
    Ok {
        source: String,
    },
    Err {
        diagnostics: Vec<PlaygroundDiagnostic>,
    },
}

#[wasm_bindgen(js_name = compileWorkspace)]
pub fn compile_workspace(input: &str) -> String {
    let output = match serde_json::from_str::<PlaygroundWorkspace>(input) {
        Ok(workspace) => compile_workspace_with_diagnostics(&workspace),
        Err(err) => PlaygroundCompileOutput::Err {
            diagnostics: vec![workspace_level_error(format!(
                "Invalid workspace JSON: {}",
                err
            ))],
        },
    };
    serde_json::to_string(&output).expect("Playground compile output should serialize")
}

#[wasm_bindgen(js_name = formatFile)]
pub fn format_file(source: &str) -> String {
    let output = match formatter::format_string(source) {
        Ok(source) => WasmFormatOutput::Ok { source },
        Err(err) => WasmFormatOutput::Err {
            diagnostics: vec![workspace_level_error(err.to_string())],
        },
    };
    serde_json::to_string(&output).expect("Playground format output should serialize")
}

fn workspace_level_error(message: String) -> PlaygroundDiagnostic {
    PlaygroundDiagnostic {
        path: None,
        line: None,
        column: None,
        byte_offset: None,
        length: 0,
        severity: PlaygroundDiagnosticSeverity::Error,
        message,
        help: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn compile_workspace_compiles_manifest_workspace() {
        let workspace = json!({
            "entry_path": "song.loom",
            "active_path": "song.loom",
            "files": {
                "song.loom": "---\nfragments:\n  intro: sections/intro.loom\n---\n\n# Lead: 1\n\n[[intro]]\n",
                "sections/intro.loom": "# 1\nC4 | ^ |\n"
            }
        });

        let output: Value =
            serde_json::from_str(&compile_workspace(&workspace.to_string())).unwrap();

        assert_eq!(output["status"], "ok");
        assert!(!output["events"].as_array().unwrap().is_empty());
        assert_eq!(output["metadata"]["bpm"], 120);
        assert_eq!(output["metadata"]["loop"], false);
        assert_eq!(output["metadata"]["unit"], "bar");
    }

    #[test]
    fn compile_workspace_reports_invalid_json() {
        let output: Value = serde_json::from_str(&compile_workspace("not json")).unwrap();

        assert_eq!(output["status"], "err");
        assert_eq!(output["diagnostics"][0]["severity"], "error");
        assert!(output["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid workspace JSON"));
    }

    #[test]
    fn format_file_formats_source() {
        let output: Value = serde_json::from_str(&format_file(
            "# Lead: 1\n## sound 26\n## bank 1/2\n\n## pan 64\nC4 |^-|\n",
        ))
        .unwrap();

        assert_eq!(output["status"], "ok");
        assert_eq!(
            output["source"],
            "# Lead: 1\n\n## sound 26\n## bank 1/2\n## pan 64\n\nC4 | ^ -  |\n"
        );
    }
}
