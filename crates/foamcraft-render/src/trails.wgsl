// SPDX-License-Identifier: GPL-3.0-or-later
// Light trails (Task_35): wide lines in screen space read straight from the ring of each particle,
// drawn additively into a floating-point light image (tested against the scene depth, not
// written); then an optional soft glow and the composition onto the scene:
// c = 1 - exp(-exposure · light), image = 1 - (1 - scene) · (1 - c).

struct View {
    view_proj: mat4x4<f32>,
    // cos and sin of the yaw (ring points are in the flow frame), viewport size in pixels.
    rot: vec4<f32>,
    // K, newest slot, number of written points, segments drawn per particle.
    ring: vec4<u32>,
    // width at the head (px), exposure, speed range for the colour (lo, hi).
    style: vec4<f32>,
    // constant colour, w: 1 = colour by speed.
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> ring: array<vec4<f32>>;

struct TrailOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};

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
    return vec3<f32>(view.rot.x * f.x - view.rot.y * f.y, view.rot.y * f.x + view.rot.x * f.y, f.z);
}

// Point j behind the head of particle p (0 = the newest).
fn point(p: u32, j: u32) -> vec4<f32> {
    let k = view.ring.x;
    let slot = (view.ring.y + k - (j % k)) % k;
    return ring[p * k + slot];
}

// Cut a clip-space segment at the near plane (z >= 0 in the wgpu convention).
fn cut(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    if (a.z >= 0.0) { return a; }
    let t = a.z / (a.z - b.z);
    return mix(a, b, clamp(t, 0.0, 1.0));
}

@vertex
fn vs_trail(@builtin(vertex_index) id: u32) -> TrailOut {
    var out: TrailOut;
    let segs = view.ring.w;
    let p = id / (segs * 6u);
    let j = (id / 6u) % segs;
    let corner = id % 6u;
    let head = point(p, j);
    let tail = point(p, j + 1u);
    out.clip = vec4<f32>(0.0, 0.0, -1.0, 1.0);
    out.color = vec3<f32>(0.0);
    // Not written yet, or a new start between the two points: nothing to draw.
    if (j + 1u >= view.ring.z || head.w < 0.0) {
        return out;
    }
    var a = view.view_proj * vec4<f32>(world(head.xyz), 1.0);
    var b = view.view_proj * vec4<f32>(world(tail.xyz), 1.0);
    if (a.z < 0.0 && b.z < 0.0) {
        return out;
    }
    a = cut(a, b);
    b = cut(b, a);
    let size = view.rot.zw;
    let pa = a.xy / a.w * 0.5 * size;
    let pb = b.xy / b.w * 0.5 * size;
    var d = pb - pa;
    if (length(d) < 1e-6) { d = vec2<f32>(1.0, 0.0); }
    let n = normalize(vec2<f32>(-d.y, d.x));
    let k = f32(max(view.ring.x, 2u));
    // Narrower and darker towards the end of the trail.
    let fa = f32(j) / k;
    let fb = f32(j + 1u) / k;
    let wa = view.style.x * (1.0 - 2.0 / 3.0 * fa) * 0.5;
    let wb = view.style.x * (1.0 - 2.0 / 3.0 * fb) * 0.5;
    var c = view.color.xyz;
    if (view.color.w > 0.5) {
        c = colormap((head.w - view.style.z) / max(view.style.w - view.style.z, 1e-30));
    }
    let bright = pow(1.0 - fa, 1.5);
    // Two triangles: a0 b0 b1, a0 b1 a1.
    var side = array<f32, 6>(-1.0, -1.0, 1.0, -1.0, 1.0, 1.0);
    var end = array<u32, 6>(0u, 1u, 1u, 0u, 1u, 0u);
    let e = end[corner];
    let base = select(a, b, e == 1u);
    let w = select(wa, wb, e == 1u);
    let px = select(pa, pb, e == 1u) + n * w * side[corner];
    out.clip = vec4<f32>(px / (0.5 * size) * base.w, base.z, base.w);
    out.color = c * bright * view.style.y;
    return out;
}

@fragment
fn fs_trail(o: TrailOut) -> @location(0) vec4<f32> {
    return vec4<f32>(o.color, 1.0);
}
