use wide::f32x8;

/// Convert from 8x8 block to 64x1 block
pub fn unboxing(
    input: &[f32],
    output: &mut [f32],
    rounded_px_w: u32,
    rounded_px_h: u32,
    block_w: u32,
    block_h: u32,
) {
    assert_eq!(rounded_px_w % 8, 0);
    assert_eq!(rounded_px_h % 8, 0);
    assert_eq!(input.len(), output.len());

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                let result = f32x8::from(&input[index..index + 8]).to_array();

                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                output[row_start..row_start + 8].copy_from_slice(&result);
                index += 8;
            }
        }
    }
}

/// Convert from 64x1 block to 8x8 block
pub fn boxing(
    input: &[f32],
    output: &mut [f32],
    rounded_px_w: u32,
    rounded_px_h: u32,
    block_w: u32,
    block_h: u32,
) {
    assert_eq!(rounded_px_w % 8, 0);
    assert_eq!(rounded_px_h % 8, 0);
    assert_eq!(input.len(), output.len());

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                let result = f32x8::from(&input[row_start..row_start + 8]).to_array();
                output[index..index + 8].copy_from_slice(&result);
                index += 8;
            }
        }
    }
}
