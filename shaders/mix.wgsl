struct U {
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
@group(0) @binding(3) 
var tBlend_t: texture_2d<f32>;
@group(0) @binding(4) 
var tBlend_s: sampler;

fn main_1() {
    var colA: vec4<f32>;
    var colB: vec4<f32>;

    let _e8 = vUv_1;
    let _e9 = textureSample(tDiffuse_t, tDiffuse_s, _e8);
    colA = _e9;
    let _e11 = vUv_1;
    let _e12 = textureSample(tBlend_t, tBlend_s, _e11);
    colB = _e12;
    let _e14 = colA;
    let _e15 = colB;
    let _e16 = global.amount;
    o_color = mix(_e14, _e15, vec4(_e16));
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
