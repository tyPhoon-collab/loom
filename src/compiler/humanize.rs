use crate::compiler::event::MidiEvent;
use crate::dsl::token::HumanizeConfig;

pub fn apply_humanize(events: &mut [MidiEvent], config: &HumanizeConfig) {
    if config.timing == 0.0 && config.velocity == 0 {
        return;
    }

    let mut note_idx = 0usize;
    for event in events.iter_mut() {
        let MidiEvent::Note {
            time,
            duration,
            channel,
            note,
            velocity,
        } = event
        else {
            continue;
        };

        let base = humanize_seed(
            config.seed,
            note_idx,
            *time,
            *duration,
            *channel,
            *note,
            *velocity,
        );
        if config.timing > 0.0 {
            let timing_delta = unit_noise(base ^ 0x9e37_79b9_7f4a_7c15) * config.timing;
            *time = round_humanized_time((*time + timing_delta).max(0.0));
        }
        if config.velocity > 0 {
            let velocity_delta = (unit_noise(base ^ 0xbf58_476d_1ce4_e5b9)
                * f64::from(config.velocity))
            .round() as i32;
            let humanized = i32::from(*velocity) + velocity_delta;
            *velocity = humanized.clamp(1, 127) as u8;
        }
        note_idx += 1;
    }
}

fn round_humanized_time(time: f64) -> f64 {
    (time * 1_000_000.0).round() / 1_000_000.0
}

fn humanize_seed(
    seed: u64,
    index: usize,
    time: f64,
    duration: f64,
    channel: u8,
    note: u8,
    velocity: u8,
) -> u64 {
    let mut state = seed ^ 0x6c8e_9cf5_7093_2bd5;
    state = mix_u64(state ^ index as u64);
    state = mix_u64(state ^ time.to_bits());
    state = mix_u64(state ^ duration.to_bits());
    state = mix_u64(state ^ u64::from(channel));
    state = mix_u64(state ^ (u64::from(note) << 8));
    mix_u64(state ^ (u64::from(velocity) << 16))
}

fn unit_noise(seed: u64) -> f64 {
    let value = mix_u64(seed);
    let normalized = (value >> 11) as f64 / ((1_u64 << 53) as f64);
    normalized * 2.0 - 1.0
}

fn mix_u64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
