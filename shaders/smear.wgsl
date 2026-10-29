struct U {
    time: f32,
    amount: f32,
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

fn rotate2D(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e17 = theta_1;
    let _e19 = theta_1;
    let _e22 = theta_1;
    let _e24 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e17), -(sin(_e19))), vec2<f32>(sin(_e22), cos(_e24)));
    let _e30 = m;
    let _e31 = position_1;
    return (_e30 * _e31);
}

fn main_1() {
    var p: vec2<f32>;
    var sPos: vec2<f32>;
    var off: vec2<f32>;
    var ang: f32;
    var col: vec4<f32>;

    let _e13 = vUv_1;
    p = _e13;
    let _e15 = vUv_1;
    sPos = _e15;
    let _e17 = sPos;
    let _e18 = textureSample(tDiffuse_t, tDiffuse_s, _e17);
    off = (_e18.xy - vec2(0.5f));
    let _e24 = global.time;
    let _e26 = global.speed;
    let _e28 = global.seed;
    ang = (((_e24 * TWO_PI) * _e26) + (_e28 * TWO_PI));
    let _e32 = off;
    let _e33 = ang;
    let _e34 = rotate2D(_e32, _e33);
    off = _e34;
    let _e35 = p;
    let _e36 = off;
    let _e37 = global.amount;
    p = (_e35 + ((_e36 * _e37) * 0.15f));
    let _e42 = p;
    let _e43 = textureSample(tDiffuse_t, tDiffuse_s, _e42);
    col = _e43;
    let _e45 = col;
    o_color = _e45;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
