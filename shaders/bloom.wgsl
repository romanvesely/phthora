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
var base_t: texture_2d<f32>;
@group(0) @binding(2) 
var base_s: sampler;
@group(0) @binding(3) 
var level0_t: texture_2d<f32>;
@group(0) @binding(4) 
var level0_s: sampler;
@group(0) @binding(5) 
var level1_t: texture_2d<f32>;
@group(0) @binding(6) 
var level1_s: sampler;
@group(0) @binding(7) 
var level2_t: texture_2d<f32>;
@group(0) @binding(8) 
var level2_s: sampler;
@group(0) @binding(9) 
var level3_t: texture_2d<f32>;
@group(0) @binding(10) 
var level3_s: sampler;
@group(0) @binding(11) 
var level4_t: texture_2d<f32>;
@group(0) @binding(12) 
var level4_s: sampler;

fn main_1() {
    var origCol: vec4<f32>;
    var bloomCol: vec4<f32>;
    var color: vec4<f32>;

    let _e16 = vUv_1;
    let _e17 = textureSample(base_t, base_s, _e16);
    origCol = _e17;
    let _e19 = vUv_1;
    let _e20 = textureSample(level0_t, level0_s, _e19);
    bloomCol = _e20;
    let _e22 = bloomCol;
    let _e23 = vUv_1;
    let _e24 = textureSample(level1_t, level1_s, _e23);
    bloomCol = (_e22 + _e24);
    let _e26 = bloomCol;
    let _e27 = vUv_1;
    let _e28 = textureSample(level2_t, level2_s, _e27);
    bloomCol = (_e26 + _e28);
    let _e30 = bloomCol;
    let _e31 = vUv_1;
    let _e32 = textureSample(level3_t, level3_s, _e31);
    bloomCol = (_e30 + _e32);
    let _e34 = origCol;
    let _e35 = bloomCol;
    let _e36 = global.amount;
    color = (_e34 + (_e35 * _e36));
    let _e40 = color;
    o_color = _e40;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e33 = o_color;
    return FragmentOutput(_e33);
}
