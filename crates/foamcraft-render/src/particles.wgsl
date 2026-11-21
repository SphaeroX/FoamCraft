// SPDX-License-Identifier: GPL-3.0-or-later
// Particle advection in the sampled velocity grids (the same algorithm as
// foamcraft_sim::particles): midpoint rule, finest grid containing the point, a masked corner
// blocks, PCG random numbers with a fixed seed. Writes two line vertices per particle (tail and
// head) and their colours, so the scene's line pipeline draws them.

struct Params {
    count: u32,
    levels: u32,
    seeds: u32,
    seed: u32,
    frame: u32,
    color_by: u32,
    dt: f32,
    max_age: f32,
    trail: f32,
    range_lo: f32,
    range_hi: f32,
    _pad: f32,
    constant: vec4<f32>,
    // cos and sin of the yaw: grids are in the flow frame, the drawing in the world
    rot: vec4<f32>,
};

struct Level {
    lo: vec4<f32>,
    hi: vec4<f32>,
    // x, y, z points and the offset into `field`
    dims: vec4<u32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> levels: array<Level>;
@group(0) @binding(2) var<storage, read> field: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> seeds: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> state: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read_write> verts: array<u32>;
@group(0) @binding(6) var<storage, read_write> colors: array<f32>;
@group(0) @binding(7) var<storage, read> pressure: array<f32>;

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let word = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random(i: u32, frame: u32, k: u32) -> f32 {
    let h = pcg(i ^ pcg(frame ^ pcg(params.seed + k)));
    return f32(h >> 8u) / 16777216.0;
}

// w: 1 velocity, 0 blocked, -1 outside
fn sample(p: vec3<f32>) -> vec4<f32> {
    var l = i32(params.levels) - 1;
    loop {
        if (l < 0) { break; }
        let lv = levels[u32(l)];
        let d = lv.dims.xyz;
        let step = (lv.hi.xyz - lv.lo.xyz) / vec3<f32>(d + vec3<u32>(1u));
        let c = (p - lv.lo.xyz) / step - vec3<f32>(1.0);
        let top = vec3<f32>(d - vec3<u32>(1u));
        if (all(c >= vec3<f32>(0.0)) && all(c <= top)) {
            let b = min(vec3<u32>(floor(c)), d - vec3<u32>(2u));
            let f = c - vec3<f32>(b);
            var u = vec3<f32>(0.0);
            for (var corner = 0u; corner < 8u; corner = corner + 1u) {
                let o = vec3<u32>(corner & 1u, (corner >> 1u) & 1u, (corner >> 2u) & 1u);
                let q = b + o;
                let v = field[lv.dims.w + (q.z * d.y + q.y) * d.x + q.x];
                if (v.w == 0.0) {
                    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
                }
                let w3 = select(vec3<f32>(1.0) - f, f, o == vec3<u32>(1u));
                u = u + w3.x * w3.y * w3.z * v.xyz;
            }
            return vec4<f32>(u, 1.0);
        }
        l = l - 1;
    }
    return vec4<f32>(0.0, 0.0, 0.0, -1.0);
}

// Trilinear pressure of the finest grid that contains p (0 outside or without pressure).
fn pressure_at(p: vec3<f32>) -> f32 {
    if (arrayLength(&pressure) < arrayLength(&field)) { return 0.0; }
    var l = i32(params.levels) - 1;
    loop {
        if (l < 0) { break; }
        let lv = levels[u32(l)];
        let d = lv.dims.xyz;
        let step = (lv.hi.xyz - lv.lo.xyz) / vec3<f32>(d + vec3<u32>(1u));
        let c = (p - lv.lo.xyz) / step - vec3<f32>(1.0);
        let top = vec3<f32>(d - vec3<u32>(1u));
        if (all(c >= vec3<f32>(0.0)) && all(c <= top)) {
            let b = min(vec3<u32>(floor(c)), d - vec3<u32>(2u));
            let f = c - vec3<f32>(b);
            var v = 0.0;
            for (var corner = 0u; corner < 8u; corner = corner + 1u) {
                let o = vec3<u32>(corner & 1u, (corner >> 1u) & 1u, (corner >> 2u) & 1u);
                let q = b + o;
                let w3 = select(vec3<f32>(1.0) - f, f, o == vec3<u32>(1u));
                v = v + w3.x * w3.y * w3.z * pressure[lv.dims.w + (q.z * d.y + q.y) * d.x + q.x];
            }
            return v;
        }
        l = l - 1;
    }
    return 0.0;
}

fn respawn(i: u32, frame: u32) -> vec4<f32> {
    let pick = pcg(i ^ pcg(frame ^ pcg(params.seed))) % max(params.seeds, 1u);
    return vec4<f32>(seeds[pick].xyz, 0.0);
}

// Linear sRGB of the FoamCraft colour scale (OKLab, as foamcraft_sim::results::colormap).
fn colormap(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    let l = 0.45 + 0.50 * x;
    let a = -0.06 * sin(3.14159265 * x);
    let b = -0.10 + 0.26 * x;
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;
    let rgb = vec3<f32>(
        4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
        -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
        -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3,
    );
    return clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn write(i: u32, s: vec4<f32>, u: vec3<f32>) {
    let head = s.xyz;
    let tail = head - u * params.trail;
    var c = params.constant.xyz;
    let span = max(params.range_hi - params.range_lo, 1e-30);
    if (params.color_by == 0u) {
        c = colormap((length(u) - params.range_lo) / span);
    } else if (params.color_by == 1u) {
        c = colormap(s.w / max(params.max_age, 1e-30));
    } else if (params.color_by == 3u) {
        c = colormap((pressure_at(head) - params.range_lo) / span);
    }
    var ends = array<vec3<f32>, 2>(tail, head);
    for (var e = 0u; e < 2u; e = e + 1u) {
        let base = (2u * i + e) * 7u;
        let f = ends[e];
        let p = vec3<f32>(
            params.rot.x * f.x - params.rot.y * f.y,
            params.rot.y * f.x + params.rot.x * f.y,
            f.z,
        );
        verts[base] = bitcast<u32>(p.x);
        verts[base + 1u] = bitcast<u32>(p.y);
        verts[base + 2u] = bitcast<u32>(p.z);
        verts[base + 3u] = bitcast<u32>(0.0);
        verts[base + 4u] = bitcast<u32>(0.0);
        verts[base + 5u] = bitcast<u32>(1.0);
        verts[base + 6u] = 0u;
        let shade = select(0.35, 1.0, e == 1u);
        colors[(2u * i + e) * 3u] = c.x * shade;
        colors[(2u * i + e) * 3u + 1u] = c.y * shade;
        colors[(2u * i + e) * 3u + 2u] = c.z * shade;
    }
}

@compute @workgroup_size(64)
fn init(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.count) { return; }
    var s = respawn(i, 0u);
    s.w = random(i, 0u, 7u) * params.max_age;
    state[i] = s;
    write(i, s, vec3<f32>(0.0));
}

@compute @workgroup_size(64)
fn advect(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.count) { return; }
    let s = state[i];
    let p = s.xyz;
    let dt = params.dt;
    var next = s;
    var ok = false;
    var u = vec3<f32>(0.0);
    let v1 = sample(p);
    if (v1.w > 0.5) {
        let v2 = sample(p + 0.5 * dt * v1.xyz);
        if (v2.w > 0.5) {
            let q = p + dt * v2.xyz;
            let age = s.w + dt;
            let v3 = sample(q);
            if (age <= params.max_age && v3.w > 0.5) {
                next = vec4<f32>(q, age);
                u = v3.xyz;
                ok = true;
            }
        }
    }
    if (!ok) {
        next = respawn(i, params.frame);
    }
    state[i] = next;
    write(i, next, u);
}
