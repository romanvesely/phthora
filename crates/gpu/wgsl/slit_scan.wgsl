// Slit Scan: the image is cut into stripes along `angle`, stripe i shows the input
// from a delay proportional to i. Frames come from a ring in a 2D array texture.
struct U {
    amount: f32,
    slices: f32,
    angle: f32,
    b_smooth: f32,
    aspect: f32,
    // ring layer of the newest frame, layer count, frames recorded so far
    head: f32,
    cap: f32,
    count: f32,
}

@group(0) @binding(0)
var<uniform> global: U;
@group(0) @binding(1)
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2)
var samp: sampler;
@group(0) @binding(3)
var ring_t: texture_2d_array<f32>;

// The frame `d` frames ago (0 = the live input at full size); older than recorded repeats the oldest.
fn frame(d: f32, uv: vec2<f32>) -> vec4<f32> {
    let cap = i32(global.cap);
    let k = min(i32(d), min(i32(global.count), cap) - 1);
    if (k <= 0) {
        return textureSampleLevel(tDiffuse_t, samp, uv, 0f);
    }
    let layer = (i32(global.head) - k + cap) % cap;
    return textureSampleLevel(ring_t, samp, uv, layer, 0f);
}

@fragment
fn main(@location(0) vUv: vec2<f32>) -> @location(0) vec4<f32> {
    let n = max(global.slices, 2f);
    let theta = global.angle * 6.2831855f;
    let dir = vec2<f32>(cos(theta), sin(theta));
    // position along the stripe direction, 0..1 across the whole image
    let p = vec2<f32>((vUv.x - 0.5) * global.aspect, vUv.y - 0.5);
    let extent = abs(dir.x) * global.aspect + abs(dir.y);
    let s = clamp(dot(p, dir) / extent + 0.5, 0f, 1f);
    if (global.b_smooth > 0.5) {
        let d = global.amount * (n - 1f) * s;
        let d0 = floor(d);
        return mix(frame(d0, vUv), frame(d0 + 1f, vUv), d - d0);
    }
    let i = min(floor(s * n), n - 1f);
    return frame(floor(global.amount * i + 0.5), vUv);
}
