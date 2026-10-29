struct U {
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

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e14 = n_1;
    return fract((sin(_e14) * 43758.547f));
}

fn main_1() {
    var p: vec2<f32>;
    var sTime: f32;
    var offset: vec2<f32>;

    let _e12 = vUv_1;
    p = _e12;
    let _e14 = global.time;
    let _e15 = global.speed;
    let _e21 = global.seed;
    sTime = floor(((((_e14 * _e15) * 60f) * 4f) + (_e21 * 694.48f)));
    let _e27 = sTime;
    let _e28 = random1d(_e27);
    let _e29 = sTime;
    let _e32 = random1d((_e29 + 999.99f));
    let _e37 = global.amount;
    offset = (((vec2<f32>(_e28, _e32) - vec2(0.5f)) * _e37) * 0.1f);
    let _e42 = p;
    let _e43 = offset;
    p = (_e42 + _e43);
    let _e45 = p;
    let _e46 = textureSample(tDiffuse_t, tDiffuse_s, _e45);
    o_color = _e46;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
