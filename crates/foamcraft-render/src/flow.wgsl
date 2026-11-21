// SPDX-License-Identifier: GPL-3.0-or-later
// Flow on the surface (Task_33): lines with lights running along them, and wool tufts that
// flutter. Both depend only on fixed vertex data and the time in the globals, so a frame repeats
// exactly for the same time.

struct Globals {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    hover: u32,
    time: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
    // Tufts: the sideways direction of the flutter.
    @location(2) lateral: vec3<f32>,
    // Lines: running time, phase, period (0 = no light), unused.
    // Tufts: distance from the foot, amplitude (radians), frequency (Hz), first phase.
    @location(3) p: vec4<f32>,
    // Tufts: second phase.
    @location(4) q: f32,
};

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) p: vec4<f32>,
    @location(2) normal: vec3<f32>,
};

@vertex
fn vs_lines(v: In) -> Out {
    var out: Out;
    out.clip = globals.view_proj * vec4<f32>(v.pos, 1.0);
    out.color = v.color;
    out.p = v.p;
    out.normal = vec3<f32>(0.0);
    return out;
}

@fragment
fn fs_lines(o: Out) -> @location(0) vec4<f32> {
    if (o.p.z <= 0.0) {
        return vec4<f32>(o.color, 1.0);
    }
    // A bright head moving along the line with a dark tail behind it.
    let u = fract((globals.time - o.p.x) / o.p.z + o.p.y);
    let head = pow(1.0 - u, 8.0);
    let c = o.color * (0.35 + 0.65 * head) + vec3<f32>(0.5, 0.5, 0.5) * head * head;
    return vec4<f32>(min(c, vec3<f32>(1.0)), 1.0);
}

const TAU: f32 = 6.2831853;

@vertex
fn vs_tufts(v: In) -> Out {
    var out: Out;
    let w = TAU * v.p.z * globals.time;
    let angle = v.p.y * (0.6 * sin(w + v.p.w) + 0.4 * sin(1.7 * w + v.q));
    // The flutter grows from the foot to the tip.
    let pos = v.pos + v.lateral * (v.p.x * sin(angle));
    out.clip = globals.view_proj * vec4<f32>(pos, 1.0);
    out.color = v.color;
    out.p = v.p;
    out.normal = vec3<f32>(0.0);
    return out;
}

@fragment
fn fs_tufts(o: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(o.color, 1.0);
}

// Ribbons and tubes (Task_35): lateral = face normal; p = running time (screen seconds), unused,
// time between two marks (0 = none), unused.
@vertex
fn vs_ribbons(v: In) -> Out {
    var out: Out;
    out.clip = globals.view_proj * vec4<f32>(v.pos, 1.0);
    out.color = v.color;
    out.p = v.p;
    out.normal = v.lateral;
    return out;
}

@fragment
fn fs_ribbons(o: Out) -> @location(0) vec4<f32> {
    // Head light and a key light from above, from both sides of a ribbon.
    let n = normalize(o.normal + vec3<f32>(0.0, 0.0, 1e-6));
    let key = abs(dot(n, normalize(vec3<f32>(0.3, 0.2, 1.0))));
    let shade = 0.35 + 0.65 * key;
    var c = o.color * shade;
    // Time marks running downstream: darker stripes every mark period.
    if (o.p.z > 0.0) {
        let u = fract((o.p.x - globals.time) / o.p.z);
        c = c * select(1.0, 0.62, u > 0.5);
    }
    return vec4<f32>(c, 1.0);
}
