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
                for in_x in 0..8 {
                    output[((block_y * 8 + in_y) * rounded_px_w + (block_x * 8 + in_x)) as usize] =
                        input[index];

                    index += 1;
                }
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
                for in_x in 0..8 {
                    output[index] = input
                        [((block_y * 8 + in_y) * rounded_px_w + (block_x * 8 + in_x)) as usize];

                    index += 1;
                }
            }
        }
    }
}
