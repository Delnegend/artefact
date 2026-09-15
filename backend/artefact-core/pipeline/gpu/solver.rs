//! GPU solver orchestration: buffer layout, pipeline creation, iteration loop.

use wgpu::{
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, ShaderStages,
    util::DeviceExt,
};

use crate::{
    jpeg::Coefficient,
    utils::{aligned::AlignedF32, auxiliary::upsample_pixels, dct::idct8x8},
};

use super::{GpuContext, GpuError};

const NCHANNELS: usize = 3;
const CH_FIELDS: u32 = 9; // fields per channel (in units of `count`)
const F_FDATA: u32 = 0;
const F_FISTA: u32 = 1;
const F_COS: u32 = 8;
const WORKGROUP: u32 = 256;

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
    num_wg: u32,
    tv_alpha: f32,
    tgv_alpha: f32,
    step_size: f32,
    factor: f32,
    dct_alpha0: f32,
    dct_alpha1: f32,
    dct_alpha2: f32,
}

impl Params {
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
}

/// Write one word of per-channel metadata (`meta[c * 8 + k]`).
const fn meta_word(c: usize, k: usize, value: u32, out: &mut [u32]) {
    out[c * 8 + k] = value;
}

/// Solve on the GPU, returning one `AlignedF32` (pixel data) per component.
///
/// # Errors
/// Propagates any GPU error while building buffers/pipelines or running.
#[allow(clippy::too_many_arguments)]
pub async fn solve(
    ctx: &GpuContext,
    coefs: &[Coefficient],
    weight: f32,
    pweight: &[f32; 3],
    iterations: usize,
    max_w: u32,
    max_h: u32,
    count: usize,
) -> Result<Vec<AlignedF32>, GpuError> {
    let device = ctx.device();
    let queue = ctx.queue();
    let nchannel = coefs.len().min(NCHANNELS);

    // --- CPU-side per-channel prep + layout ---------------------------------
    let mut coefs_len = 0usize;
    for coef in coefs {
        coefs_len += coef.block_count as usize * 64;
    }
    let num_wg = count.div_ceil(WORKGROUP as usize) as u32;
    let a_norm = 0u32;
    let a_mean = count as u32;
    let a_coefs = a_mean + nchannel as u32 * count as u32;
    let a_quant = a_coefs + coefs_len as u32;
    let a_partials = a_quant + nchannel as u32 * 64;
    let a_l2 = a_partials + num_wg * nchannel as u32;
    let aux_len = (a_l2 + nchannel as u32) as usize;

    let chan_len = CH_FIELDS as usize * count;
    let mut chan_data: Vec<Vec<f32>> = Vec::with_capacity(nchannel);
    let mut aux_data = vec![0.0f32; aux_len];
    let mut meta = vec![0u32; NCHANNELS * 8];
    let mut coef_off = 0usize;
    let mut max_comp_px = 0u32;

    for (c, coef) in coefs.iter().enumerate() {
        let rw = coef.rounded_px_w as usize;
        let blocks = coef.block_count as usize;

        // Dequantized DCT target (block order) + spatial image data.
        let mut cos = vec![0.0f32; rw * coef.rounded_px_h as usize];
        let mut image_data = vec![0.0f32; rw * coef.rounded_px_h as usize];
        for i in 0..blocks {
            let mut block = [0.0f32; 64];
            for j in 0..64 {
                let v = coef.dct_coefs[i * 64 + j] * coef.quant_table[j];
                cos[i * 64 + j] = v;
                block[j] = v;
            }
            idct8x8(&mut block);
            let by = i / coef.block_w as usize;
            let bx = i % coef.block_w as usize;
            for in_y in 0..8 {
                let row = (by * 8 + in_y) * rw + bx * 8;
                for in_x in 0..8 {
                    image_data[row + in_x] = block[in_y * 8 + in_x];
                }
            }
        }

        let mut chan = vec![0.0f32; chan_len];
        upsample_pixels(
            &image_data,
            coef.rounded_px_w,
            coef.rounded_px_h,
            coef.horizontal_samp_factor.usize(),
            coef.vertical_samp_factor.usize(),
            max_w,
            max_h,
            &mut chan[F_FDATA as usize * count..(F_FDATA as usize + 1) * count],
        );
        chan[F_COS as usize * count..F_COS as usize * count + cos.len()].copy_from_slice(&cos);
        // The CPU seeds the FISTA momentum buffer with the initial image, so the
        // first iteration's momentum term is a no-op.
        chan.copy_within(
            F_FDATA as usize * count..(F_FDATA as usize + 1) * count,
            F_FISTA as usize * count,
        );

        meta_word(c, 0, coef.block_count, &mut meta);
        meta_word(c, 1, coef_off as u32, &mut meta);
        meta_word(c, 2, coef.rounded_px_w, &mut meta);
        meta_word(c, 3, coef.rounded_px_h, &mut meta);
        meta_word(c, 4, coef.block_w, &mut meta);
        meta_word(c, 5, coef.block_h, &mut meta);
        meta_word(c, 6, coef.horizontal_samp_factor.u32(), &mut meta);
        meta_word(c, 7, coef.vertical_samp_factor.u32(), &mut meta);

        aux_data[a_coefs as usize + coef_off..a_coefs as usize + coef_off + blocks * 64]
            .copy_from_slice(&coef.dct_coefs);
        aux_data[a_quant as usize + c * 64..a_quant as usize + c * 64 + 64]
            .copy_from_slice(&coef.quant_table);

        coef_off += blocks * 64;
        max_comp_px = max_comp_px.max(coef.rounded_px_w * coef.rounded_px_h);
        chan_data.push(chan);
    }

    // --- GPU resources ------------------------------------------------------
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("artefact-solver"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/solver.wgsl").into()),
    });

    let storage = |binding| BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let layout_entries = [
        storage(0),
        storage(1),
        storage(2),
        storage(3),
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ];
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("artefact-solver-bgl"),
        entries: &layout_entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("artefact-solver-pl"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let chan_bufs: Vec<wgpu::Buffer> = (0..NCHANNELS)
        .map(|i| {
            chan_data.get(i).map_or_else(
                || {
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("artefact-chan-unused"),
                        size: (chan_len * 4) as u64,
                        usage: BufferUsages::STORAGE
                            | BufferUsages::COPY_SRC
                            | BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    })
                },
                |d| {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("artefact-chan"),
                        contents: bytemuck::cast_slice(d),
                        usage: BufferUsages::STORAGE
                            | BufferUsages::COPY_SRC
                            | BufferUsages::COPY_DST,
                    })
                },
            )
        })
        .collect();

    let aux_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("artefact-aux"),
        contents: bytemuck::cast_slice(&aux_data),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
    });
    let meta_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("artefact-meta"),
        contents: bytemuck::cast_slice(&meta),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    let mut params = Params {
        count: count as u32,
        max_w,
        max_h,
        nchannel: nchannel as u32,
        fdata_field: F_FDATA,
        fista_field: F_FISTA,
        a_norm,
        a_mean,
        a_coefs,
        a_quant,
        a_partials,
        a_l2,
        num_wg,
        tv_alpha: 1.0 / (nchannel as f32).sqrt(),
        tgv_alpha: (weight / 2.0_f32.sqrt()) / (nchannel as f32).sqrt(),
        step_size: (count as f32).sqrt() / 2.0 / (1.0 + iterations as f32).sqrt(),
        factor: 0.0,
        dct_alpha0: pweight[0] * 2.0 * 255.0 * 2.0_f32.sqrt(),
        dct_alpha1: pweight[1] * 2.0 * 255.0 * 2.0_f32.sqrt(),
        dct_alpha2: pweight[2] * 2.0 * 255.0 * 2.0_f32.sqrt(),
    };
    let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("artefact-params"),
        size: Params::SIZE,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("artefact-solver-bg"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: chan_bufs[0].as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: chan_bufs[1].as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: chan_bufs[2].as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: aux_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: meta_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: params_buf.as_entire_binding(),
            },
        ],
    });

    let pipeline = |entry: &str| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };
    let p_fista = pipeline("fista");
    let p_tv_prepare = pipeline("tv_prepare");
    let p_tv_apply = pipeline("tv_apply");
    let p_dct_gradient = pipeline("dct_gradient");
    let p_tgv_prepare = pipeline("tgv_prepare");
    let p_tgv_apply = pipeline("tgv_apply");
    let p_reduce1 = pipeline("reduce1");
    let p_reduce2 = pipeline("reduce2");
    let p_descent = pipeline("descent");
    let p_resample_down = pipeline("resample_down");
    let p_block_transform = pipeline("block_transform");
    let p_resample_up = pipeline("resample_up");

    let max_block_count: u32 = coefs.iter().map(|c| c.block_count).max().unwrap_or(0);
    let total_px = (count * nchannel) as u32;
    let wg_px = total_px.div_ceil(WORKGROUP);
    let wg_count = (count as u32).div_ceil(WORKGROUP);
    let comp_wg = max_comp_px.div_ceil(WORKGROUP);

    // The current iterate ping-pongs between the two state fields: FISTA reads
    // `cur` and writes `scratch`, then the remaining passes operate on the
    // freshly written `scratch`.
    let mut cur = F_FDATA;
    let mut scratch = F_FISTA;

    for it in 0..iterations {
        let term = fista_term(it);
        let next = f32::midpoint(1.0, 4.0_f32.mul_add(term * term, 1.0).sqrt());
        params.factor = (term - 1.0) / next;
        params.fdata_field = cur;
        params.fista_field = scratch;
        queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("artefact-fista"),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_pipeline(&p_fista);
            pass.dispatch_workgroups(wg_px, 1, 1);
            drop(pass);
            queue.submit(Some(encoder.finish()));
        }

        // Promote the FISTA result to the current iterate.
        params.fdata_field = scratch;
        params.fista_field = cur;
        queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("artefact-iteration"),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &bind_group, &[]);

            dispatch(&mut pass, &p_tv_prepare, wg_count);
            dispatch(&mut pass, &p_tv_apply, wg_px);
            dispatch_blocks(&mut pass, &p_dct_gradient, max_block_count, nchannel);
            dispatch(&mut pass, &p_tgv_prepare, wg_count);
            dispatch(&mut pass, &p_tgv_apply, wg_px);

            pass.set_pipeline(&p_reduce1);
            pass.dispatch_workgroups(num_wg, nchannel as u32, 1);
            pass.set_pipeline(&p_reduce2);
            pass.dispatch_workgroups(nchannel as u32, 1, 1);

            dispatch(&mut pass, &p_descent, wg_px);

            pass.set_pipeline(&p_resample_down);
            pass.dispatch_workgroups(comp_wg, nchannel as u32, 1);
            dispatch_blocks(&mut pass, &p_block_transform, max_block_count, nchannel);
            pass.set_pipeline(&p_resample_up);
            pass.dispatch_workgroups(comp_wg, nchannel as u32, 1);
            drop(pass);
            queue.submit(Some(encoder.finish()));
        }

        std::mem::swap(&mut cur, &mut scratch);
    }

    // --- readback -----------------------------------------------------------
    let read_field = cur;
    let mut out = Vec::with_capacity(nchannel);
    for buf in chan_bufs.iter().take(nchannel) {
        let offset = (read_field as usize * count * 4) as u64;
        let size = (count * 4) as u64;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("artefact-readback"),
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("artefact-staging"),
            size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(buf, offset, &staging, 0, size);
        queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        await_map(device, rx).await?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|e| GpuError::Runtime(e.to_string()))?;
        let mut data = AlignedF32::zeros(count);
        data.copy_from_slice(bytemuck::cast_slice(&mapped[..]));
        drop(mapped);
        staging.unmap();
        out.push(data);
    }

    Ok(out)
}

/// Wait for a `map_async` callback.
///
/// On native the device must be polled to drive the callback, so we block on
/// `poll(Wait)`. On wasm `poll` is a no-op and the callback runs as a task, so
/// blocking would deadlock the single thread; there we await the channel.
#[cfg(not(target_arch = "wasm32"))]
async fn await_map(
    device: &wgpu::Device,
    mut rx: futures_channel::oneshot::Receiver<Result<(), wgpu::BufferAsyncError>>,
) -> Result<(), GpuError> {
    loop {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|e| GpuError::Runtime(e.to_string()))?;
        match rx.try_recv() {
            Ok(Some(result)) => return result.map_err(|e| GpuError::Runtime(e.to_string())),
            Ok(None) => continue,
            Err(e) => return Err(GpuError::Runtime(e.to_string())),
        }
    }
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

fn dispatch(pass: &mut wgpu::ComputePass<'_>, pipeline: &wgpu::ComputePipeline, workgroups: u32) {
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(workgroups, 1, 1);
}

/// One 64-lane workgroup per block, laid out as `(max_blocks, nchannel)`.
fn dispatch_blocks(
    pass: &mut wgpu::ComputePass<'_>,
    pipeline: &wgpu::ComputePipeline,
    max_blocks: u32,
    nchannel: usize,
) {
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(max_blocks, nchannel as u32, 1);
}

/// The FISTA `term` before iteration `it` (CPU `run_fista` recurrence).
fn fista_term(iterations: usize) -> f32 {
    let mut term = 1.0_f32;
    for _ in 0..iterations {
        term = f32::midpoint(1.0, 4.0_f32.mul_add(term * term, 1.0).sqrt());
    }
    term
}
