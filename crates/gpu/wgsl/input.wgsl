// Draws the source image (stored top-down, straight alpha) into the bottom-up,
// premultiplied working space. flip_x mirrors horizontally (webcam).
struct U {
    flip_x: f32,
    force_opaque: f32,
}

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var src_t: texture_2d<f32>;
@group(0) @binding(2) var src_s: sampler;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    var x = uv.x;
    if (u.flip_x > 0.5) {
        x = 1.0 - x;
    }
    var c = textureSample(src_t, src_s, vec2<f32>(x, 1.0 - uv.y));
    if (u.force_opaque > 0.5) {
        c.a = 1.0;
    }
    return vec4<f32>(c.rgb * c.a, c.a);
}
