struct U {
    slices: f32,
    amount: f32,
    time: f32,
    speedV: f32,
    angle: f32,
    speedH: f32,
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

fn steppedVal(v: f32, steps: f32) -> f32 {
    var v_1: f32;
    var steps_1: f32;

    v_1 = v;
    steps_1 = steps;
    let _e22 = v_1;
    let _e23 = steps_1;
    let _e26 = steps_1;
    return (floor((_e22 * _e23)) / _e26);
}

fn hash(x: vec2<f32>) -> vec2<f32> {
    var x_1: vec2<f32>;
    var k: vec2<f32> = vec2<f32>(0.3183099f, 0.3678794f);

    x_1 = x;
    let _e24 = x_1;
    let _e25 = k;
    let _e27 = k;
    x_1 = ((_e24 * _e25) + _e27.yx);
    let _e34 = k;
    let _e36 = x_1;
    let _e38 = x_1;
    let _e41 = x_1;
    let _e43 = x_1;
    return (vec2(-1f) + (2f * fract(((16f * _e34) * fract(((_e36.x * _e38.y) * (_e41.x + _e43.y)))))));
}

fn noise2dgrad(p: vec2<f32>) -> f32 {
    var p_1: vec2<f32>;
    var i: vec2<f32>;
    var f: vec2<f32>;
    var u: vec2<f32>;
    var n: f32;

    p_1 = p;
    let _e20 = p_1;
    i = floor(_e20);
    let _e23 = p_1;
    f = fract(_e23);
    let _e26 = f;
    let _e27 = f;
    let _e31 = f;
    u = ((_e26 * _e27) * (vec2(3f) - (2f * _e31)));
    let _e37 = i;
    let _e42 = hash((_e37 + vec2<f32>(0f, 0f)));
    let _e43 = f;
    let _e49 = i;
    let _e54 = hash((_e49 + vec2<f32>(1f, 0f)));
    let _e55 = f;
    let _e61 = u;
    let _e64 = i;
    let _e69 = hash((_e64 + vec2<f32>(0f, 1f)));
    let _e70 = f;
    let _e76 = i;
    let _e81 = hash((_e76 + vec2<f32>(1f, 1f)));
    let _e82 = f;
    let _e88 = u;
    let _e91 = u;
    n = mix(mix(dot(_e42, (_e43 - vec2<f32>(0f, 0f))), dot(_e54, (_e55 - vec2<f32>(1f, 0f))), _e61.x), mix(dot(_e69, (_e70 - vec2<f32>(0f, 1f))), dot(_e81, (_e82 - vec2<f32>(1f, 1f))), _e88.x), _e91.y);
    let _e95 = n;
    return ((_e95 * 0.5f) + 0.5f);
}

fn noiseLoop1d(n_1: f32, scale: f32, seed: f32) -> f32 {
    var n_2: f32;
    var scale_1: f32;
    var seed_1: f32;
    var x_2: f32;
    var y: f32;

    n_2 = n_1;
    scale_1 = scale;
    seed_1 = seed;
    let _e24 = n_2;
    let _e28 = seed_1;
    x_2 = (cos((_e24 * 6.2831855f)) + _e28);
    let _e31 = n_2;
    let _e35 = seed_1;
    y = (sin((_e31 * 6.2831855f)) + _e35);
    let _e38 = x_2;
    let _e39 = y;
    let _e41 = scale_1;
    let _e43 = noise2dgrad((vec2<f32>(_e38, _e39) * _e41));
    return _e43;
}

fn random1d(n_3: f32) -> f32 {
    var n_4: f32;

    n_4 = n_3;
    let _e20 = n_4;
    return fract((sin(_e20) * 43758.547f));
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
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
    var uv: vec2<f32>;
    var ang: f32;
    var uvR: vec2<f32>;
    var n_5: f32;
    var ns: f32;
    var nsr: f32;
    var off: f32;
    var uvn: vec2<f32>;

    let _e18 = vUv_1;
    uv = _e18;
    let _e20 = global.angle;
    ang = (_e20 * 3.1415927f);
    let _e24 = uv;
    uvR = _e24;
    let _e26 = uv;
    uvR = (_e26 - vec2(0.5f));
    let _e30 = uvR;
    let _e31 = ang;
    let _e32 = rotate2d(_e30, _e31);
    uvR = _e32;
    let _e33 = uvR;
    uvR = (_e33 + vec2(0.5f));
    let _e37 = uvR;
    let _e41 = global.time;
    let _e42 = global.speedV;
    let _e45 = global.seed;
    let _e50 = global.slices;
    let _e51 = random1d(_e50);
    let _e52 = noiseLoop1d((((_e37.y * 4f) + (_e41 * _e42)) + (_e45 * 98.76f)), 1f, _e51);
    n_5 = _e52;
    let _e54 = n_5;
    let _e56 = global.slices;
    let _e57 = steppedVal(fract(_e54), _e56);
    ns = (_e57 + 2f);
    let _e61 = ns;
    let _e62 = random1d(_e61);
    nsr = _e62;
    let _e64 = nsr;
    let _e65 = global.time;
    let _e70 = global.speedH;
    let _e72 = nsr;
    let _e76 = global.seed;
    let _e82 = global.amount;
    off = (((_e64 * sin((((((_e65 * 3.1415927f) * 2f) * _e70) + (_e72 * 40f)) + (_e76 * 98.76f)))) * _e82) * 0.2f);
    let _e87 = uv;
    let _e88 = off;
    let _e92 = ang;
    let _e94 = rotate2d(vec2<f32>(_e88, 0f), (3.1415927f - _e92));
    uvn = (_e87 + _e94);
    let _e97 = uvn;
    uvn = fract(_e97);
    let _e99 = uvn;
    let _e100 = textureSample(tDiffuse_t, tDiffuse_s, _e99);
    o_color = _e100;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
