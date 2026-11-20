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

struct AuxLayout {
    a_norm: u32,
    a_mean: u32,
    a_coefs: u32,
    a_quant: u32,
    a_partials: u32,
    a_l2: u32,
    num_wg: u32,
    aux_len: usize,
}

struct CpuPrepared {
    layout: AuxLayout,
    chan_len: usize,
    chan_data: Vec<Vec<f32>>,
    aux_data: Vec<f32>,
    meta: [[u32; 8]; NCHANNELS],
    max_comp_px: u32,
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

fn to_u32(v: usize) -> Result<u32, GpuError> {
    u32::try_from(v).map_err(|_| GpuError::Runtime("size exceeds u32".into()))
}

fn to_usize(v: u32) -> Result<usize, GpuError> {
    usize::try_from(v).map_err(|_| GpuError::Runtime("size exceeds usize".into()))
}

fn to_u64(v: usize) -> Result<u64, GpuError> {
    u64::try_from(v).map_err(|_| GpuError::Runtime("size exceeds u64".into()))
}

fn u32_to_f32(n: u32) -> f32 {
    let hi = f32::from(u16::try_from(n >> 16).unwrap_or(u16::MAX));
    let lo = f32::from(u16::try_from(n & 0xFFFF).unwrap_or(0));
    hi.mul_add(65536.0, lo)
}

fn compute_aux_layout(
    coefs: &[Coefficient],
    nchannel_u32: u32,
    count_u32: u32,
) -> Result<AuxLayout, GpuError> {
    let mut coefs_len = 0u32;
    for coef in coefs {
        let block_elems = coef
            .block_count
            .checked_mul(64)
            .ok_or_else(|| GpuError::Runtime("coefficient count overflow".into()))?;
        coefs_len = coefs_len
            .checked_add(block_elems)
            .ok_or_else(|| GpuError::Runtime("total coefficient count overflow".into()))?;
    }
    let num_wg = count_u32.div_ceil(WORKGROUP);
    let a_norm = 0u32;
    let a_mean = count_u32;
    let a_coefs = nchannel_u32
        .checked_mul(count_u32)
        .and_then(|v| a_mean.checked_add(v))
        .ok_or_else(|| GpuError::Runtime("aux buffer offset overflow".into()))?;
    let a_quant = a_coefs
        .checked_add(coefs_len)
        .ok_or_else(|| GpuError::Runtime("aux buffer offset overflow".into()))?;
    let a_partials = nchannel_u32
        .checked_mul(64)
        .and_then(|v| a_quant.checked_add(v))
        .ok_or_else(|| GpuError::Runtime("aux buffer offset overflow".into()))?;
    let a_l2 = num_wg
        .checked_mul(nchannel_u32)
        .and_then(|v| a_partials.checked_add(v))
        .ok_or_else(|| GpuError::Runtime("aux buffer offset overflow".into()))?;
    let aux_len = a_l2
        .checked_add(nchannel_u32)
        .ok_or_else(|| GpuError::Runtime("aux buffer length overflow".into()))
        .and_then(to_usize)?;
    Ok(AuxLayout {
        a_norm,
        a_mean,
        a_coefs,
        a_quant,
        a_partials,
        a_l2,
        num_wg,
        aux_len,
    })
}

fn prepare_channel_spatial(
    coef: &Coefficient,
    rw: usize,
    px_len: usize,
    blocks: usize,
    block_w: usize,
) -> Result<(Vec<f32>, Vec<f32>), GpuError> {
    let mut cos = vec![0.0f32; px_len];
    let mut image_data = vec![0.0f32; px_len];
    let (dct_chunks, _) = coef.dct_coefs.as_chunks::<64>();
    let (cos_chunks, _) = cos.as_chunks_mut::<64>();
    for (i, (dct_chunk, cos_chunk)) in dct_chunks.iter().zip(cos_chunks).take(blocks).enumerate() {
        let mut block = [0.0f32; 64];
        for ((b, c_out), (&d, &q)) in block
            .iter_mut()
            .zip(cos_chunk.iter_mut())
            .zip(dct_chunk.iter().zip(coef.quant_table.iter()))
        {
            let v = d * q;
            *c_out = v;
            *b = v;
        }
        idct8x8(&mut block);
        let (Some(by), Some(bx)) = (i.checked_div(block_w), i.checked_rem(block_w)) else {
            return Err(GpuError::Runtime("zero block width".into()));
        };
        let y_base = by
            .checked_mul(8)
            .ok_or_else(|| GpuError::Runtime("block y overflow".into()))?;
        let x_base = bx
            .checked_mul(8)
            .ok_or_else(|| GpuError::Runtime("block x overflow".into()))?;
        let (block_rows, _) = block.as_chunks::<8>();
        for (in_y, block_row) in block_rows.iter().enumerate() {
            let row_start = y_base
                .checked_add(in_y)
                .and_then(|y| y.checked_mul(rw))
                .and_then(|r| r.checked_add(x_base))
                .ok_or_else(|| GpuError::Runtime("pixel row offset overflow".into()))?;
            let row_end = row_start
                .checked_add(8)
                .ok_or_else(|| GpuError::Runtime("pixel row end overflow".into()))?;
            image_data
                .get_mut(row_start..row_end)
                .ok_or_else(|| GpuError::Runtime("image_data out of bounds".into()))?
                .copy_from_slice(block_row);
        }
    }
    Ok((cos, image_data))
}

struct CpuStateCtx<'a> {
    coefs: &'a [Coefficient],
    nchannel_u32: u32,
    max_w: u32,
    max_h: u32,
    count: usize,
    count_u32: u32,
}

fn prepare_cpu_state(ctx: &CpuStateCtx<'_>) -> Result<CpuPrepared, GpuError> {
    let coefs = ctx.coefs;
    let count = ctx.count;
    let max_w = ctx.max_w;
    let max_h = ctx.max_h;
    let layout = compute_aux_layout(coefs, ctx.nchannel_u32, ctx.count_u32)?;
    let chan_len = to_usize(CH_FIELDS)?
        .checked_mul(count)
        .ok_or_else(|| GpuError::Runtime("channel buffer length overflow".into()))?;
    let mut chan_data: Vec<Vec<f32>> = Vec::with_capacity(coefs.len());
    let mut aux_data = vec![0.0f32; layout.aux_len];
    let mut meta = [[0u32; 8]; NCHANNELS];
    let mut coef_off = 0usize;
    let mut max_comp_px = 0u32;
    let fdata_off = to_usize(F_FDATA)?
        .checked_mul(count)
        .ok_or_else(|| GpuError::Runtime("fdata offset overflow".into()))?;
    let fdata_end = fdata_off
        .checked_add(count)
        .ok_or_else(|| GpuError::Runtime("fdata end overflow".into()))?;
    let fista_off = to_usize(F_FISTA)?
        .checked_mul(count)
        .ok_or_else(|| GpuError::Runtime("fista offset overflow".into()))?;
    let cos_off = to_usize(F_COS)?
        .checked_mul(count)
        .ok_or_else(|| GpuError::Runtime("cos offset overflow".into()))?;
    let a_coefs_usize = to_usize(layout.a_coefs)?;
    let a_quant_usize = to_usize(layout.a_quant)?;
    for (c, coef) in coefs.iter().enumerate() {
        let rw = to_usize(coef.rounded_px_w)?;
        let rh = to_usize(coef.rounded_px_h)?;
        let blocks = to_usize(coef.block_count)?;
        let block_w = to_usize(coef.block_w)?;
        let px_len = rw
            .checked_mul(rh)
            .ok_or_else(|| GpuError::Runtime("channel pixel count overflow".into()))?;
        let coef_elems = blocks
            .checked_mul(64)
            .ok_or_else(|| GpuError::Runtime("block element count overflow".into()))?;
        if coef.dct_coefs.len() != coef_elems || px_len < coef_elems || block_w == 0 {
            return Err(GpuError::Runtime("invalid coefficient dimensions".into()));
        }
        let chan = build_channel_buffer(
            &prepare_channel_spatial(coef, rw, px_len, blocks, block_w)?,
            coef,
            (
                chan_len, fdata_off, fdata_end, fista_off, cos_off, max_w, max_h,
            ),
        )?;
        let coef_off_u32 = to_u32(coef_off)?;
        if let Some(m) = meta.get_mut(c) {
            *m = [
                coef.block_count,
                coef_off_u32,
                coef.rounded_px_w,
                coef.rounded_px_h,
                coef.block_w,
                coef.block_h,
                coef.horizontal_samp_factor.u32(),
                coef.vertical_samp_factor.u32(),
            ];
        }
        let aux_coef_start = a_coefs_usize
            .checked_add(coef_off)
            .ok_or_else(|| GpuError::Runtime("aux coefs start overflow".into()))?;
        let aux_coef_end = aux_coef_start
            .checked_add(coef_elems)
            .ok_or_else(|| GpuError::Runtime("aux coefs end overflow".into()))?;
        aux_data
            .get_mut(aux_coef_start..aux_coef_end)
            .ok_or_else(|| GpuError::Runtime("aux coefs slice out of bounds".into()))?
            .copy_from_slice(&coef.dct_coefs);
        let aux_quant_start = c
            .checked_mul(64)
            .and_then(|v| a_quant_usize.checked_add(v))
            .ok_or_else(|| GpuError::Runtime("aux quant start overflow".into()))?;
        let aux_quant_end = aux_quant_start
            .checked_add(64)
            .ok_or_else(|| GpuError::Runtime("aux quant end overflow".into()))?;
        aux_data
            .get_mut(aux_quant_start..aux_quant_end)
            .ok_or_else(|| GpuError::Runtime("aux quant slice out of bounds".into()))?
            .copy_from_slice(&coef.quant_table);
        coef_off = coef_off
            .checked_add(coef_elems)
            .ok_or_else(|| GpuError::Runtime("coef_off overflow".into()))?;
        let comp_px = coef
            .rounded_px_w
            .checked_mul(coef.rounded_px_h)
            .ok_or_else(|| GpuError::Runtime("component pixel count overflow".into()))?;
        max_comp_px = max_comp_px.max(comp_px);
        chan_data.push(chan);
    }
    Ok(CpuPrepared {
        layout,
        chan_len,
        chan_data,
        aux_data,
        meta,
        max_comp_px,
    })
}

fn build_channel_buffer(
    (cos, image_data): &(Vec<f32>, Vec<f32>),
    coef: &Coefficient,
    (chan_len, fdata_off, fdata_end, fista_off, cos_off, max_w, max_h): (
        usize,
        usize,
        usize,
        usize,
        usize,
        u32,
        u32,
    ),
) -> Result<Vec<f32>, GpuError> {
    let mut chan = vec![0.0f32; chan_len];
    let fdata_slice = chan
        .get_mut(fdata_off..fdata_end)
        .ok_or_else(|| GpuError::Runtime("fdata slice out of bounds".into()))?;
    upsample_pixels(
        image_data,
        coef.rounded_px_w,
        coef.rounded_px_h,
        coef.horizontal_samp_factor.usize(),
        coef.vertical_samp_factor.usize(),
        max_w,
        max_h,
        fdata_slice,
    );
    let cos_end = cos_off
        .checked_add(cos.len())
        .ok_or_else(|| GpuError::Runtime("cos end overflow".into()))?;
    chan.get_mut(cos_off..cos_end)
        .ok_or_else(|| GpuError::Runtime("cos slice out of bounds".into()))?
        .copy_from_slice(cos);
    chan.copy_within(fdata_off..fdata_end, fista_off);
    Ok(chan)
}

fn build_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("artefact-solver-bgl"),
        entries: &layout_entries,
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

struct IterationArgs<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    bind_group: &'a wgpu::BindGroup,
    pipelines: &'a Pipelines,
    params_buf: &'a wgpu::Buffer,
    wg_px: u32,
    wg_count: u32,
    comp_wg: u32,
    num_wg: u32,
    max_block_count: u32,
    nchannel_u32: u32,
}

fn run_iteration(args: &IterationArgs<'_>, params: &mut Params, it: usize, cur: u32, scratch: u32) {
    let term = fista_term(it);
    let next = f32::midpoint(1.0, 4.0_f32.mul_add(term * term, 1.0).sqrt());
    params.factor = (term - 1.0) / next;
    params.fdata_field = cur;
    params.fista_field = scratch;
    args.queue
        .write_buffer(args.params_buf, 0, bytemuck::bytes_of(params));

    {
        let mut encoder = args
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("artefact-fista"),
            });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_bind_group(0, args.bind_group, &[]);
        pass.set_pipeline(&args.pipelines.fista);
        pass.dispatch_workgroups(args.wg_px, 1, 1);
        drop(pass);
        args.queue.submit(Some(encoder.finish()));
    }

    // Promote the FISTA result to the current iterate.
    params.fdata_field = scratch;
    params.fista_field = cur;
    args.queue
        .write_buffer(args.params_buf, 0, bytemuck::bytes_of(params));

    let mut encoder = args
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("artefact-iteration"),
        });
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    pass.set_bind_group(0, args.bind_group, &[]);

    dispatch(&mut pass, &args.pipelines.tv_prepare, args.wg_count);
    dispatch(&mut pass, &args.pipelines.tv_apply, args.wg_px);
    dispatch_blocks(
        &mut pass,
        &args.pipelines.dct_gradient,
        args.max_block_count,
        args.nchannel_u32,
    );
    dispatch(&mut pass, &args.pipelines.tgv_prepare, args.wg_count);
    dispatch(&mut pass, &args.pipelines.tgv_apply, args.wg_px);

    pass.set_pipeline(&args.pipelines.reduce1);
    pass.dispatch_workgroups(args.num_wg, args.nchannel_u32, 1);
    pass.set_pipeline(&args.pipelines.reduce2);
    pass.dispatch_workgroups(args.nchannel_u32, 1, 1);

    dispatch(&mut pass, &args.pipelines.descent, args.wg_px);

    pass.set_pipeline(&args.pipelines.resample_down);
    pass.dispatch_workgroups(args.comp_wg, args.nchannel_u32, 1);
    dispatch_blocks(
        &mut pass,
        &args.pipelines.block_transform,
        args.max_block_count,
        args.nchannel_u32,
    );
    pass.set_pipeline(&args.pipelines.resample_up);
    pass.dispatch_workgroups(args.comp_wg, args.nchannel_u32, 1);
    drop(pass);
    args.queue.submit(Some(encoder.finish()));
}

struct GpuResources {
    chan_bufs: [wgpu::Buffer; NCHANNELS],
    params: Params,
    params_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipelines: Pipelines,
    wg_px: u32,
    wg_count: u32,
    comp_wg: u32,
    num_wg: u32,
    max_block_count: u32,
    nchannel_u32: u32,
}

struct SetupContext<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    coefs: &'a [Coefficient],
    prepared: &'a CpuPrepared,
    weight: f32,
    pweight: &'a [f32; 3],
    iterations: usize,
    max_w: u32,
    max_h: u32,
    count_u32: u32,
    nchannel_u32: u32,
}

fn build_uniform_params(ctx: &SetupContext<'_>) -> Params {
    let layout = &ctx.prepared.layout;
    let channel_count_f = u32_to_f32(ctx.nchannel_u32);
    let pixel_count_f = u32_to_f32(ctx.count_u32);
    let iterations_f = u32_to_f32(u32::try_from(ctx.iterations).unwrap_or(u32::MAX));
    Params {
        count: ctx.count_u32,
        max_w: ctx.max_w,
        max_h: ctx.max_h,
        nchannel: ctx.nchannel_u32,
        fdata_field: F_FDATA,
        fista_field: F_FISTA,
        a_norm: layout.a_norm,
        a_mean: layout.a_mean,
        a_coefs: layout.a_coefs,
        a_quant: layout.a_quant,
        a_partials: layout.a_partials,
        a_l2: layout.a_l2,
        num_wg: layout.num_wg,
        tv_alpha: 1.0 / channel_count_f.sqrt(),
        tgv_alpha: (ctx.weight / 2.0_f32.sqrt()) / channel_count_f.sqrt(),
        step_size: pixel_count_f.sqrt() / 2.0 / (1.0 + iterations_f).sqrt(),
        factor: 0.0,
        dct_alpha0: ctx.pweight[0] * 2.0 * 255.0 * 2.0_f32.sqrt(),
        dct_alpha1: ctx.pweight[1] * 2.0 * 255.0 * 2.0_f32.sqrt(),
        dct_alpha2: ctx.pweight[2] * 2.0 * 255.0 * 2.0_f32.sqrt(),
    }
}

fn setup_gpu_resources(ctx: &SetupContext<'_>) -> Result<GpuResources, GpuError> {
    let device = ctx.device;
    let queue = ctx.queue;
    let prepared = ctx.prepared;
    let layout = &prepared.layout;
    let bind_group_layout = build_bind_group_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("artefact-solver-pl"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let chan_bytes = to_u64(prepared.chan_len)?
        .checked_mul(4)
        .ok_or_else(|| GpuError::Runtime("channel byte size overflow".into()))?;
    let chan_bufs = create_chan_buffers(device, prepared, chan_bytes);
    let aux_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("artefact-aux"),
        contents: bytemuck::cast_slice(&prepared.aux_data),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
    });
    let meta_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("artefact-meta"),
        contents: bytemuck::cast_slice(&prepared.meta),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    let params = build_uniform_params(ctx);
    let params_size = to_u64(std::mem::size_of::<Params>())?;
    let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("artefact-params"),
        size: params_size,
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
    let pipelines = build_pipelines(device, &pipeline_layout);
    let max_block_count: u32 = ctx.coefs.iter().map(|c| c.block_count).max().unwrap_or(0);
    let total_px = ctx
        .count_u32
        .checked_mul(ctx.nchannel_u32)
        .ok_or_else(|| GpuError::Runtime("total pixel count overflow".into()))?;
    let wg_px = total_px.div_ceil(WORKGROUP);
    let wg_count = ctx.count_u32.div_ceil(WORKGROUP);
    let comp_wg = prepared.max_comp_px.div_ceil(WORKGROUP);
    Ok(GpuResources {
        chan_bufs,
        params,
        params_buf,
        bind_group,
        pipelines,
        wg_px,
        wg_count,
        comp_wg,
        num_wg: layout.num_wg,
        max_block_count,
        nchannel_u32: ctx.nchannel_u32,
    })
}

fn create_chan_buffers(
    device: &wgpu::Device,
    prepared: &CpuPrepared,
    chan_bytes: u64,
) -> [wgpu::Buffer; NCHANNELS] {
    std::array::from_fn(|i| {
        prepared.chan_data.get(i).map_or_else(
            || {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("artefact-chan-unused"),
                    size: chan_bytes,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            },
            |d| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("artefact-chan"),
                    contents: bytemuck::cast_slice(d),
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                })
            },
        )
    })
}

/// Solve on the GPU, returning one `AlignedF32` (pixel data) per component.
///
/// # Errors
/// Propagates any GPU error while building buffers/pipelines or running.
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
        let queue = ctx.queue();
        let coefs = coefs.get(..coefs.len().min(NCHANNELS)).unwrap_or(coefs);
        let nchannel = coefs.len();
        let nchannel_u32 = to_u32(nchannel)?;
        let count = usize::try_from(max_w.saturating_mul(max_h))
            .map_err(|_| GpuError::Runtime("pixel count overflow".into()))?;
        let count_u32 = to_u32(count)?;
        let prepared = prepare_cpu_state(&CpuStateCtx {
            coefs,
            nchannel_u32,
            max_w,
            max_h,
            count,
            count_u32,
        })?;
        let mut res = setup_gpu_resources(&SetupContext {
            device,
            queue,
            coefs,
            prepared: &prepared,
            weight,
            pweight,
            iterations,
            max_w,
            max_h,
            count_u32,
            nchannel_u32,
        })?;
        // The current iterate ping-pongs between the two state fields: FISTA reads
        // `cur` and writes `scratch`, then the remaining passes operate on the
        // freshly written `scratch`.
        let mut cur = F_FDATA;
        let mut scratch = F_FISTA;

        let iter_args = IterationArgs {
            device,
            queue,
            bind_group: &res.bind_group,
            pipelines: &res.pipelines,
            params_buf: &res.params_buf,
            wg_px: res.wg_px,
            wg_count: res.wg_count,
            comp_wg: res.comp_wg,
            num_wg: res.num_wg,
            max_block_count: res.max_block_count,
            nchannel_u32: res.nchannel_u32,
        };

        for it in 0..iterations {
            run_iteration(&iter_args, &mut res.params, it, cur, scratch);
            std::mem::swap(&mut cur, &mut scratch);
        }

        readback_channels(device, queue, &res.chan_bufs, nchannel, cur, count).await
    })
}

fn readback_channels<'a>(
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    chan_bufs: &'a [wgpu::Buffer; NCHANNELS],
    nchannel: usize,
    read_field: u32,
    count: usize,
) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<AlignedF32>, GpuError>> + 'a>> {
    Box::pin(async move {
        let mut out = Vec::with_capacity(nchannel);
        let readback_size = to_u64(count)?
            .checked_mul(4)
            .ok_or_else(|| GpuError::Runtime("readback byte size overflow".into()))?;
        let readback_offset = u64::from(read_field)
            .checked_mul(readback_size)
            .ok_or_else(|| GpuError::Runtime("readback offset overflow".into()))?;
        for buf in chan_bufs.iter().take(nchannel) {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("artefact-readback"),
            });
            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("artefact-staging"),
                size: readback_size,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(buf, readback_offset, &staging, 0, readback_size);
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

fn dispatch(pass: &mut wgpu::ComputePass<'_>, pipeline: &wgpu::ComputePipeline, workgroups: u32) {
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(workgroups, 1, 1);
}

/// One 64-lane workgroup per block, laid out as `(max_blocks, nchannel)`.
fn dispatch_blocks(
    pass: &mut wgpu::ComputePass<'_>,
    pipeline: &wgpu::ComputePipeline,
    max_blocks: u32,
    nchannel: u32,
) {
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(max_blocks, nchannel, 1);
}

/// The FISTA `term` before iteration `it` (CPU `run_fista` recurrence).
fn fista_term(iterations: usize) -> f32 {
    let mut term = 1.0_f32;
    for _ in 0..iterations {
        term = f32::midpoint(1.0, 4.0_f32.mul_add(term * term, 1.0).sqrt());
    }
    term
}
