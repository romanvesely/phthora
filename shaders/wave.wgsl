struct U {
    amount: f32,
    size: f32,
    position: f32,
    angle: f32,
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

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e19 = theta_1;
    let _e21 = theta_1;
    let _e24 = theta_1;
    let _e26 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e19), -(sin(_e21))), vec2<f32>(sin(_e24), cos(_e26)));
    let _e32 = m;
    let _e33 = position_1;
    return (_e32 * _e33);
}

fn main_1() {
    var angle2_: f32;
    var uv: vec2<f32>;
    var wave: f32;

    let _e15 = global.angle;
    angle2_ = (_e15 * TWO_PI);
    let _e18 = vUv_1;
    uv = _e18;
    let _e20 = uv;
    uv = (_e20 - vec2(0.5f));
    let _e24 = uv;
    let _e25 = angle2_;
    let _e26 = rotate2d(_e24, _e25);
    uv = _e26;
    let _e27 = uv;
    uv = (_e27 + vec2(0.5f));
    let _e31 = uv;
    let _e33 = global.size;
    let _e35 = global.position;
    let _e41 = global.amount;
    wave = ((sin((((_e31.x * _e33) + _e35) * TWO_PI)) * 0.5f) * _e41);
    let _e45 = uv;
    let _e47 = wave;
    uv.x = (_e45.x + _e47);
    let _e52 = uv;
    uv.x = (1f - abs(((2f * fract((_e52.x / 2f))) - 1f)));
    let _e62 = uv;
    uv = (_e62 - vec2(0.5f));
    let _e66 = uv;
    let _e67 = angle2_;
    let _e69 = rotate2d(_e66, -(_e67));
    uv = _e69;
    let _e70 = uv;
    uv = (_e70 + vec2(0.5f));
    let _e74 = uv;
    let _e75 = textureSample(tDiffuse_t, tDiffuse_s, _e74);
    o_color = _e75;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e23 = o_color;
    return FragmentOutput(_e23);
}
