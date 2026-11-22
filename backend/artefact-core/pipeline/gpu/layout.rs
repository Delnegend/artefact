//! Checked GPU solver layout: buffer sizes, offsets and dispatch grids.
//!
//! Everything here is pure, so the device-limit rules are unit-testable without
//! a GPU. The solver allocates, binds and dispatches exactly what [`Layout`]
//! describes. WGSL indexes storage with `u32`, so every element count and
//! invocation index is kept in `u32` range here.

use crate::jpeg::Coefficient;

use super::GpuError;

/// State fields per channel buffer, each `count` floats long. Kernels name the
/// rest (`F_OBJ` .. `F_HYY`) in `shaders/solver.wgsl`.
pub(super) const F_FDATA: u32 = 0;
pub(super) const F_FISTA: u32 = 1;
pub(super) const F_COS: u32 = 8;
const CHANNEL_FIELDS: u32 = 9;
/// Solver channels (Y, Cb, Cr), one storage buffer each.
pub(super) const MAX_CHANNELS: usize = 3;
/// Invocations per workgroup of the per-pixel kernels.
const PIXEL_WORKGROUP: u32 = 256;
/// Invocations per workgroup of the per-block kernels (one 8x8 block each).
const BLOCK_WORKGROUP: u32 = 64;
/// Cap on first-stage L2 reduction workgroups; `reduce1` strides over the rest.
const MAX_REDUCE_GROUPS: u32 = 1024;

/// The device limits the solver's layout depends on.
#[derive(Clone, Copy, Debug)]
pub(super) struct DeviceLimits {
    pub(super) buffer_size: u64,
    pub(super) storage_binding_size: u64,
    pub(super) workgroups_per_dimension: u32,
}

impl From<&wgpu::Limits> for DeviceLimits {
    fn from(limits: &wgpu::Limits) -> Self {
        Self {
            buffer_size: limits.max_buffer_size,
            storage_binding_size: limits.max_storage_buffer_binding_size,
            workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
        }
    }
}

/// A 2D workgroup grid. Kernels flatten it as `wid.y * num_workgroups.x + wid.x`,
/// so work larger than one dispatch dimension folds into rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Grid {
    pub(super) x: u32,
    pub(super) y: u32,
}

impl Grid {
    fn new(
        what: &str,
        groups: u32,
        group_size: u32,
        limits: &DeviceLimits,
    ) -> Result<Self, GpuError> {
        let max = limits.workgroups_per_dimension;
        let x = groups.min(max);
        if x == 0 {
            return Err(invalid(&format!("{what} has no work to dispatch")));
        }
        let y = groups.div_ceil(x);
        if y > max {
            return Err(GpuError::Unsupported(format!(
                "{what} need {groups} workgroups; the device allows {max} x {max}"
            )));
        }
        // Every invocation index (`group * group_size + lane`) must fit `u32`.
        x.checked_mul(y)
            .and_then(|g| g.checked_mul(group_size))
            .ok_or_else(|| GpuError::Unsupported(format!("{what} exceed u32 invocation ids")))?;
        Ok(Self { x, y })
    }

    fn for_pixels(what: &str, invocations: u32, limits: &DeviceLimits) -> Result<Self, GpuError> {
        Self::new(
            what,
            invocations.div_ceil(PIXEL_WORKGROUP),
            PIXEL_WORKGROUP,
            limits,
        )
    }
}

/// Geometry of one JPEG component.
#[derive(Clone, Copy, Debug)]
pub(super) struct ComponentShape {
    pub(super) rounded_w: u32,
    pub(super) rounded_h: u32,
    pub(super) block_w: u32,
    pub(super) block_h: u32,
    pub(super) block_count: u32,
    pub(super) h_factor: u32,
    pub(super) v_factor: u32,
}

impl ComponentShape {
    /// Read a component's geometry, rejecting coefficient data that does not
    /// match its block grid.
    pub(super) fn of(coef: &Coefficient) -> Result<Self, GpuError> {
        let shape = Self {
            rounded_w: coef.rounded_px_w,
            rounded_h: coef.rounded_px_h,
            block_w: coef.block_w,
            block_h: coef.block_h,
            block_count: coef.block_count,
            h_factor: coef.horizontal_samp_factor.u32(),
            v_factor: coef.vertical_samp_factor.u32(),
        };
        if usize::try_from(shape.coef_len()?).ok() != Some(coef.dct_coefs.len()) {
            return Err(invalid("coefficient count does not match the block grid"));
        }
        Ok(shape)
    }

    /// Component pixels; equal to its coefficient count since blocks tile it.
    pub(super) fn pixels(&self) -> Result<u32, GpuError> {
        self.rounded_w
            .checked_mul(self.rounded_h)
            .ok_or_else(|| invalid("component pixel count overflows u32"))
    }

    fn coef_len(&self) -> Result<u32, GpuError> {
        self.block_count
            .checked_mul(64)
            .ok_or_else(|| invalid("coefficient count overflows u32"))
    }

    /// Check the block grid tiles the component and fits a `count`-pixel image.
    fn validate(&self, count: u32) -> Result<u32, GpuError> {
        let pixels = self.pixels()?;
        let tiles = self.block_w.checked_mul(8) == Some(self.rounded_w)
            && self.block_h.checked_mul(8) == Some(self.rounded_h)
            && self.block_w.checked_mul(self.block_h) == Some(self.block_count);
        if !tiles || self.h_factor == 0 || self.v_factor == 0 {
            return Err(invalid("inconsistent component block grid"));
        }
        // Component rasters share `count`-sized slots in the channel and aux buffers.
        if pixels == 0 || pixels > count {
            return Err(invalid("component does not fit the image grid"));
        }
        Ok(pixels)
    }
}

/// Float offsets of the regions packed into the shared `aux` buffer.
#[derive(Clone, Copy, Debug)]
pub(super) struct AuxLayout {
    /// Per-pixel TV/TGV norm (`count`).
    pub(super) a_norm: u32,
    /// Per-channel downsampled component (`nchannel * count`).
    pub(super) a_mean: u32,
    /// All channels' quantised coefficients.
    pub(super) a_coefs: u32,
    /// Per-channel quantisation tables (`nchannel * 64`).
    pub(super) a_quant: u32,
    /// Per-channel first-stage L2 partial sums (`nchannel * reduce_groups`).
    pub(super) a_partials: u32,
    /// Per-channel L2 norms (`nchannel`).
    pub(super) a_l2: u32,
    pub(super) len: u32,
}

impl AuxLayout {
    fn new(
        count: u32,
        nchannel: u32,
        coefs_len: u32,
        reduce_groups: u32,
    ) -> Result<Self, GpuError> {
        let after = |start: u32, len: Option<u32>| {
            len.and_then(|l| start.checked_add(l)).ok_or_else(|| {
                GpuError::Unsupported("auxiliary buffer exceeds u32 indexing".into())
            })
        };
        let a_norm = 0;
        let a_mean = count;
        let a_coefs = after(a_mean, nchannel.checked_mul(count))?;
        let a_quant = after(a_coefs, Some(coefs_len))?;
        let a_partials = after(a_quant, nchannel.checked_mul(64))?;
        let a_l2 = after(a_partials, reduce_groups.checked_mul(nchannel))?;
        let len = after(a_l2, Some(nchannel))?;
        Ok(Self {
            a_norm,
            a_mean,
            a_coefs,
            a_quant,
            a_partials,
            a_l2,
            len,
        })
    }
}

/// Every size, offset and dispatch grid of one solve.
#[derive(Clone, Debug)]
pub(super) struct Layout {
    pub(super) max_w: u32,
    pub(super) max_h: u32,
    /// Pixels of the padded full-resolution grid.
    pub(super) count: u32,
    pub(super) nchannel: u32,
    /// `f32` elements of each channel state buffer.
    pub(super) channel_len: u32,
    pub(super) aux: AuxLayout,
    /// Per-channel coefficient offset, relative to `aux.a_coefs`.
    coef_offsets: [u32; MAX_CHANNELS],
    pub(super) reduce_groups: u32,
    /// `count * nchannel` invocations.
    pub(super) pixel_grid: Grid,
    /// `count` invocations.
    pub(super) plane_grid: Grid,
    /// The largest component's pixels; channels are the `z` layers.
    pub(super) component_grid: Grid,
    /// One workgroup per block of the largest component; channels are the `z` layers.
    pub(super) block_grid: Grid,
}

impl Layout {
    /// Plan a solve of `shapes` on a `max_w` x `max_h` grid, or explain which
    /// device limit it exceeds.
    pub(super) fn new(
        shapes: &[ComponentShape],
        max_w: u32,
        max_h: u32,
        limits: &DeviceLimits,
    ) -> Result<Self, GpuError> {
        if shapes.is_empty() || shapes.len() > MAX_CHANNELS {
            return Err(invalid("expected 1 to 3 components"));
        }
        let nchannel = u32::try_from(shapes.len()).map_err(|_| invalid("too many components"))?;
        let count = max_w
            .checked_mul(max_h)
            .filter(|&c| c > 0)
            .ok_or_else(|| invalid("image grid is empty or overflows u32"))?;

        let mut coef_offsets = [0; MAX_CHANNELS];
        let mut coefs_len = 0_u32;
        let mut max_blocks = 0;
        let mut max_component_pixels = 0;
        for (shape, offset) in shapes.iter().zip(&mut coef_offsets) {
            let pixels = shape.validate(count)?;
            *offset = coefs_len;
            coefs_len = coefs_len
                .checked_add(shape.coef_len()?)
                .ok_or_else(|| GpuError::Unsupported("coefficients exceed u32 indexing".into()))?;
            max_blocks = max_blocks.max(shape.block_count);
            max_component_pixels = max_component_pixels.max(pixels);
        }

        let channel_len = CHANNEL_FIELDS
            .checked_mul(count)
            .ok_or_else(|| GpuError::Unsupported("channel state exceeds u32 indexing".into()))?;
        check_binding("channel state buffer", channel_len, limits)?;

        let reduce_groups = count
            .div_ceil(PIXEL_WORKGROUP)
            .min(MAX_REDUCE_GROUPS)
            .min(limits.workgroups_per_dimension);
        let aux = AuxLayout::new(count, nchannel, coefs_len, reduce_groups)?;
        check_binding("auxiliary buffer", aux.len, limits)?;

        let pixel_invocations = count
            .checked_mul(nchannel)
            .ok_or_else(|| GpuError::Unsupported("pixel work exceeds u32 indexing".into()))?;
        Ok(Self {
            max_w,
            max_h,
            count,
            nchannel,
            channel_len,
            aux,
            coef_offsets,
            reduce_groups,
            pixel_grid: Grid::for_pixels("per-pixel kernels", pixel_invocations, limits)?,
            plane_grid: Grid::for_pixels("per-plane kernels", count, limits)?,
            component_grid: Grid::for_pixels("resampling kernels", max_component_pixels, limits)?,
            block_grid: Grid::new("block kernels", max_blocks, BLOCK_WORKGROUP, limits)?,
        })
    }

    /// Byte offset of state `field` within a channel buffer. Fields are below
    /// `CHANNEL_FIELDS`, so this stays inside the checked `channel_len`.
    pub(super) fn field_bytes(&self, field: u32) -> u64 {
        f32_bytes(field.saturating_mul(self.count))
    }

    /// Byte offsets in `aux` of channel `c`'s coefficients and quantisation
    /// table; `None` for a channel outside the layout.
    pub(super) fn aux_channel_bytes(&self, c: usize) -> Option<(u64, u64)> {
        if c >= usize::try_from(self.nchannel).ok()? {
            return None;
        }
        let coefs = self.aux.a_coefs.checked_add(*self.coef_offsets.get(c)?)?;
        let quant = u32::try_from(c)
            .ok()?
            .checked_mul(64)?
            .checked_add(self.aux.a_quant)?;
        Some((f32_bytes(coefs), f32_bytes(quant)))
    }

    /// Per-channel metadata in `M_*` order of `shaders/solver.wgsl`.
    pub(super) fn meta(&self, shapes: &[ComponentShape]) -> [[u32; 8]; MAX_CHANNELS] {
        let mut meta = [[0; 8]; MAX_CHANNELS];
        for ((m, s), &offset) in meta.iter_mut().zip(shapes).zip(&self.coef_offsets) {
            *m = [
                s.block_count,
                offset,
                s.rounded_w,
                s.rounded_h,
                s.block_w,
                s.block_h,
                s.h_factor,
                s.v_factor,
            ];
        }
        meta
    }
}

/// `f32` elements as bytes.
pub(super) fn f32_bytes(elements: u32) -> u64 {
    u64::from(elements).saturating_mul(4)
}

fn check_binding(what: &str, elements: u32, limits: &DeviceLimits) -> Result<(), GpuError> {
    let bytes = f32_bytes(elements);
    let limit = limits.buffer_size.min(limits.storage_binding_size);
    if bytes > limit {
        return Err(GpuError::Unsupported(format!(
            "{what} needs {bytes} bytes; the device allows {limit}"
        )));
    }
    Ok(())
}

fn invalid(message: &str) -> GpuError {
    GpuError::Runtime(format!("invalid GPU input: {message}"))
}

#[cfg(test)]
mod tests {
    use super::{ComponentShape, DeviceLimits, Grid, Layout};
    use crate::pipeline::gpu::GpuError;

    /// `wgpu::Limits::default()`: what the solver got before requesting adapter limits.
    const DEFAULT: DeviceLimits = DeviceLimits {
        buffer_size: 256 << 20,
        storage_binding_size: 128 << 20,
        workgroups_per_dimension: 65_535,
    };
    /// A desktop GPU's adapter limits (RADV).
    const DESKTOP: DeviceLimits = DeviceLimits {
        buffer_size: 2_147_483_647,
        storage_binding_size: 2_147_483_644,
        workgroups_per_dimension: 65_535,
    };

    fn shape(w: u32, h: u32, factor: u32) -> ComponentShape {
        let (block_w, block_h) = (w.div_ceil(8), h.div_ceil(8));
        ComponentShape {
            rounded_w: block_w.saturating_mul(8),
            rounded_h: block_h.saturating_mul(8),
            block_w,
            block_h,
            block_count: block_w.saturating_mul(block_h),
            h_factor: factor,
            v_factor: factor,
        }
    }

    /// Luma plus two 2x2-subsampled chroma components.
    fn yuv420(w: u32, h: u32) -> [ComponentShape; 3] {
        let chroma = shape(w.div_ceil(2), h.div_ceil(2), 2);
        [shape(w, h, 1), chroma, chroma]
    }

    fn unsupported(result: Result<Layout, GpuError>) -> String {
        match result {
            Err(GpuError::Unsupported(message)) => message,
            other => panic!("expected an unsupported-image error, got {other:?}"),
        }
    }

    #[test]
    fn issue_55_uhd_exceeds_default_limits_but_fits_desktop_limits() {
        let shapes = yuv420(3840, 2160);
        let message = unsupported(Layout::new(&shapes, 3840, 2160, &DEFAULT));
        assert!(message.contains("298598400"), "{message}");

        let layout = Layout::new(&shapes, 3840, 2160, &DESKTOP).expect("UHD layout");
        // 97,200 pixel workgroups and 129,600 luma blocks both exceed one dimension.
        assert_eq!(layout.pixel_grid, Grid { x: 65_535, y: 2 });
        assert_eq!(layout.block_grid, Grid { x: 65_535, y: 2 });
        assert_eq!(layout.reduce_groups, 1024);
    }

    #[test]
    fn eight_k_fits_desktop_limits() {
        let layout = Layout::new(&yuv420(7680, 4320), 7680, 4320, &DESKTOP).expect("8K layout");
        assert_eq!(layout.block_grid, Grid { x: 65_535, y: 8 });
        assert_eq!(layout.component_grid, Grid { x: 65_535, y: 2 });
    }

    #[test]
    fn buffer_and_binding_limits_are_inclusive() {
        let shapes = [shape(64, 64, 1)];
        let channel_bytes = 9 * 64 * 64 * 4;
        let exact = DeviceLimits {
            storage_binding_size: channel_bytes,
            buffer_size: channel_bytes,
            ..DESKTOP
        };
        Layout::new(&shapes, 64, 64, &exact).expect("exact limits");
        for limits in [
            DeviceLimits {
                storage_binding_size: channel_bytes - 1,
                ..DESKTOP
            },
            DeviceLimits {
                buffer_size: channel_bytes - 1,
                ..DESKTOP
            },
        ] {
            let message = unsupported(Layout::new(&shapes, 64, 64, &limits));
            assert!(message.contains("channel state"), "{message}");
        }
    }

    #[test]
    fn grids_fold_into_rows_until_both_dimensions_are_full() {
        let limits = DeviceLimits {
            workgroups_per_dimension: 4,
            ..DESKTOP
        };
        let grid = |groups| Grid::new("t", groups, 64, &limits);
        assert_eq!(grid(4).ok(), Some(Grid { x: 4, y: 1 }));
        assert_eq!(grid(5).ok(), Some(Grid { x: 4, y: 2 }));
        assert_eq!(grid(16).ok(), Some(Grid { x: 4, y: 4 }));
        assert!(matches!(grid(17), Err(GpuError::Unsupported(_))));
        assert!(matches!(grid(0), Err(GpuError::Runtime(_))));
    }

    #[test]
    fn invocation_ids_must_fit_u32() {
        let limits = DeviceLimits {
            workgroups_per_dimension: u32::MAX,
            ..DESKTOP
        };
        assert!(Grid::new("t", u32::MAX / 256, 256, &limits).is_ok());
        assert!(matches!(
            Grid::new("t", u32::MAX / 256 + 1, 256, &limits),
            Err(GpuError::Unsupported(_))
        ));
    }

    #[test]
    fn rejects_inconsistent_or_oversized_components() {
        let mut bad = shape(16, 16, 1);
        bad.block_count += 1;
        assert!(matches!(
            Layout::new(&[bad], 16, 16, &DESKTOP),
            Err(GpuError::Runtime(_))
        ));
        // A component larger than the image grid would overrun the shared slots.
        assert!(Layout::new(&[shape(24, 16, 1)], 16, 16, &DESKTOP).is_err());
        assert!(Layout::new(&[], 16, 16, &DESKTOP).is_err());
        assert!(Layout::new(&[shape(16, 16, 1)], 0, 16, &DESKTOP).is_err());
        let four = [shape(16, 16, 1); 4];
        assert!(Layout::new(&four, 16, 16, &DESKTOP).is_err());
    }

    #[test]
    fn aux_regions_are_disjoint_and_ordered() {
        let shapes = yuv420(100, 60);
        let layout = Layout::new(&shapes, 104, 64, &DESKTOP).expect("layout");
        let (aux, count) = (layout.aux, 104 * 64);
        let chroma_coefs = 56 * 32;
        assert_eq!(aux.a_mean, count);
        assert_eq!(aux.a_coefs, count * 4);
        assert_eq!(aux.a_quant, aux.a_coefs + count + 2 * chroma_coefs);
        assert_eq!(aux.a_partials, aux.a_quant + 3 * 64);
        assert_eq!(aux.a_l2, aux.a_partials + 3 * layout.reduce_groups);
        assert_eq!(aux.len, aux.a_l2 + 3);

        let bytes = |floats: u32| u64::from(floats) * 4;
        assert_eq!(
            layout.aux_channel_bytes(2),
            Some((
                bytes(aux.a_coefs + count + chroma_coefs),
                bytes(aux.a_quant + 128)
            ))
        );
        assert_eq!(layout.aux_channel_bytes(3), None);
        let meta = layout.meta(&shapes);
        assert_eq!(meta[1], [7 * 4, count, 56, 32, 7, 4, 2, 2]);
        assert_eq!(meta[2][1], count + chroma_coefs);
    }

    /// Sweep dimensions, subsampling ratios and device limits and verify the
    /// produced layout obeys every buffer, binding, grid and `u32` invariant.
    #[test]
    fn layout_invariants_hold_across_geometries_and_limits() {
        let sizes: [(u32, u32); 6] = [
            (8, 8),
            (17, 9),
            (640, 480),
            (1920, 1080),
            (3840, 2160),
            (7680, 4320),
        ];
        let limit_sets = [
            DEFAULT,
            DESKTOP,
            DeviceLimits {
                workgroups_per_dimension: 32,
                ..DESKTOP
            },
        ];
        for limits in limit_sets {
            for &(w, h) in &sizes {
                let w444 = w.div_ceil(8) * 8;
                let h444 = h.div_ceil(8) * 8;
                let w420 = w.div_ceil(16) * 16;
                let h420 = h.div_ceil(16) * 16;
                let cases: Vec<(u32, u32, Vec<ComponentShape>)> = vec![
                    (w444, h444, vec![shape(w, h, 1)]),
                    (w444, h444, vec![shape(w, h, 1); 3]),
                    (w420, h420, yuv420(w, h).to_vec()),
                ];
                for (max_w, max_h, shapes) in cases {
                    let Ok(layout) = Layout::new(&shapes, max_w, max_h, &limits) else {
                        continue;
                    };
                    let max_bytes = limits.buffer_size.min(limits.storage_binding_size);
                    assert!(u64::from(layout.channel_len) * 4 <= max_bytes);
                    assert!(u64::from(layout.aux.len) * 4 <= max_bytes);
                    assert!(layout.aux.a_norm < layout.aux.a_mean);
                    assert!(layout.aux.a_mean < layout.aux.a_coefs);
                    assert!(layout.aux.a_coefs < layout.aux.a_quant);
                    assert!(layout.aux.a_quant < layout.aux.a_partials);
                    assert!(layout.aux.a_partials < layout.aux.a_l2);
                    assert!(layout.aux.a_l2 < layout.aux.len);
                    assert!(layout.reduce_groups >= 1);
                    assert!(layout.reduce_groups <= limits.workgroups_per_dimension);
                    for (grid, needed) in [
                        (
                            layout.pixel_grid,
                            (layout.count * layout.nchannel).div_ceil(256),
                        ),
                        (layout.plane_grid, layout.count.div_ceil(256)),
                        (
                            layout.block_grid,
                            shapes.iter().map(|s| s.block_count).max().unwrap(),
                        ),
                    ] {
                        assert!(grid.x >= 1 && grid.x <= limits.workgroups_per_dimension);
                        assert!(grid.y >= 1 && grid.y <= limits.workgroups_per_dimension);
                        assert!(u64::from(grid.x) * u64::from(grid.y) >= u64::from(needed));
                    }
                }
            }
        }
    }
}
