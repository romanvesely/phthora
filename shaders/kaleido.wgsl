struct U {
    sides: f32,
    angle: f32,
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
    var p: vec2<f32>;
    var r: f32;
    var a: f32;
    var tau: f32 = 6.2832f;
    var color: vec4<f32>;

    let _e10 = vUv_1;
    let _e11 = textureSample(tDiffuse_t, tDiffuse_s, _e10);
    orig = _e11;
    let _e13 = vUv_1;
    p = (_e13 - vec2(0.5f));
    let _e18 = p;
    r = length(_e18);
    let _e21 = p;
    let _e23 = p;
    let _e26 = global.angle;
    a = (atan2(_e21.y, _e23.x) + (_e26 * 6.283f));
    let _e35 = a;
    let _e36 = tau;
    let _e37 = global.sides;
    let _e38 = (_e36 / _e37);
    a = (_e35 - (floor((_e35 / _e38)) * _e38));
    let _e43 = a;
    let _e44 = tau;
    let _e45 = global.sides;
    a = abs((_e43 - ((_e44 / _e45) / 2f)));
    let _e51 = r;
    let _e52 = a;
    let _e54 = a;
    p = (_e51 * vec2<f32>(cos(_e52), sin(_e54)));
    let _e58 = p;
    p = fract((_e58 + vec2(0.5f)));
    let _e63 = p;
    let _e64 = textureSample(tDiffuse_t, tDiffuse_s, _e63);
    color = _e64;
    let _e66 = orig;
    let _e67 = color;
    let _e68 = global.amount;
    color = mix(_e66, _e67, vec4(_e68));
    let _e71 = color;
    o_color = _e71;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
