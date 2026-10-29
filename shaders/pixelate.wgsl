struct U {
    pixelsX: f32,
    pixelsY: f32,
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
    var orig: vec4<f32>;
    var color: vec4<f32>;

    let _e10 = vUv_1;
    p = _e10;
    let _e12 = p;
    let _e13 = textureSample(tDiffuse_t, tDiffuse_s, _e12);
    orig = _e13;
    let _e16 = p;
    let _e18 = global.pixelsX;
    let _e21 = global.pixelsX;
    let _e24 = global.pixelsX;
    p.x = ((floor((_e16.x * _e18)) / _e21) + (0.5f / _e24));
    let _e28 = p;
    let _e30 = global.pixelsY;
    let _e33 = global.pixelsY;
    let _e36 = global.pixelsY;
    p.y = ((floor((_e28.y * _e30)) / _e33) + (0.5f / _e36));
    let _e39 = p;
    let _e40 = textureSample(tDiffuse_t, tDiffuse_s, _e39);
    color = _e40;
    let _e42 = orig;
    let _e43 = color;
    let _e44 = global.amount;
    color = mix(_e42, _e43, vec4(_e44));
    let _e47 = color;
    o_color = _e47;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
