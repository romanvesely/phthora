struct U {
    fade: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tCurrent_t: texture_2d<f32>;
@group(0) @binding(2) 
var tCurrent_s: sampler;
@group(0) @binding(3) 
var tPrev_t: texture_2d<f32>;
@group(0) @binding(4) 
var tPrev_s: sampler;

fn main_1() {
    let _e8 = vUv_1;
    let _e9 = textureSample(tPrev_t, tPrev_s, _e8);
    let _e10 = global.fade;
    let _e12 = vUv_1;
    let _e13 = textureSample(tCurrent_t, tCurrent_s, _e12);
    o_color = ((_e9 * _e10) + _e13);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
