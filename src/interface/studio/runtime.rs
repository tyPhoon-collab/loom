use crossterm::event::{self, KeyEvent};
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

const FRAME_DURATION: Duration = Duration::from_millis(33);

pub(super) fn load_or_create_studio_file(path: &Path) -> Result<(String, bool)> {
    match fs::read_to_string(path) {
        Ok(content) => Ok((content, false)),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    miette::miette!(
                        "Failed to create parent directory for {}: {}",
                        path.display(),
                        e
                    )
                })?;
            }
            fs::write(path, "")
                .map_err(|e| miette::miette!("Failed to create {}: {}", path.display(), e))?;
            Ok((String::new(), true))
        }
        Err(err) => Err(miette::miette!(
            "Failed to read {}: {}",
            path.display(),
            err
        )),
    }
}

pub(super) fn poll_key_event() -> Result<Option<KeyEvent>> {
    if !event::poll(FRAME_DURATION).into_diagnostic()? {
        return Ok(None);
    }
    match event::read().into_diagnostic()? {
        event::Event::Key(key) => Ok(Some(key)),
        _ => Ok(None),
    }
}

pub(super) fn midi_device_name(port_index: usize) -> String {
    if let Ok(midi_out) = midir::MidiOutput::new("Loom Studio Info") {
        let ports = midi_out.ports();
        if let Some(port) = ports.get(port_index) {
            midi_out
                .port_name(port)
                .unwrap_or_else(|_| format!("Port {}", port_index))
        } else {
            format!("Port {} (Not Found)", port_index)
        }
    } else {
        format!("Port {}", port_index)
    }
}
