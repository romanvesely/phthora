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

fn main_1() {
    var p: vec2<f32>;
    var r: f32;
    var a: f32;
    var distort: f32;
    var color: vec4<f32>;

    let _e6 = vUv_1;
    p = (_e6 - vec2(0.5f));
    let _e11 = p;
    r = length(_e11);
    let _e14 = p;
    let _e16 = p;
    a = atan2(_e14.y, _e16.x);
    let _e20 = r;
    let _e21 = r;
    distort = (_e20 * _e21);
    let _e24 = r;
    let _e25 = distort;
    let _e26 = global.amount;
    r = mix(_e24, _e25, _e26);
    let _e28 = r;
    let _e29 = a;
    let _e31 = a;
    p = (_e28 * vec2<f32>(cos(_e29), sin(_e31)));
    let _e35 = p;
    let _e39 = textureSample(tDiffuse_t, tDiffuse_s, (_e35 + vec2(0.5f)));
    color = _e39;
    let _e41 = color;
    o_color = _e41;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e13 = o_color;
    return FragmentOutput(_e13);
}
