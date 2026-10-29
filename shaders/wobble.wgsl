struct U {
    time: f32,
    strength: f32,
    size: f32,
    speed: f32,
    seed: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const TWO_PI: f32 = 6.2831855f;

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
    var pos: f32;

    let _e18 = vUv_1;
    p = (vec2(-1f) + (2f * _e18));
    let _e23 = global.time;
    let _e25 = global.speed;
    let _e27 = global.seed;
    let _e30 = p;
    let _e31 = global.size;
    pos = ((((_e23 * TWO_PI) * _e25) + (_e27 * TWO_PI)) + length(((_e30 * _e31) * 20f)));
    let _e38 = vUv_1;
    let _e39 = global.strength;
    let _e42 = pos;
    let _e44 = pos;
    let _e49 = textureSample(tDiffuse_t, tDiffuse_s, (_e38 + ((_e39 * 0.05f) * vec2<f32>(cos(_e42), sin(_e44)))));
    o_color = _e49;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e23 = o_color;
    return FragmentOutput(_e23);
}
