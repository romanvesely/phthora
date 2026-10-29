struct U {
    center: f32,
    amount: f32,
    angle: f32,
    wave: f32,
    jaggies: f32,
    time: f32,
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

fn rotate2d(_angle: f32) -> mat2x2<f32> {
    var _angle_1: f32;

    _angle_1 = _angle;
    let _e20 = _angle_1;
    let _e22 = _angle_1;
    let _e25 = _angle_1;
    let _e27 = _angle_1;
    return mat2x2<f32>(vec2<f32>(cos(_e20), -(sin(_e22))), vec2<f32>(sin(_e25), cos(_e27)));
}

fn range(vmin: f32, vmax: f32, value: f32) -> f32 {
    var vmin_1: f32;
    var vmax_1: f32;
    var value_1: f32;

    vmin_1 = vmin;
    vmax_1 = vmax;
    value_1 = value;
    let _e24 = value_1;
    let _e25 = vmin_1;
    let _e27 = vmax_1;
    let _e28 = vmin_1;
    return ((_e24 - _e25) / (_e27 - _e28));
}

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e20 = n_1;
    return fract((sin(_e20) * 43758.547f));
}

fn cubicPulse(position: f32, amount: f32, x: f32) -> f32 {
    var position_1: f32;
    var amount_1: f32;
    var x_1: f32;

    position_1 = position;
    amount_1 = amount;
    x_1 = x;
    let _e24 = x_1;
    let _e25 = position_1;
    x_1 = abs((_e24 - _e25));
    let _e28 = x_1;
    let _e29 = amount_1;
    if (_e28 > _e29) {
        return 0f;
    }
    let _e32 = x_1;
    let _e33 = amount_1;
    x_1 = (_e32 / _e33);
    let _e36 = x_1;
    let _e37 = x_1;
    let _e41 = x_1;
    return (1f - ((_e36 * _e37) * (3f - (2f * _e41))));
}

fn tri(x_2: f32) -> f32 {
    var x_3: f32;

    x_3 = x_2;
    let _e21 = x_3;
    return abs(((2f * fract(((_e21 / 2f) + 0.5f))) - 1f));
}

fn tri_1(p: vec2<f32>) -> vec2<f32> {
    var p_1: vec2<f32>;

    p_1 = p;
    let _e20 = p_1;
    let _e22 = tri(_e20.x);
    let _e23 = p_1;
    let _e25 = tri(_e23.y);
    return vec2<f32>(_e22, _e25);
}

fn main_1() {
    var uv: vec2<f32>;
    var angle2_: f32;
    var start: f32;
    var end: f32;
    var fps: f32 = 120f;
    var steppedT: f32;
    var yOff: f32;
    var steps: f32 = 100f;
    var xStepped: f32;

    let _e18 = vUv_1;
    uv = _e18;
    let _e20 = global.angle;
    angle2_ = ((_e20 * 3.1415927f) * 2f);
    let _e26 = global.center;
    let _e27 = global.amount;
    start = (_e26 - (_e27 / 2f));
    let _e32 = global.center;
    let _e33 = global.amount;
    end = (_e32 + (_e33 / 2f));
    let _e38 = uv;
    uv = (_e38 - vec2(0.5f));
    let _e42 = uv;
    let _e43 = angle2_;
    let _e44 = rotate2d(_e43);
    uv = (_e42 * _e44);
    let _e46 = uv;
    uv = (_e46 + vec2(0.5f));
    let _e54 = global.time;
    let _e55 = fps;
    steppedT = floor((_e54 * _e55));
    let _e59 = uv;
    let _e61 = start;
    let _e63 = uv;
    let _e65 = end;
    if ((_e59.x > _e61) && (_e63.x < _e65)) {
        {
            let _e68 = uv;
            let _e70 = global.time;
            let _e72 = global.seed;
            let _e81 = global.wave;
            yOff = ((sin(((((_e68.x + _e70) + _e72) * 3.1415927f) * 4f)) * 0.2f) * _e81);
            let _e86 = uv;
            let _e88 = steps;
            let _e90 = global.seed;
            xStepped = floor(((_e86.x * _e88) + (_e90 * 391.48f)));
            let _e96 = yOff;
            let _e97 = xStepped;
            let _e98 = steppedT;
            let _e100 = random1d((_e97 + _e98));
            let _e105 = global.jaggies;
            yOff = (_e96 + (((_e100 - 0.5f) * 0.1f) * _e105));
            let _e108 = yOff;
            let _e109 = global.center;
            let _e110 = global.amount;
            let _e113 = uv;
            let _e115 = cubicPulse(_e109, (_e110 / 2f), _e113.x);
            yOff = (_e108 * _e115);
            let _e118 = uv;
            let _e120 = yOff;
            uv.y = (_e118.y + _e120);
            let _e123 = start;
            uv.x = _e123;
        }
    }
    let _e124 = uv;
    let _e126 = end;
    if (_e124.x > _e126) {
        let _e129 = uv;
        let _e131 = end;
        let _e132 = start;
        uv.x = (_e129.x - (_e131 - _e132));
    }
    let _e135 = uv;
    uv = (_e135 - vec2(0.5f));
    let _e139 = uv;
    let _e140 = angle2_;
    let _e142 = rotate2d(-(_e140));
    uv = (_e139 * _e142);
    let _e144 = uv;
    uv = (_e144 + vec2(0.5f));
    let _e148 = uv;
    let _e149 = tri_1(_e148);
    uv = _e149;
    let _e150 = uv;
    let _e151 = textureSample(tDiffuse_t, tDiffuse_s, _e150);
    o_color = _e151;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
