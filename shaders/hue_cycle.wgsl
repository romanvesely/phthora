struct U {
    hue: f32,
    time: f32,
    speed: f32,
    amount: f32,
    seed: f32,
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
    var angle: f32;
    var s: f32;
    var c: f32;
    var weights: vec3<f32>;
    var len: f32;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    orig = _e15;
    let _e17 = orig;
    color = _e17;
    let _e19 = global.hue;
    let _e22 = global.time;
    let _e23 = global.speed;
    let _e28 = global.seed;
    angle = (((_e19 * 6.2831855f) + ((_e22 * _e23) * 6.2831855f)) + (_e28 * 6.2831855f));
    let _e33 = angle;
    s = sin(_e33);
    let _e36 = angle;
    c = cos(_e36);
    let _e40 = c;
    let _e45 = s;
    let _e47 = c;
    let _e51 = s;
    let _e53 = c;
    weights = ((vec3<f32>((2f * _e40), ((-1.7320508f * _e45) - _e47), ((1.7320508f * _e51) - _e53)) + vec3(1f)) / vec3(3f));
    let _e63 = color;
    len = length(_e63.xyz);
    let _e67 = color;
    let _e69 = color;
    let _e71 = weights;
    let _e74 = color;
    let _e76 = weights;
    let _e79 = color;
    let _e81 = weights;
    let _e84 = vec3<f32>(dot(_e69.xyz, _e71.xyz), dot(_e74.xyz, _e76.zxy), dot(_e79.xyz, _e81.yzx));
    color.x = _e84.x;
    color.y = _e84.y;
    color.z = _e84.z;
    let _e91 = orig;
    let _e92 = color;
    let _e93 = global.amount;
    color = mix(_e91, _e92, vec4(_e93));
    let _e96 = color;
    o_color = _e96;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
