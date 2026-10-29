struct U {
    mvp: mat4x4<f32>,
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> global: U;
@group(0) @binding(1) var map_t: texture_2d<f32>;
@group(0) @binding(2) var map_s: sampler;

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> VsOut {
    var o: VsOut;
    o.pos = global.mvp * vec4<f32>(position, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(map_t, map_s, uv);
}
