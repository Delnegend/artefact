use crate::utils::auxiliary::AuxTraits;

pub trait Coef: AuxTraits {
    fn rounded_px_w(&self) -> u32;
    fn rounded_px_h(&self) -> u32;
    fn block_w(&self) -> u32;
    fn block_h(&self) -> u32;
    fn block_count(&self) -> u32;
    fn horiz_factor(&self) -> u32;
    fn vert_factor(&self) -> u32;

    /// Clamp one dequantized 8x8 block (64 values) into its quantized box.
    fn clamp_block(&self, block_idx: usize, data: &mut [f32]);
}
