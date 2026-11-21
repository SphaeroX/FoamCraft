// SPDX-License-Identifier: GPL-3.0-or-later
// Flat colour with a head light and a little ambient term; hovered and selected faces are tinted.
// fs_pick writes the pick id instead of a colour.

struct Globals {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    hover: u32,
};

struct Object {
    color: vec4<f32>,
    // Placement of the mesh in the world (identity unless a model is being dragged).
    model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> selected: array<u32>;
@group(1) @binding(0) var<uniform> object: Object;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) pick: u32,
    @location(3) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) pick: u32,
    @location(3) color: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    let world = object.model * vec4<f32>(position, 1.0);
    out.clip = globals.view_proj * world;
    out.world = world.xyz;
    out.normal = (object.model * vec4<f32>(normal, 0.0)).xyz;
    out.pick = pick;
    out.color = color;
    return out;
}

fn is_selected(id: u32) -> bool {
    let word = id / 32u;
    if (word >= arrayLength(&selected)) {
        return false;
    }
    return (selected[word] & (1u << (id % 32u))) != 0u;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Translucent objects: screen-door, every other pixel (no blending or sorting needed).
    if (object.color.a < 0.99) {
        let px = vec2<u32>(in.clip.xy);
        if (((px.x + px.y) & 1u) == 0u) {
            discard;
        }
    }
    let n = normalize(in.normal);
    let to_eye = normalize(globals.eye.xyz - in.world);
    // Key light from above, fill from the viewer.
    let key = max(dot(n, normalize(vec3<f32>(0.3, 0.2, 1.0))), 0.0);
    let fill = abs(dot(n, to_eye));
    let shade = 0.18 + 0.5 * key + 0.4 * fill;
    var color = in.color;
    if (in.pick != 0u && is_selected(in.pick)) {
        color = mix(color, vec3<f32>(0.95, 0.55, 0.15), 0.65);
    }
    if (in.pick != 0u && in.pick == globals.hover) {
        color = mix(color, vec3<f32>(1.0, 1.0, 1.0), 0.35);
    }
    return vec4<f32>(color * shade, 1.0);
}

@fragment
fn fs_pick(in: VsOut) -> @location(0) u32 {
    return in.pick;
}
