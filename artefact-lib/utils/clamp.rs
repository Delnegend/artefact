pub fn clamp(val: f32) -> u8 {
    match val {
        v if v < 0.0 => 0,
        v if v > 255.0 => 255,
        v => v as u8,
    }
}
