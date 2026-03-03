pub fn parse_u7(raw: &str) -> std::result::Result<u8, String> {
    let value = raw
        .parse::<u16>()
        .map_err(|_| format!("Invalid number '{}'", raw))?;
    if value > 127 {
        return Err(format!("Out of range '{}': expected 0..127", raw));
    }
    Ok(value as u8)
}

pub fn ensure_u7_i32(value: i32, label: &str) -> std::result::Result<u8, String> {
    if !(0..=127).contains(&value) {
        return Err(format!(
            "{} out of range: {} (expected 0..127)",
            label, value
        ));
    }
    Ok(value as u8)
}

pub fn ensure_channel_1_based(channel: u8) -> std::result::Result<u8, String> {
    if !(1..=16).contains(&channel) {
        return Err(format!(
            "Invalid MIDI channel: {} (expected 1..16)",
            channel
        ));
    }
    Ok(channel)
}

pub fn to_zero_based_channel(channel: u8) -> std::result::Result<u8, String> {
    let ch = ensure_channel_1_based(channel)?;
    Ok(ch - 1)
}
