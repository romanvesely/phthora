// Converts the bottom-up, premultiplied working image to top-down straight alpha for
// export, with the same integer rounding as unpremultiplying on the CPU.
@group(0) @binding(0) var src_t: texture_2d<f32>;

@fragment
fn main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(src_t);
    let p = vec2<u32>(u32(pos.x), dims.y - 1u - u32(pos.y));
    let c = textureLoad(src_t, p, 0);
    let a = u32(round(c.a * 255.0));
    var rgb = vec3<u32>(round(c.rgb * 255.0));
    if (a != 0u && a != 255u) {
        rgb = min((rgb * 255u + vec3<u32>(a / 2u)) / a, vec3<u32>(255u));
    }
    return vec4<f32>(vec3<f32>(rgb) / 255.0, f32(a) / 255.0);
}
