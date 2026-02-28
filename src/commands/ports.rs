use midir::MidiOutput;
use miette::{IntoDiagnostic, Result};

pub fn handle_ports() -> Result<()> {
    let midi_out = MidiOutput::new("Loom MIDI Output").into_diagnostic()?;
    let out_ports = midi_out.ports();

    if out_ports.is_empty() {
        println!("No MIDI output ports available.");
    } else {
        println!("Available MIDI output ports:");
        for (i, p) in out_ports.iter().enumerate() {
            let port_name = midi_out
                .port_name(p)
                .unwrap_or_else(|_| "Unknown".to_string());
            println!("  {}: {}", i, port_name);
        }
    }

    Ok(())
}
