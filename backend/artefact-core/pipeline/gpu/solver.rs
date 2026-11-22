//! GPU solver orchestration: resource upload, pipeline creation, iteration loop.
//!
//! Sizes, offsets and dispatch grids come from [`Layout`], which is checked
//! against the device's limits before anything large is allocated.

use wgpu::{
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, ShaderStages,
    util::DeviceExt,
};

use crate::{
    jpeg::Coefficient,
    utils::{aligned::AlignedF32, auxiliary::upsample_pixels, dct::idct8x8},
};

use super::{
    GpuContext, GpuError,
    layout::{
        ComponentShape, DeviceLimits, F_COS, F_FDATA, F_FISTA, Grid, Layout, MAX_CHANNELS,
        f32_bytes,
    },
};

/// Size of the placeholder bound to channel slots a solve does not use.
const PLACEHOLDER_BYTES: u64 = 4;

/// Uniform scalars; field order must match `Params` in `shaders/solver.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    count: u32,
    max_w: u32,
    max_h: u32,
    nchannel: u32,
    fdata_field: u32,
    fista_field: u32,
    a_norm: u32,
    a_mean: u32,
    a_coefs: u32,
    a_quant: u32,
    a_partials: u32,
    a_l2: u32,
    reduce_groups: u32,
    tv_alpha: f32,
    tgv_alpha: f32,
    step_size: f32,
    factor: f32,
    dct_alpha0: f32,
    dct_alpha1: f32,
    dct_alpha2: f32,
}

impl Params {
    /// The weights mirror the CPU `solver_step`.
    fn new(layout: &Layout, weight: f32, pweight: &[f32; 3], iterations: usize) -> Self {
        let channels = u32_to_f32(layout.nchannel).sqrt();
        let iterations = u32_to_f32(u32::try_from(iterations).unwrap_or(u32::MAX));
        let dct_alpha = |pw: f32| pw * 2.0 * 255.0 * 2.0_f32.sqrt();
        let [pw0, pw1, pw2] = *pweight;
        let aux = &layout.aux;
        Self {
            count: layout.count,
            max_w: layout.max_w,
            max_h: layout.max_h,
            nchannel: layout.nchannel,
            fdata_field: F_FDATA,
            fista_field: F_FISTA,
            a_norm: aux.a_norm,
            a_mean: aux.a_mean,
            a_coefs: aux.a_coefs,
            a_quant: aux.a_quant,
            a_partials: aux.a_partials,
            a_l2: aux.a_l2,
            reduce_groups: layout.reduce_groups,
            tv_alpha: 1.0 / channels,
            tgv_alpha: weight / 2.0_f32.sqrt() / channels,
            step_size: u32_to_f32(layout.count).sqrt() / 2.0 / (1.0 + iterations).sqrt(),
            factor: 0.0,
            dct_alpha0: dct_alpha(pw0),
            dct_alpha1: dct_alpha(pw1),
            dct_alpha2: dct_alpha(pw2),
        }
    }
}

struct Pipelines {
    fista: wgpu::ComputePipeline,
    tv_prepare: wgpu::ComputePipeline,
    tv_apply: wgpu::ComputePipeline,
    dct_gradient: wgpu::ComputePipeline,
    tgv_prepare: wgpu::ComputePipeline,
    tgv_apply: wgpu::ComputePipeline,
    reduce1: wgpu::ComputePipeline,
    reduce2: wgpu::ComputePipeline,
    descent: wgpu::ComputePipeline,
    resample_down: wgpu::ComputePipeline,
    block_transform: wgpu::ComputePipeline,
    resample_up: wgpu::ComputePipeline,
}

fn u32_to_f32(n: u32) -> f32 {
    let hi = f32::from(u16::try_from(n >> 16).unwrap_or(u16::MAX));
    let lo = f32::from(u16::try_from(n & 0xFFFF).unwrap_or(0));
    hi.mul_add(65536.0, lo)
}

/// Solve on the GPU, returning one `AlignedF32` (pixel data) per component.
///
/// # Errors
/// [`GpuError::Unsupported`] when the image needs more buffer space, dispatch
/// work or memory than the device provides; [`GpuError::Runtime`] for invalid
/// coefficients or a failed GPU operation.
///
/// Returned as a boxed `!Send` future because `wgpu`'s buffer types trigger
/// `Send` auto-trait recursion overflow across `.await` points.
pub fn solve<'a>(
    ctx: &'a GpuContext,
    coefs: &'a [Coefficient],
    weight: f32,
    pweight: &'a [f32; 3],
    iterations: usize,
    max_w: u32,
    max_h: u32,
) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<AlignedF32>, GpuError>> + 'a>> {
    Box::pin(async move {
        let device = ctx.device();
        let shapes = coefs
            .iter()
            .map(ComponentShape::of)
            .collect::<Result<Vec<_>, _>>()?;
        let layout = Layout::new(&shapes, max_w, max_h, &DeviceLimits::from(&device.limits()))?;
        with_captured_errors(device, async {
            let (resources, field) =
                run(ctx, coefs, &shapes, &layout, weight, pweight, iterations)?;
            read_back(ctx, &resources.channels, &layout, field).await
        })
        .await
    })
}

/// Run `work` with every `wgpu` error captured and returned. Uncaptured errors
/// panic natively, so this is what turns an unforeseen failure (notably running
/// out of GPU memory) into an error that lets `process_auto` fall back to the CPU.
pub(super) fn with_captured_errors<'a, T: 'a>(
    device: &'a wgpu::Device,
    work: impl Future<Output = Result<T, GpuError>> + 'a,
) -> std::pin::Pin<Box<dyn Future<Output = Result<T, GpuError>> + 'a>> {
    Box::pin(async move {
        let [out_of_memory, validation, internal] = [
            wgpu::ErrorFilter::OutOfMemory,
            wgpu::ErrorFilter::Validation,
            wgpu::ErrorFilter::Internal,
        ]
        .map(|filter| device.push_error_scope(filter));
        let result = work.await;
        // Scopes pop in reverse push order.
        let internal = internal.pop().await;
        let validation = validation.pop().await;
        let out_of_memory = out_of_memory.pop().await;
        captured_error(out_of_memory, validation, internal).map_or(result, Err)
    })
}

/// The one error to report for what the three scopes captured, if anything.
/// Out-of-memory is an image that does not fit, not a malfunction.
pub(super) fn captured_error(
    out_of_memory: Option<wgpu::Error>,
    validation: Option<wgpu::Error>,
    internal: Option<wgpu::Error>,
) -> Option<GpuError> {
    if let Some(e) = out_of_memory {
        return Some(GpuError::Unsupported(format!("out of GPU memory: {e}")));
    }
    validation
        .or(internal)
        .map(|e| GpuError::Runtime(e.to_string()))
}

/// Upload the state and record every iteration. Returns the resources and the
/// state field holding the final iterate.
fn run(
    ctx: &GpuContext,
    coefs: &[Coefficient],
    shapes: &[ComponentShape],
    layout: &Layout,
    weight: f32,
    pweight: &[f32; 3],
    iterations: usize,
) -> Result<(Resources, u32), GpuError> {
    let (device, queue) = (ctx.device(), ctx.queue());
    let mut params = Params::new(layout, weight, pweight, iterations);
    let resources = Resources::new(device, queue, coefs, shapes, layout, &params)?;

    // The iterate ping-pongs between the two state fields: FISTA reads `cur`
    // and writes `scratch`, then the remaining passes refine `scratch`.
    let (mut cur, mut scratch) = (F_FDATA, F_FISTA);
    let mut term = 1.0_f32;
    for _ in 0..iterations {
        let next = f32::midpoint(1.0, 4.0_f32.mul_add(term * term, 1.0).sqrt());
        params.factor = (term - 1.0) / next;
        term = next;
        resources.iterate(device, queue, layout, &mut params, cur, scratch);
        std::mem::swap(&mut cur, &mut scratch);
    }
    Ok((resources, cur))
}

struct Resources {
    channels: [wgpu::Buffer; MAX_CHANNELS],
    params: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipelines: Pipelines,
}

impl Resources {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        coefs: &[Coefficient],
        shapes: &[ComponentShape],
        layout: &Layout,
        params: &Params,
    ) -> Result<Self, GpuError> {
        // `wgpu` zero-initialises buffers, so only non-zero regions are uploaded.
        let aux = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("artefact-aux"),
            size: f32_bytes(layout.aux.len),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let channels = upload_channels(device, queue, &aux, coefs, shapes, layout)?;
        let meta = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("artefact-meta"),
            contents: bytemuck::cast_slice(&layout.meta(shapes)),
            usage: BufferUsages::STORAGE,
        });
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("artefact-params"),
            contents: bytemuck::bytes_of(params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = build_bind_group_layout(device);
        let [c0, c1, c2] = &channels;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("artefact-solver-bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: c0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: c1.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: c2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: aux.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: meta.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("artefact-solver-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        Ok(Self {
            channels,
            params: params_buf,
            bind_group,
            pipelines: build_pipelines(device, &pipeline_layout),
        })
    }

    /// One FISTA iteration. Two submissions, because the state fields swap
    /// roles between the momentum pass and the rest (queue writes land between
    /// submissions).
    fn iterate(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &Layout,
        params: &mut Params,
        cur: u32,
        scratch: u32,
    ) {
        let p = &self.pipelines;
        params.fdata_field = cur;
        params.fista_field = scratch;
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(params));
        self.submit(device, queue, "artefact-fista", |pass| {
            dispatch(pass, &p.fista, layout.pixel_grid, 1);
        });

        // Promote the FISTA result to the current iterate.
        params.fdata_field = scratch;
        params.fista_field = cur;
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(params));
        self.submit(device, queue, "artefact-iteration", |pass| {
            dispatch(pass, &p.tv_prepare, layout.plane_grid, 1);
            dispatch(pass, &p.tv_apply, layout.pixel_grid, 1);
            dispatch(pass, &p.dct_gradient, layout.block_grid, layout.nchannel);
            dispatch(pass, &p.tgv_prepare, layout.plane_grid, 1);
            dispatch(pass, &p.tgv_apply, layout.pixel_grid, 1);
            pass.set_pipeline(&p.reduce1);
            pass.dispatch_workgroups(layout.reduce_groups, layout.nchannel, 1);
            pass.set_pipeline(&p.reduce2);
            pass.dispatch_workgroups(layout.nchannel, 1, 1);
            dispatch(pass, &p.descent, layout.pixel_grid, 1);
            dispatch(
                pass,
                &p.resample_down,
                layout.component_grid,
                layout.nchannel,
            );
            dispatch(pass, &p.block_transform, layout.block_grid, layout.nchannel);
            dispatch(pass, &p.resample_up, layout.component_grid, layout.nchannel);
        });
    }

    fn submit(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        record: impl FnOnce(&mut wgpu::ComputePass<'_>),
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            record(&mut pass);
        }
        queue.submit(Some(encoder.finish()));
    }
}

/// Create the channel state buffers and upload each component's initial
/// state, plus its coefficients and quantisation table into `aux`.
fn upload_channels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    aux: &wgpu::Buffer,
    coefs: &[Coefficient],
    shapes: &[ComponentShape],
    layout: &Layout,
) -> Result<[wgpu::Buffer; MAX_CHANNELS], GpuError> {
    let mut channels = Vec::with_capacity(MAX_CHANNELS);
    for (c, (coef, shape)) in coefs.iter().zip(shapes).enumerate() {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("artefact-channel"),
            size: f32_bytes(layout.channel_len),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Built and uploaded one channel at a time to bound host memory.
        let (cos, pixels) = channel_init(coef, shape, layout)?;
        let pixels = bytemuck::cast_slice(&pixels);
        queue.write_buffer(&buffer, layout.field_bytes(F_FDATA), pixels);
        queue.write_buffer(&buffer, layout.field_bytes(F_FISTA), pixels);
        queue.write_buffer(
            &buffer,
            layout.field_bytes(F_COS),
            bytemuck::cast_slice(&cos),
        );

        let (coefs_at, quant_at) = layout
            .aux_channel_bytes(c)
            .ok_or_else(|| GpuError::Runtime("component outside the solver layout".into()))?;
        queue.write_buffer(aux, coefs_at, bytemuck::cast_slice(&coef.dct_coefs));
        queue.write_buffer(aux, quant_at, bytemuck::cast_slice(&coef.quant_table));
        channels.push(buffer);
    }
    // The bind group always has three channel slots; kernels never touch
    // slots at or beyond `nchannel`.
    while channels.len() < MAX_CHANNELS {
        channels.push(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("artefact-channel-unused"),
            size: PLACEHOLDER_BYTES,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        }));
    }
    channels
        .try_into()
        .map_err(|_| GpuError::Runtime("too many channel buffers".into()))
}

/// A component's dequantised coefficients (the projection target, block-major)
/// and its decoded raster upsampled to the full image grid.
fn channel_init(
    coef: &Coefficient,
    shape: &ComponentShape,
    layout: &Layout,
) -> Result<(Vec<f32>, Vec<f32>), GpuError> {
    let overflow = || GpuError::Runtime("component raster index overflow".into());
    let to_usize = |v: u32| usize::try_from(v).map_err(|_| overflow());
    let (rw, block_w) = (to_usize(shape.rounded_w)?, to_usize(shape.block_w)?);
    let mut cos = Vec::with_capacity(coef.dct_coefs.len());
    let mut raster = vec![0.0_f32; to_usize(shape.pixels()?)?];
    for (i, dct) in coef.dct_coefs.as_chunks::<64>().0.iter().enumerate() {
        let mut block = [0.0_f32; 64];
        for ((b, &d), &q) in block.iter_mut().zip(dct).zip(&coef.quant_table) {
            *b = d * q;
        }
        cos.extend_from_slice(&block);
        idct8x8(&mut block);
        let (Some(by), Some(bx)) = (i.checked_div(block_w), i.checked_rem(block_w)) else {
            return Err(overflow());
        };
        let x = bx.checked_mul(8).ok_or_else(overflow)?;
        for (row, src) in block.as_chunks::<8>().0.iter().enumerate() {
            let start = by
                .checked_mul(8)
                .and_then(|y| y.checked_add(row))
                .and_then(|y| y.checked_mul(rw))
                .and_then(|r| r.checked_add(x))
                .ok_or_else(overflow)?;
            raster
                .get_mut(start..start.saturating_add(8))
                .ok_or_else(overflow)?
                .copy_from_slice(src);
        }
    }
    let mut pixels = vec![0.0_f32; to_usize(layout.count)?];
    upsample_pixels(
        &raster,
        shape.rounded_w,
        shape.rounded_h,
        to_usize(shape.h_factor)?,
        to_usize(shape.v_factor)?,
        layout.max_w,
        layout.max_h,
        &mut pixels,
    );
    Ok((cos, pixels))
}

fn build_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let buffer = |binding, ty| BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let storage = BufferBindingType::Storage { read_only: false };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("artefact-solver-bgl"),
        entries: &[
            buffer(0, storage),
            buffer(1, storage),
            buffer(2, storage),
            buffer(3, storage),
            buffer(4, BufferBindingType::Storage { read_only: true }),
            buffer(5, BufferBindingType::Uniform),
        ],
    })
}

fn build_pipelines(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout) -> Pipelines {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("artefact-solver"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/solver.wgsl").into()),
    });
    let pipeline = |entry: &str| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry),
            layout: Some(pipeline_layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };
    Pipelines {
        fista: pipeline("fista"),
        tv_prepare: pipeline("tv_prepare"),
        tv_apply: pipeline("tv_apply"),
        dct_gradient: pipeline("dct_gradient"),
        tgv_prepare: pipeline("tgv_prepare"),
        tgv_apply: pipeline("tgv_apply"),
        reduce1: pipeline("reduce1"),
        reduce2: pipeline("reduce2"),
        descent: pipeline("descent"),
        resample_down: pipeline("resample_down"),
        block_transform: pipeline("block_transform"),
        resample_up: pipeline("resample_up"),
    }
}

/// Dispatch a folded `grid` with `layers` channel layers in `z`.
fn dispatch(
    pass: &mut wgpu::ComputePass<'_>,
    pipeline: &wgpu::ComputePipeline,
    grid: Grid,
    layers: u32,
) {
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(grid.x, grid.y, layers);
}

/// Copy `field` of each solved channel back to the host through one reused
/// staging buffer. Boxed and `!Send` for the same reason as [`solve`].
fn read_back<'a>(
    ctx: &'a GpuContext,
    channels: &'a [wgpu::Buffer; MAX_CHANNELS],
    layout: &'a Layout,
    field: u32,
) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<AlignedF32>, GpuError>> + 'a>> {
    Box::pin(async move {
        let (device, queue) = (ctx.device(), ctx.queue());
        let bytes = f32_bytes(layout.count);
        let to_usize =
            |v: u32| usize::try_from(v).map_err(|_| GpuError::Runtime("size exceeds usize".into()));
        let (count, nchannel) = (to_usize(layout.count)?, to_usize(layout.nchannel)?);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("artefact-staging"),
            size: bytes,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut out = Vec::with_capacity(nchannel);
        for channel in channels.iter().take(nchannel) {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("artefact-readback"),
            });
            encoder.copy_buffer_to_buffer(channel, layout.field_bytes(field), &staging, 0, bytes);
            queue.submit(Some(encoder.finish()));

            let slice = staging.slice(..);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            await_map(device, rx).await?;
            let mut data = AlignedF32::zeros(count);
            {
                let mapped = slice
                    .get_mapped_range()
                    .map_err(|e| GpuError::Runtime(e.to_string()))?;
                data.copy_from_slice(bytemuck::cast_slice(&mapped));
            }
            staging.unmap();
            out.push(data);
        }
        Ok(out)
    })
}

/// Wait for a `map_async` callback.
///
/// On native the device must be polled to drive the callback, so we block on
/// `poll(Wait)`. On wasm `poll` is a no-op and the callback runs as a task, so
/// blocking would deadlock the single thread; there we await the channel.
#[cfg(not(target_arch = "wasm32"))]
fn await_map(
    device: &wgpu::Device,
    mut rx: futures_channel::oneshot::Receiver<Result<(), wgpu::BufferAsyncError>>,
) -> impl Future<Output = Result<(), GpuError>> + '_ {
    std::future::ready(loop {
        if let Err(e) = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        }) {
            break Err(GpuError::Runtime(e.to_string()));
        }
        match rx.try_recv() {
            Ok(Some(result)) => break result.map_err(|e| GpuError::Runtime(e.to_string())),
            Ok(None) => {}
            Err(e) => break Err(GpuError::Runtime(e.to_string())),
        }
    })
}

#[cfg(target_arch = "wasm32")]
async fn await_map(
    _device: &wgpu::Device,
    rx: futures_channel::oneshot::Receiver<Result<(), wgpu::BufferAsyncError>>,
) -> Result<(), GpuError> {
    rx.await
        .map_err(|e| GpuError::Runtime(e.to_string()))?
        .map_err(|e| GpuError::Runtime(e.to_string()))
}
