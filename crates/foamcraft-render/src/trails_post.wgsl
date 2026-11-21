// SPDX-License-Identifier: GPL-3.0-or-later
// Light trails (Task_35): full-screen passes over the light image: a smaller copy, a blur and the
// composition onto the scene.

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var glow_tex: texture_2d<f32>;
@group(0) @binding(3) var<uniform> post: vec4<f32>;

struct Full {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) id: u32) -> Full {
    var out: Full;
    let x = f32((id << 1u) & 2u);
    let y = f32(id & 2u);
    out.clip = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

// Smaller copy (the sampler averages).
@fragment
fn fs_down(o: Full) -> @location(0) vec4<f32> {
    return textureSample(source, smp, o.uv);
}

// Blur along post.xy (one texel step), nine taps.
@fragment
fn fs_blur(o: Full) -> @location(0) vec4<f32> {
    var weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    var c = textureSample(source, smp, o.uv) * weights[0];
    for (var i = 1; i < 5; i = i + 1) {
        let off = post.xy * f32(i);
        c = c + textureSample(source, smp, o.uv + off) * weights[i];
        c = c + textureSample(source, smp, o.uv - off) * weights[i];
    }
    return c;
}

// Light (plus 60 % glow when post.z > 0) mapped with 1 - exp(-light); blended as a screen.
// Fading of the light image in the view: the blend multiplies by a constant.
@fragment
fn fs_fade(o: Full) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

// The light image has the size of the target: read it at the pixel (also inside a region).
@fragment
fn fs_compose(o: Full) -> @location(0) vec4<f32> {
    let uv = o.clip.xy / vec2<f32>(textureDimensions(source));
    var light = textureSample(source, smp, uv).xyz;
    if (post.z > 0.0) {
        light = light + 0.6 * textureSample(glow_tex, smp, uv).xyz;
    }
    let c = vec3<f32>(1.0) - exp(-post.w * light);
    return vec4<f32>(c, max(c.x, max(c.y, c.z)));
}
