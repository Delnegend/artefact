#[derive(Debug, Clone, Copy)]
pub enum AdaptiveWidth {
    X8(u32),
    X16(u32),
    X32(u32),
    X64(u32),
}

/// Returns the indexes of the current row in the image, wrapped inside a
/// `GroupWidth` enum to indicate how many pixels can be processed at once.
pub fn get_adaptive_widths(max_rounded_px_w: u32) -> Vec<AdaptiveWidth> {
    let (mut tmp, mut idx) = (vec![], 0);
    loop {
        match idx {
            x if x + 64 <= max_rounded_px_w => {
                tmp.push(AdaptiveWidth::X64(idx));
                idx += 64;
            }
            x if x + 32 <= max_rounded_px_w => {
                tmp.push(AdaptiveWidth::X32(idx));
                idx += 32;
            }
            x if x + 16 <= max_rounded_px_w => {
                tmp.push(AdaptiveWidth::X16(idx));
                idx += 16;
            }
            x if x + 8 <= max_rounded_px_w => {
                tmp.push(AdaptiveWidth::X8(idx));
                idx += 8;
            }
            _ => break,
        }
    }
    tmp
}
