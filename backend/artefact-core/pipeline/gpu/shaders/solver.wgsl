// GPU solver kernels. All entry points share one bind group:
//   0..2  per-channel state buffers (field `f` at `f * count`)
//   3     `aux` scratch (norm, mean, coefficient data, quant tables, reductions)
//   4     per-channel metadata (stride 8)
//   5     `params` uniform scalars
//
// Layout must match `Params` / `meta` offsets in `pipeline/gpu/solver.rs`.

const F_FDATA: u32 = 0u;
const F_FISTA: u32 = 1u;
const F_OBJ: u32 = 2u;
const F_PDX: u32 = 3u;
const F_PDY: u32 = 4u;
const F_HXX: u32 = 5u;
const F_HXY: u32 = 6u;
const F_HYY: u32 = 7u;
const F_COS: u32 = 8u;

// meta stride
const M_BLOCKS: u32 = 0u; // block_count
const M_COEF: u32 = 1u;   // float offset into `aux`
const M_RW: u32 = 2u;     // rounded width
const M_RH: u32 = 3u;     // rounded height
const M_BW: u32 = 4u;     // block width
const M_BH: u32 = 5u;     // block height
const M_HF: u32 = 6u;     // horizontal sampling factor
const M_VF: u32 = 7u;     // vertical sampling factor
const M_STRIDE: u32 = 8u;

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
};

@group(0) @binding(0) var<storage, read_write> c0: array<f32>;
@group(0) @binding(1) var<storage, read_write> c1: array<f32>;
@group(0) @binding(2) var<storage, read_write> c2: array<f32>;
@group(0) @binding(3) var<storage, read_write> aux: array<f32>;
@group(0) @binding(4) var<storage, read> mdata: array<u32>;
@group(0) @binding(5) var<uniform> p: Params;

fn ld(c: u32, i: u32) -> f32 {
    if (c == 0u) { return c0[i]; }
    if (c == 1u) { return c1[i]; }
    return c2[i];
}

fn st(c: u32, i: u32, v: f32) {
    if (c == 0u) { c0[i] = v; }
    else if (c == 1u) { c1[i] = v; }
    else { c2[i] = v; }
}

fn cfield(_c: u32, f: u32, i: u32) -> u32 {
    // Each channel lives in its own storage buffer, so the index is only the
    // field offset within that buffer.
    return f * p.count + i;
}

fn m(c: u32, k: u32) -> u32 {
    return mdata[c * M_STRIDE + k];
}

fn safe_div(a: f32, d: f32) -> f32 {
    return select(0.0, a / d, d != 0.0);
}

// ---------------------------------------------------------------------------
// 8x8 orthonormal DCT-II / DCT-III, separable, in workgroup memory.
// ---------------------------------------------------------------------------

var<workgroup> blk_a: array<f32, 64>;
var<workgroup> blk_b: array<f32, 64>;
var<workgroup> red: array<f32, 256>;

fn dct2d(t: u32) {
    let quarter_pi = 0.19634954084936207; // pi / 16
    // rows: blk_a -> blk_b
    if (t < 64u) {
        let y = t / 8u;
        let k = t % 8u;
        var s = 0.0;
        for (var n = 0u; n < 8u; n = n + 1u) {
            let ang = f32(2u * n + 1u) * f32(k) * quarter_pi;
            s = s + blk_a[y * 8u + n] * cos(ang);
        }
        let ck = select(1.0, 0.7071067811865475, k == 0u);
        blk_b[y * 8u + k] = 0.5 * ck * s;
    }
    workgroupBarrier();
    // cols: blk_b -> blk_a
    if (t < 64u) {
        let u = t / 8u;
        let v = t % 8u;
        var s = 0.0;
        for (var j = 0u; j < 8u; j = j + 1u) {
            let ang = f32(2u * j + 1u) * f32(u) * quarter_pi;
            s = s + blk_b[j * 8u + v] * cos(ang);
        }
        let cu = select(1.0, 0.7071067811865475, u == 0u);
        blk_a[u * 8u + v] = 0.5 * cu * s;
    }
    workgroupBarrier();
}

fn idct2d(t: u32) {
    let quarter_pi = 0.19634954084936207;
    // rows: blk_a -> blk_b (transpose basis)
    if (t < 64u) {
        let y = t / 8u;
        let n = t % 8u;
        var s = 0.0;
        for (var k = 0u; k < 8u; k = k + 1u) {
            let ang = f32(2u * n + 1u) * f32(k) * quarter_pi;
            let ck = select(1.0, 0.7071067811865475, k == 0u);
            s = s + ck * blk_a[y * 8u + k] * cos(ang);
        }
        blk_b[y * 8u + n] = 0.5 * s;
    }
    workgroupBarrier();
    // cols: blk_b -> blk_a
    if (t < 64u) {
        let m = t / 8u;
        let v = t % 8u;
        var s = 0.0;
        for (var k = 0u; k < 8u; k = k + 1u) {
            let ang = f32(2u * m + 1u) * f32(k) * quarter_pi;
            let ck = select(1.0, 0.7071067811865475, k == 0u);
            s = s + ck * blk_b[k * 8u + v] * cos(ang);
        }
        blk_a[m * 8u + v] = 0.5 * s;
    }
    workgroupBarrier();
}

// ---------------------------------------------------------------------------
// FISTA momentum
// ---------------------------------------------------------------------------

@compute @workgroup_size(256)
fn fista(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= p.count * p.nchannel) { return; }
    let c = idx / p.count;
    let i = idx % p.count;
    let fd = ld(c, cfield(c, p.fdata_field, i));
    let fp = ld(c, cfield(c, p.fista_field, i));
    st(c, cfield(c, p.fista_field, i), fd + p.factor * (fd - fp));
}

// ---------------------------------------------------------------------------
// TV
// ---------------------------------------------------------------------------

@compute @workgroup_size(256)
fn tv_prepare(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= p.count) { return; }
    let x = i % p.max_w;
    let y = i / p.max_w;
    var sum = 0.0;
    for (var c = 0u; c < p.nchannel; c = c + 1u) {
        let base = cfield(c, p.fdata_field, i);
        var gx = 0.0;
        if (x + 1u < p.max_w) { gx = ld(c, base + 1u) - ld(c, base); }
        var gy = 0.0;
        if (y + 1u < p.max_h) { gy = ld(c, base + p.max_w) - ld(c, base); }
        st(c, cfield(c, F_PDX, i), gx);
        st(c, cfield(c, F_PDY, i), gy);
        sum = sum + gx * gx + gy * gy;
    }
    aux[p.a_norm + i] = sqrt(sum);
}

@compute @workgroup_size(256)
fn tv_apply(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= p.count * p.nchannel) { return; }
    let c = idx / p.count;
    let i = idx % p.count;
    let x = i % p.max_w;
    let y = i / p.max_w;

    let gx = ld(c, cfield(c, F_PDX, i));
    let gy = ld(c, cfield(c, F_PDY, i));
    let n = aux[p.a_norm + i];

    var lx = 0.0;
    var ln = 0.0;
    if (x > 0u) {
        lx = ld(c, cfield(c, F_PDX, i - 1u));
        ln = aux[p.a_norm + i - 1u];
    }
    var uy = 0.0;
    var un = 0.0;
    if (y > 0u) {
        uy = ld(c, cfield(c, F_PDY, i - p.max_w));
        un = aux[p.a_norm + i - p.max_w];
    }

    let v = p.tv_alpha * (
        safe_div(-(gx + gy), n) + safe_div(lx, ln) + safe_div(uy, un)
    );
    st(c, cfield(c, F_OBJ, i), v);
}

// ---------------------------------------------------------------------------
// DCT data-fidelity gradient (one workgroup per 8x8 block)
// ---------------------------------------------------------------------------

@compute @workgroup_size(64)
fn dct_gradient(@builtin(workgroup_id) wid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let c = wid.y;
    let local = wid.x;
    if (local >= m(c, M_BLOCKS)) { return; }
    let bw = m(c, M_BW);
    let bx = local % bw;
    let by = local / bw;
    let hf = m(c, M_HF);
    let vf = m(c, M_VF);
    let coef_base = p.a_coefs + m(c, M_COEF) + local * 64u;
    let q_base = p.a_quant + c * 64u;
    let cos_base = cfield(c, F_COS, local * 64u);

    let t = lid.x;
    let q = aux[q_base + t];
    blk_a[t] = (ld(c, cos_base + t) - aux[coef_base + t] * q) / (q * q);
    workgroupBarrier();
    idct2d(t);

    let alpha = select(select(p.dct_alpha2, p.dct_alpha1, c == 1u), p.dct_alpha0, c == 0u);
    let in_y = t / 8u;
    let in_x = t % 8u;
    let cy = by * 8u + in_y;
    let cx = bx * 8u + in_x;
    let val = alpha * blk_a[t];
    for (var sy = 0u; sy < vf; sy = sy + 1u) {
        for (var sx = 0u; sx < hf; sx = sx + 1u) {
            let yy = cy * vf + sy;
            let xx = cx * hf + sx;
            if (xx < p.max_w && yy < p.max_h) {
                let dst = cfield(c, F_OBJ, yy * p.max_w + xx);
                st(c, dst, ld(c, dst) + val);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TGV
// ---------------------------------------------------------------------------

@compute @workgroup_size(256)
fn tgv_prepare(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= p.count) { return; }
    let x = i % p.max_w;
    let y = i / p.max_w;
    var sum = 0.0;
    for (var c = 0u; c < p.nchannel; c = c + 1u) {
        var gxx = 0.0;
        var gyx = 0.0;
        if (x > 0u) {
            gxx = ld(c, cfield(c, F_PDX, i)) - ld(c, cfield(c, F_PDX, i - 1u));
            gyx = ld(c, cfield(c, F_PDY, i)) - ld(c, cfield(c, F_PDY, i - 1u));
        }
        var gxy = 0.0;
        var gyy = 0.0;
        if (y > 0u) {
            gxy = ld(c, cfield(c, F_PDX, i)) - ld(c, cfield(c, F_PDX, i - p.max_w));
            gyy = ld(c, cfield(c, F_PDY, i)) - ld(c, cfield(c, F_PDY, i - p.max_w));
        }
        let gxy_sym = 0.5 * (gxy + gyx);
        st(c, cfield(c, F_HXX, i), gxx);
        st(c, cfield(c, F_HXY, i), gxy_sym);
        st(c, cfield(c, F_HYY, i), gyy);
        sum = sum + gxx * gxx + 2.0 * gxy_sym * gxy_sym + gyy * gyy;
    }
    aux[p.a_norm + i] = sqrt(sum);
}

@compute @workgroup_size(256)
fn tgv_apply(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= p.count * p.nchannel) { return; }
    let c = idx / p.count;
    let i = idx % p.count;
    let x = i % p.max_w;
    let y = i / p.max_w;

    let gxx = ld(c, cfield(c, F_HXX, i));
    let gxy = ld(c, cfield(c, F_HXY, i));
    let gyy = ld(c, cfield(c, F_HYY, i));
    let n = aux[p.a_norm + i];
    var v = safe_div(-(2.0 * gxx + 2.0 * gxy + 2.0 * gyy), n);

    if (x > 0u) {
        v = v + safe_div(ld(c, cfield(c, F_HXY, i - 1u)) + ld(c, cfield(c, F_HXX, i - 1u)), aux[p.a_norm + i - 1u]);
    }
    if (x + 1u < p.max_w) {
        v = v + safe_div(ld(c, cfield(c, F_HXY, i + 1u)) + ld(c, cfield(c, F_HXX, i + 1u)), aux[p.a_norm + i + 1u]);
    }
    if (y > 0u) {
        v = v + safe_div(ld(c, cfield(c, F_HYY, i - p.max_w)) + ld(c, cfield(c, F_HXY, i - p.max_w)), aux[p.a_norm + i - p.max_w]);
    }
    if (y + 1u < p.max_h) {
        v = v + safe_div(ld(c, cfield(c, F_HYY, i + p.max_w)) + ld(c, cfield(c, F_HXY, i + p.max_w)), aux[p.a_norm + i + p.max_w]);
    }
    if (x + 1u < p.max_w && y > 0u) {
        v = v + safe_div(-ld(c, cfield(c, F_HXY, i + 1u - p.max_w)), aux[p.a_norm + i + 1u - p.max_w]);
    }
    if (x > 0u && y + 1u < p.max_h) {
        v = v + safe_div(-ld(c, cfield(c, F_HXY, i - 1u + p.max_w)), aux[p.a_norm + i - 1u + p.max_w]);
    }

    let dst = cfield(c, F_OBJ, i);
    st(c, dst, ld(c, dst) + p.tgv_alpha * v);
}

// ---------------------------------------------------------------------------
// Descent (with a two-stage per-channel L2 reduction)
// ---------------------------------------------------------------------------

@compute @workgroup_size(256)
fn reduce1(@builtin(workgroup_id) wid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let chunk = wid.x;
    let c = wid.y;
    var sum = 0.0;
    var i = chunk * 256u + lid.x;
    while (i < p.count) {
        let v = ld(c, cfield(c, F_OBJ, i));
        sum = sum + v * v;
        i = i + p.num_wg * 256u;
    }
    // Reduce within the workgroup via shared memory.
    red[lid.x] = sum;
    workgroupBarrier();
    var stride = 128u;
    while (stride > 0u) {
        if (lid.x < stride) { red[lid.x] = red[lid.x] + red[lid.x + stride]; }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid.x == 0u) {
        aux[p.a_partials + c * p.num_wg + chunk] = red[0];
    }
}

@compute @workgroup_size(256)
fn reduce2(@builtin(workgroup_id) wid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let c = wid.x;
    var sum = 0.0;
    var k = lid.x;
    while (k < p.num_wg) {
        sum = sum + aux[p.a_partials + c * p.num_wg + k];
        k = k + 256u;
    }
    red[lid.x] = sum;
    workgroupBarrier();
    var stride = 128u;
    while (stride > 0u) {
        if (lid.x < stride) { red[lid.x] = red[lid.x] + red[lid.x + stride]; }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid.x == 0u) {
        aux[p.a_l2 + c] = sqrt(red[0]);
    }
}

@compute @workgroup_size(256)
fn descent(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= p.count * p.nchannel) { return; }
    let c = idx / p.count;
    let i = idx % p.count;
    let n = aux[p.a_l2 + c];
    if (n != 0.0) {
        let fd = ld(c, cfield(c, p.fdata_field, i));
        let g = ld(c, cfield(c, F_OBJ, i));
        st(c, cfield(c, p.fdata_field, i), fd - p.step_size * (g / n));
    }
}

// ---------------------------------------------------------------------------
// Projection: resample down -> block DCT/clamp/IDCT -> resample up
// ---------------------------------------------------------------------------

@compute @workgroup_size(256)
fn resample_down(@builtin(global_invocation_id) gid: vec3<u32>) {
    let wid = gid.x;
    // 2D: x = component pixel, y = channel
    let c = gid.y;
    if (c >= p.nchannel) { return; }
    let rw = m(c, M_RW);
    let rh = m(c, M_RH);
    if (wid >= rw * rh) { return; }
    // Full-resolution components are not resampled; leave `fdata` untouched.
    if (rw == p.max_w && rh == p.max_h) { return; }
    let hf = m(c, M_HF);
    let vf = m(c, M_VF);
    let cx = wid % rw;
    let cy = wid / rw;
    var sum = 0.0;
    for (var sy = 0u; sy < vf; sy = sy + 1u) {
        for (var sx = 0u; sx < hf; sx = sx + 1u) {
            let y = cy * vf + sy;
            let x = cx * hf + sx;
            if (x < p.max_w && y < p.max_h) {
                sum = sum + ld(c, cfield(c, p.fdata_field, y * p.max_w + x));
            }
        }
    }
    let mean = sum / f32(hf * vf);
    let mi = p.a_mean + c * p.count + cy * rw + cx;
    aux[mi] = mean;
    for (var sy = 0u; sy < vf; sy = sy + 1u) {
        for (var sx = 0u; sx < hf; sx = sx + 1u) {
            let y = cy * vf + sy;
            let x = cx * hf + sx;
            if (x < p.max_w && y < p.max_h) {
                let idx = cfield(c, p.fdata_field, y * p.max_w + x);
                st(c, idx, ld(c, idx) - mean);
            }
        }
    }
}

@compute @workgroup_size(64)
fn block_transform(@builtin(workgroup_id) wid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let c = wid.y;
    let local = wid.x;
    if (local >= m(c, M_BLOCKS)) { return; }
    let rw = m(c, M_RW);
    let rh = m(c, M_RH);
    let bw = m(c, M_BW);
    let hf = m(c, M_HF);
    let vf = m(c, M_VF);
    let resample = rw != p.max_w || rh != p.max_h;
    let bx = local % bw;
    let by = local / bw;
    let t = lid.x;
    let in_y = t / 8u;
    let in_x = t % 8u;

    if (resample) {
        let idx = p.a_mean + c * p.count + (by * 8u + in_y) * rw + (bx * 8u + in_x);
        blk_a[t] = aux[idx];
    } else {
        let x = bx * 8u + in_x;
        let y = by * 8u + in_y;
        blk_a[t] = ld(c, cfield(c, p.fdata_field, y * p.max_w + x));
    }
    workgroupBarrier();

    dct2d(t);

    let coef_base = p.a_coefs + m(c, M_COEF) + local * 64u;
    let q_base = p.a_quant + c * 64u;
    let q = aux[q_base + t];
    let dc = aux[coef_base + t];
    blk_a[t] = clamp(blk_a[t], (dc - 0.5) * q, (dc + 0.5) * q);
    let cos_base = cfield(c, F_COS, local * 64u);
    st(c, cos_base + t, blk_a[t]);
    workgroupBarrier();

    idct2d(t);

    if (resample) {
        let idx = p.a_mean + c * p.count + (by * 8u + in_y) * rw + (bx * 8u + in_x);
        aux[idx] = blk_a[t];
    } else {
        let x = bx * 8u + in_x;
        let y = by * 8u + in_y;
        st(c, cfield(c, p.fdata_field, y * p.max_w + x), blk_a[t]);
    }
}

@compute @workgroup_size(256)
fn resample_up(@builtin(global_invocation_id) gid: vec3<u32>) {
    let wid = gid.x;
    let c = gid.y;
    if (c >= p.nchannel) { return; }
    let rw = m(c, M_RW);
    let rh = m(c, M_RH);
    if (wid >= rw * rh) { return; }
    if (rw == p.max_w && rh == p.max_h) { return; }
    let hf = m(c, M_HF);
    let vf = m(c, M_VF);
    let cx = wid % rw;
    let cy = wid / rw;
    let mean = aux[p.a_mean + c * p.count + cy * rw + cx];
    for (var sy = 0u; sy < vf; sy = sy + 1u) {
        for (var sx = 0u; sx < hf; sx = sx + 1u) {
            let y = cy * vf + sy;
            let x = cx * hf + sx;
            if (x < p.max_w && y < p.max_h) {
                let idx = cfield(c, p.fdata_field, y * p.max_w + x);
                st(c, idx, ld(c, idx) + mean);
            }
        }
    }
}
