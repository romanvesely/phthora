struct U {
    count: f32,
    shift: f32,
    offsetV: f32,
    angle: f32,
    scale: f32,
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

fn steppedVal(v: f32, steps: f32) -> f32 {
    var v_1: f32;
    var steps_1: f32;

    v_1 = v;
    steps_1 = steps;
    let _e18 = v_1;
    let _e19 = steps_1;
    let _e22 = steps_1;
    return (floor((_e18 * _e19)) / _e22);
}

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e16 = n_1;
    return fract((sin(_e16) * 43758.547f));
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e18 = theta_1;
    let _e20 = theta_1;
    let _e23 = theta_1;
    let _e25 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e18), -(sin(_e20))), vec2<f32>(sin(_e23), cos(_e25)));
    let _e31 = m;
    let _e32 = position_1;
    return (_e31 * _e32);
}

fn main_1() {
    var uv: vec2<f32>;
    var ang: f32;
    var uvR: vec2<f32>;
    var stepX: f32;

    let _e14 = vUv_1;
    uv = _e14;
    let _e16 = global.angle;
    ang = (_e16 * 3.1415927f);
    let _e20 = uv;
    uvR = _e20;
    let _e22 = uvR;
    uvR = (_e22 - vec2(0.5f));
    let _e26 = uvR;
    let _e27 = ang;
    let _e28 = rotate2d(_e26, _e27);
    uvR = _e28;
    let _e29 = uvR;
    uvR = (_e29 + vec2(0.5f));
    let _e33 = uvR;
    let _e35 = global.count;
    let _e36 = steppedVal(_e33.x, _e35);
    stepX = _e36;
    let _e38 = stepX;
    let _e41 = global.count;
    stepX = (_e38 - (0.5f - (0.5f / _e41)));
    let _e46 = uvR;
    let _e48 = stepX;
    let _e49 = global.shift;
    uvR.x = (_e46.x + (_e48 * -(_e49)));
    let _e54 = uvR;
    let _e56 = stepX;
    let _e57 = global.offsetV;
    uvR.y = (_e54.y + ((_e56 * (_e57 - 0.5f)) * 4f));
    let _e64 = uvR;
    uvR = (_e64 - vec2(0.5f));
    let _e68 = uvR;
    let _e70 = stepX;
    let _e72 = global.scale;
    uvR = (_e68 * (1f - (abs(_e70) * _e72)));
    let _e76 = uvR;
    uvR = (_e76 + vec2(0.5f));
    let _e80 = uvR;
    uvR = (_e80 - vec2(0.5f));
    let _e84 = uvR;
    let _e85 = ang;
    let _e87 = rotate2d(_e84, -(_e85));
    uvR = _e87;
    let _e88 = uvR;
    uvR = (_e88 + vec2(0.5f));
    let _e92 = uvR;
    uvR = fract(_e92);
    let _e94 = uvR;
    let _e95 = textureSample(tDiffuse_t, tDiffuse_s, _e94);
    o_color = _e95;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
