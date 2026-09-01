use crate::utils::auxiliary::AuxTraits;

pub trait Coef: AuxTraits {
    fn rounded_px_w(&self) -> u32;
    fn rounded_px_h(&self) -> u32;
    fn block_w(&self) -> u32;
    fn block_h(&self) -> u32;
    fn block_count(&self) -> u32;
    fn horiz_factor(&self) -> u32;
    fn vert_factor(&self) -> u32;
}
