struct U {
    threshold: f32,
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

fn main_1() {
    var threshold2_: f32;
    var c: vec4<f32>;

    let _e8 = global.threshold;
    threshold2_ = mix(0.8f, 0f, _e8);
    let _e11 = vUv_1;
    let _e12 = textureSample(tDiffuse_t, tDiffuse_s, _e11);
    c = _e12;
    let _e14 = c;
    let _e16 = c;
    let _e18 = threshold2_;
    let _e20 = (_e16.xyz - vec3(_e18));
    c.x = _e20.x;
    c.y = _e20.y;
    c.z = _e20.z;
    let _e27 = c;
    o_color = vec4<f32>(_e27);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e13 = o_color;
    return FragmentOutput(_e13);
}
