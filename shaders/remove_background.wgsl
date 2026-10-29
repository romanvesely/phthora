// Keeps the person where the mask is white. tMask is the processed person mask
// (bottom-up like every texture); `amount` fades the removal in.
struct U {
    amount: f32,
    b_invert: f32,
    b_showMask: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> global: U;
@group(0) @binding(1)
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2)
var tDiffuse_s: sampler;
@group(0) @binding(3)
var tMask_t: texture_2d<f32>;
@group(0) @binding(4)
var tMask_s: sampler;

@fragment
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    let col = textureSample(tDiffuse_t, tDiffuse_s, vUv);
    var m = textureSample(tMask_t, tMask_s, vUv).x;
    if (global.b_invert > 0.5) {
        m = 1.0 - m;
    }
    if (global.b_showMask > 0.5) {
        return FragmentOutput(vec4<f32>(m, m, m, 1.0));
    }
    // col is premultiplied: scaling all four channels scales the coverage
    return FragmentOutput(mix(col, col * m, global.amount));
}
