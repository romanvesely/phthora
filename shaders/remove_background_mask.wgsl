// Mask post-processing at mask resolution (a few tens of thousands of pixels).
// mode 0: grow (radius > 0) or shrink (radius < 0) the mask by `radius` texels.
// mode 1: soften it with a blur of `radius` texels.
struct U {
    radius: f32,
    mode: f32,
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

const TAU: f32 = 6.2831853;
const RINGS: i32 = 4;
const DIRS: i32 = 16;

fn tap(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(tDiffuse_t, tDiffuse_s, uv, 0.0).x;
}

@fragment
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    let texel = 1.0 / vec2<f32>(textureDimensions(tDiffuse_t));
    let r = abs(global.radius);
    let center = tap(vUv);
    var v = center;
    if (r > 0.01) {
        let grow = global.radius > 0.0;
        var best = center;
        var sum = center;
        var wsum = 1.0;
        for (var ring = 1; ring <= RINGS; ring++) {
            let f = f32(ring) / f32(RINGS);
            let rr = r * f;
            // gaussian falloff for the blur
            let wt = exp(-2.0 * f * f);
            for (var k = 0; k < DIRS; k++) {
                // rotate every ring so the taps do not line up into spokes
                let a = (f32(k) + 0.37 * f32(ring)) * TAU / f32(DIRS);
                let t = tap(vUv + vec2<f32>(cos(a), sin(a)) * rr * texel);
                if (grow) {
                    best = max(best, t);
                } else {
                    best = min(best, t);
                }
                sum += t * wt;
                wsum += wt;
            }
        }
        if (global.mode < 0.5) {
            v = best;
        } else {
            v = sum / wsum;
        }
    }
    return FragmentOutput(vec4<f32>(v, v, v, 1.0));
}
