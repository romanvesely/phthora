// Combines a group's result with the image that went into the group (premultiplied).
// mode 0: result over input (isolated group), 1: input cut by the result's alpha,
// 2: result behind the input.
struct U {
    mode: f32,
}

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tInput_t: texture_2d<f32>;
@group(0) @binding(2) var tInput_s: sampler;
@group(0) @binding(3) var tGroup_t: texture_2d<f32>;
@group(0) @binding(4) var tGroup_s: sampler;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let i = textureSample(tInput_t, tInput_s, uv);
    let g = textureSample(tGroup_t, tGroup_s, uv);
    if (u.mode < 0.5) {
        return g + i * (1.0 - g.a);
    }
    if (u.mode < 1.5) {
        return i * g.a;
    }
    return i + g * (1.0 - i.a);
}
