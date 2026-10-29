struct U {
    size: f32,
    speed: f32,
    time: f32,
    angle: f32,
    amount: f32,
    offset: f32,
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

fn rotate2D(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e22 = theta_1;
    let _e24 = theta_1;
    let _e27 = theta_1;
    let _e29 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e22), -(sin(_e24))), vec2<f32>(sin(_e27), cos(_e29)));
    let _e35 = m;
    let _e36 = position_1;
    return (_e35 * _e36);
}

fn main_1() {
    var p: vec2<f32>;
    var orig: vec4<f32>;
    var color: vec4<f32>;

    let _e18 = vUv_1;
    p = _e18;
    let _e20 = p;
    let _e21 = textureSample(tDiffuse_t, tDiffuse_s, _e20);
    orig = _e21;
    let _e23 = p;
    p = (_e23 - vec2(0.5f));
    let _e27 = p;
    let _e28 = global.angle;
    let _e31 = rotate2D(_e27, (_e28 * 6.283f));
    p = _e31;
    let _e33 = p;
    let _e35 = global.time;
    let _e36 = global.speed;
    let _e38 = global.size;
    let _e40 = global.seed;
    p.x = (_e33.x - (((_e35 * _e36) * _e38) + (_e40 * 694.48f)));
    let _e45 = p;
    let _e46 = global.size;
    let _e47 = vec2(_e46);
    p = (_e45 - (floor((_e45 / _e47)) * _e47));
    let _e52 = p;
    let _e53 = global.size;
    p = fract(abs((_e52 - vec2((_e53 / 2f)))));
    let _e60 = p;
    let _e61 = global.offset;
    p = (_e60 + vec2((_e61 / 2f)));
    let _e66 = p;
    let _e67 = textureSample(tDiffuse_t, tDiffuse_s, _e66);
    color = _e67;
    let _e69 = orig;
    let _e70 = color;
    let _e71 = global.amount;
    color = mix(_e69, _e70, vec4(_e71));
    let _e74 = color;
    o_color = _e74;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
