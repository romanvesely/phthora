struct U {
    colLight: vec3<f32>,
    colDark: vec3<f32>,
    amount: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;

fn luma(color: vec3<f32>) -> f32 {
    var color_1: vec3<f32>;

    color_1 = color;
    let _e12 = color_1;
    return dot(_e12, vec3<f32>(0.299f, 0.587f, 0.114f));
}

fn main_1() {
    var orig: vec4<f32>;
    var duo: vec3<f32>;

    let _e10 = vUv_1;
    let _e11 = textureSample(tDiffuse_t, tDiffuse_s, _e10);
    orig = _e11;
    let _e13 = global.colDark;
    let _e14 = global.colLight;
    let _e15 = orig;
    let _e17 = luma(_e15.xyz);
    duo = mix(_e13, _e14, vec3(_e17));
    let _e21 = orig;
    let _e23 = duo;
    let _e24 = global.amount;
    let _e26 = mix(_e21.xyz, _e23, vec3(_e24));
    let _e27 = orig;
    o_color = vec4<f32>(_e26.x, _e26.y, _e26.z, _e27.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
