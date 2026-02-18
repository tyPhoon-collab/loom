use midir::{MidiOutput, MidiOutputConnection};
use miette::{miette, IntoDiagnostic, Result};

pub fn get_midi_output(client_name: &str) -> Result<MidiOutput> {
    MidiOutput::new(client_name).into_diagnostic()
}

pub fn list_ports(midi_out: &MidiOutput) {
    let ports = midi_out.ports();
    println!("Available ports:");
    for (i, port) in ports.iter().enumerate() {
        println!("  {}: {}", i, midi_out.port_name(port).unwrap_or_default());
    }
}

pub fn connect_out(
    midi_out: MidiOutput,
    port_index: usize,
    conn_name: &str,
) -> Result<MidiOutputConnection> {
    let ports = midi_out.ports();
    let port = ports.get(port_index).ok_or_else(|| {
        miette!(
            "Port index {} is out of range. Run with a valid port index.",
            port_index
        )
    })?;

    midi_out
        .connect(port, conn_name)
        .map_err(|e| miette!("Connection error: {}", e))
}
