// SPDX-License-Identifier: GPL-3.0-or-later
// Heavy particles (Task_34): like particles.wgsl, but each particle has its own velocity. Drag
// pulls it towards the flow (exact step for a locally constant flow, Schiller and Naumann
// correction), gravity minus buoyancy pulls it down. A distance field per grid point (negative
// inside solids) finds the impact between two steps; the particle sticks (shown as a short
// bright line along the normal, then starts again) or bounces. Same CPU reference:
// foamcraft_sim::heavy_gpu.

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
    rot: vec4<f32>,
    clip: vec4<f32>,
};

struct Level {
    lo: vec4<f32>,
    hi: vec4<f32>,
    dims: vec4<u32>,
};

struct Heavy {
    // Response time in creeping flow (s), diameter (m), kinematic viscosity (m²/s), 1 = stick.
    tau_p: f32,
    diameter: f32,
    nu: f32,
    stick: u32,
    // Gravity minus buoyancy (flow frame), w unused.
    gravity: vec4<f32>,
    restitution: f32,
    friction: f32,
    // Flow seconds a stuck particle stays before it starts again.
    stick_time: f32,
    // Length of the stuck mark along the normal, m.
    mark: f32,
    // Settling velocity in still medium (flow frame), w unused.
    settle: vec4<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> levels: array<Level>;
@group(0) @binding(2) var<storage, read> field: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> seeds: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> state: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read_write> verts: array<u32>;
@group(0) @binding(6) var<storage, read_write> colors: array<f32>;
// The distance field takes the place of the pressure (at most 8 storage buffers per stage).
@group(0) @binding(7) var<storage, read> dist: array<f32>;
// Velocity xyz; w: < 0 flying (-1 - bounces), >= 0 stuck for that many flow seconds.
@group(0) @binding(8) var<storage, read_write> motion: array<vec4<f32>>;
@group(0) @binding(9) var<uniform> heavy: Heavy;

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let word = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random(i: u32, frame: u32, k: u32) -> f32 {
    let h = pcg(i ^ pcg(frame ^ pcg(params.seed + k)));
    return f32(h >> 8u) / 16777216.0;
}

// Velocity of the finest grid containing p: w 1 inside (masked corners count as resting wall,
// so the flow goes to zero towards a solid), -1 outside every grid.
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
                let w3 = select(vec3<f32>(1.0) - f, f, o == vec3<u32>(1u));
                u = u + w3.x * w3.y * w3.z * v.xyz * v.w;
            }
            return vec4<f32>(u, 1.0);
        }
        l = l - 1;
    }
    return vec4<f32>(0.0, 0.0, 0.0, -1.0);
}

// Distance to the surface (negative inside), trilinear in the finest grid; large outside.
fn distance_at(p: vec3<f32>) -> f32 {
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
                v = v + w3.x * w3.y * w3.z * dist[lv.dims.w + (q.z * d.y + q.y) * d.x + q.x];
            }
            return v;
        }
        l = l - 1;
    }
    return 1e30;
}

fn normal_at(p: vec3<f32>, e: f32) -> vec3<f32> {
    let g = vec3<f32>(
        distance_at(p + vec3<f32>(e, 0.0, 0.0)) - distance_at(p - vec3<f32>(e, 0.0, 0.0)),
        distance_at(p + vec3<f32>(0.0, e, 0.0)) - distance_at(p - vec3<f32>(0.0, e, 0.0)),
        distance_at(p + vec3<f32>(0.0, 0.0, e)) - distance_at(p - vec3<f32>(0.0, 0.0, e)),
    );
    let n = length(g);
    if (n < 1e-30) { return vec3<f32>(0.0, 0.0, 1.0); }
    return g / n;
}

fn drag_factor(re: f32) -> f32 {
    if (re <= 1000.0) {
        return 1.0 + 0.15 * pow(max(re, 0.0), 0.687);
    }
    return 0.44 * re / 24.0;
}

fn tau_e(rel: f32) -> f32 {
    return heavy.tau_p / drag_factor(rel * heavy.diameter / max(heavy.nu, 1e-30));
}

struct Moved {
    x: vec3<f32>,
    v: vec3<f32>,
};

fn exact(x: vec3<f32>, v: vec3<f32>, u: vec3<f32>, tau: f32, dt: f32) -> Moved {
    let g = heavy.gravity.xyz;
    let v_inf = u + g * tau;
    if (tau <= 1e-30) {
        return Moved(x + v_inf * dt, v_inf);
    }
    let e = exp(-dt / tau);
    return Moved(x + v_inf * dt + (v - v_inf) * tau * (1.0 - e), v_inf + (v - v_inf) * e);
}

fn respawn(i: u32, frame: u32) -> vec4<f32> {
    let pick = pcg(i ^ pcg(frame ^ pcg(params.seed))) % max(params.seeds, 1u);
    return vec4<f32>(seeds[pick].xyz, 0.0);
}

// Start velocity: the flow plus the settling velocity.
fn start_velocity(p: vec3<f32>) -> vec3<f32> {
    let u = sample(p);
    return u.xyz + heavy.settle.xyz;
}

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

fn world(f: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        params.rot.x * f.x - params.rot.y * f.y,
        params.rot.y * f.x + params.rot.x * f.y,
        f.z,
    );
}

// Tail along the particle's own velocity; a stuck particle is a bright mark along the normal.
fn write(i: u32, s: vec4<f32>, m: vec4<f32>, u: vec3<f32>) {
    let head = s.xyz;
    var tail = head - m.xyz * params.trail;
    var c = params.constant.xyz;
    let span = max(params.range_hi - params.range_lo, 1e-30);
    if (params.color_by == 0u) {
        c = colormap((length(m.xyz) - params.range_lo) / span);
    } else if (params.color_by == 1u) {
        c = colormap(s.w / max(params.max_age, 1e-30));
    } else if (params.color_by == 4u) {
        c = colormap((length(u - m.xyz) - params.range_lo) / span);
    }
    if (m.w >= 0.0) {
        tail = head + normal_at(head, heavy.mark) * heavy.mark;
        let fade = clamp(1.0 - m.w / max(heavy.stick_time, 1e-30), 0.0, 1.0);
        c = mix(c, vec3<f32>(1.0, 0.95, 0.7), 0.7) * (0.4 + 0.6 * fade);
    }
    var ends = array<vec3<f32>, 2>(tail, head);
    if (any(params.clip.xyz != vec3<f32>(0.0))) {
        if (dot(params.clip.xyz, world(head)) + params.clip.w > 0.0) {
            ends[0] = head;
        }
    }
    for (var e = 0u; e < 2u; e = e + 1u) {
        let base = (2u * i + e) * 7u;
        let p = world(ends[e]);
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
    let m = vec4<f32>(start_velocity(s.xyz), -1.0);
    motion[i] = m;
    write(i, s, m, sample(s.xyz).xyz);
}

@compute @workgroup_size(64)
fn advect(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.count) { return; }
    var s = state[i];
    var m = motion[i];
    let dt = params.dt;
    // Stuck: stay, then start again.
    if (m.w >= 0.0) {
        m.w = m.w + dt;
        if (m.w > heavy.stick_time) {
            s = respawn(i, params.frame);
            m = vec4<f32>(start_velocity(s.xyz), -1.0);
        }
        state[i] = s;
        motion[i] = m;
        write(i, s, m, vec3<f32>(0.0));
        return;
    }
    let x = s.xyz;
    let v = m.xyz;
    let u0 = sample(x);
    let age = s.w + dt;
    if (u0.w < 0.0 || age > params.max_age) {
        s = respawn(i, params.frame);
        m = vec4<f32>(start_velocity(s.xyz), -1.0);
        state[i] = s;
        motion[i] = m;
        write(i, s, m, sample(s.xyz).xyz);
        return;
    }
    let first = exact(x, v, u0.xyz, tau_e(length(u0.xyz - v)), dt);
    let um4 = sample(0.5 * (x + first.x));
    let um = select(u0.xyz, um4.xyz, um4.w > 0.0);
    var next = exact(x, v, um, tau_e(length(um - v)), dt);
    // Impact: the distance changes sign between the two points.
    let d0 = distance_at(x);
    let d1 = distance_at(next.x);
    var stuck = false;
    if (d0 > 0.0 && d1 <= 0.0) {
        let f = d0 / max(d0 - d1, 1e-30);
        let hit = x + (next.x - x) * f;
        let vi = mix(v, next.v, f);
        let n = normal_at(hit, 0.25 * heavy.mark);
        let bounces = u32(-m.w - 1.0);
        if (heavy.stick == 1u || bounces >= 5u) {
            next = Moved(hit, vec3<f32>(0.0));
            stuck = true;
        } else {
            let vn = n * dot(vi, n);
            let after = -vn * heavy.restitution + (vi - vn) * (1.0 - heavy.friction);
            next = Moved(hit + n * (0.05 * heavy.mark), after);
            m.w = m.w - 1.0;
        }
    }
    s = vec4<f32>(next.x, age);
    if (stuck) {
        m = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    } else {
        m = vec4<f32>(next.v, m.w);
    }
    state[i] = s;
    motion[i] = m;
    write(i, s, m, um);
}
