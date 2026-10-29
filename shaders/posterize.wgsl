struct U {
    levels: f32,
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

fn main_1() {
    var orig: vec4<f32>;
    var color: vec4<f32>;

    let _e8 = vUv_1;
    let _e9 = textureSample(tDiffuse_t, tDiffuse_s, _e8);
    orig = _e9;
    let _e11 = orig;
    color = _e11;
    let _e13 = color;
    let _e15 = color;
    let _e17 = global.levels;
    let _e23 = global.levels;
    let _e25 = (floor(((_e15.xyz * _e17) + vec3(0.5f))) / vec3(_e23));
    color.x = _e25.x;
    color.y = _e25.y;
    color.z = _e25.z;
    let _e32 = orig;
    let _e33 = color;
    let _e34 = global.amount;
    color = mix(_e32, _e33, vec4(_e34));
    let _e37 = color;
    o_color = _e37;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e15 = o_color;
    return FragmentOutput(_e15);
}
