pub fn apply_swing_to_time(time: f64, swing: Option<(u8, u8)>) -> f64 {
    let (swing_grid, swing_amount) = match swing {
        Some((g, a)) => (g, a),
        None => return time,
    };
    if swing_grid == 0 || swing_amount == 50 {
        return time;
    }
    let grid = 4.0 / (swing_grid as f64);
    let pair_cycle = grid * 2.0;

    let time_with_eps = time + 1e-9;
    let cycle_start = (time_with_eps / pair_cycle).floor() * pair_cycle;
    let pos_in_cycle = time - cycle_start;

    let first_half_duration = pair_cycle * (swing_amount as f64) / 100.0;
    let first_half_ratio = first_half_duration / grid;

    if pos_in_cycle < grid - 1e-9 {
        cycle_start + pos_in_cycle * first_half_ratio
    } else {
        let second_half_duration = pair_cycle - first_half_duration;
        let second_half_ratio = second_half_duration / grid;
        let pos_in_second_half = pos_in_cycle - grid;
        cycle_start + first_half_duration + pos_in_second_half * second_half_ratio
    }
}
