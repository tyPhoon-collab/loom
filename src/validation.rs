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

pub fn parse_signature(signature: &str) -> std::result::Result<(u16, u16), String> {
    let (num_raw, den_raw) = signature.split_once('/').ok_or_else(|| {
        format!(
            "Invalid signature '{}': expected <numerator>/<denominator>",
            signature
        )
    })?;
    let numerator = num_raw
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("Invalid signature numerator '{}'", num_raw.trim()))?;
    let denominator = den_raw
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("Invalid signature denominator '{}'", den_raw.trim()))?;

    if numerator == 0 {
        return Err("Signature numerator must be >= 1".to_string());
    }
    if denominator == 0 {
        return Err("Signature denominator must be >= 1".to_string());
    }
    if !denominator.is_power_of_two() {
        return Err(format!(
            "Signature denominator must be a power of two, got {}",
            denominator
        ));
    }
    Ok((numerator, denominator))
}

pub fn validate_unit(unit: &str) -> std::result::Result<(), String> {
    match unit.to_ascii_lowercase().as_str() {
        "bar" | "beat" => Ok(()),
        _ => Err(format!("Invalid unit '{}': expected 'bar' or 'beat'", unit)),
    }
}

pub fn beats_per_unit(unit: &str, signature: &str) -> std::result::Result<f64, String> {
    validate_unit(unit)?;
    let (numerator, _) = parse_signature(signature)?;
    match unit.to_ascii_lowercase().as_str() {
        "bar" => Ok(numerator as f64),
        "beat" => Ok(1.0),
        _ => unreachable!(),
    }
}

pub fn parse_loop_range_units(range_str: &str) -> std::result::Result<(f64, f64), String> {
    let parts: Vec<&str> = range_str.split('~').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid loop_range format. Expected 'start ~ end' (e.g. '1 ~ 4'), got '{}'",
            range_str
        ));
    }

    let start = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Invalid loop_range start '{}'", parts[0].trim()))?;
    let end = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Invalid loop_range end '{}'", parts[1].trim()))?;

    if !start.is_finite() || !end.is_finite() {
        return Err("loop_range start/end must be finite numbers".to_string());
    }
    if start < 1.0 {
        return Err(format!(
            "loop_range start must be >= 1 (1-based), got {}",
            start
        ));
    }
    if end < start {
        return Err(format!(
            "loop_range end must be >= start (start={}, end={})",
            start, end
        ));
    }
    Ok((start, end))
}
