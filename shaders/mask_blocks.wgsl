struct U {
    b_showMask: f32,
    time: f32,
    amount: f32,
    speed: f32,
    scale: f32,
    resolution: vec2<f32>,
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
var tOrig_t: texture_2d<f32>;
@group(0) @binding(2) 
var tOrig_s: sampler;
@group(0) @binding(3) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(4) 
var tDiffuse_s: sampler;

fn rand(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e22 = n_1;
    return fract((sin(_e22) * 43758.547f));
}

fn noise(p: f32) -> f32 {
    var p_1: f32;
    var fl: f32;
    var fc: f32;

    p_1 = p;
    let _e22 = p_1;
    fl = floor(_e22);
    let _e25 = p_1;
    fc = fract(_e25);
    let _e28 = fl;
    let _e29 = rand(_e28);
    let _e30 = fl;
    let _e33 = rand((_e30 + 1f));
    let _e34 = fc;
    return mix(_e29, _e33, _e34);
}

fn main_1() {
    var speed2_: f32;
    var amount2_: f32;
    var scale2_: f32;
    var colUnder: vec4<f32>;
    var colOver: vec4<f32>;
    var stepT: f32;
    var uv: vec2<f32>;
    var cellid: f32;
    var mask: f32;
    var col: vec4<f32> = vec4(1f);

    let _e22 = global.speed;
    speed2_ = mix(0f, 60f, _e22);
    let _e26 = global.amount;
    amount2_ = (1f - _e26);
    let _e31 = global.scale;
    scale2_ = mix(0f, 40f, _e31);
    let _e34 = vUv_1;
    let _e35 = textureSample(tOrig_t, tOrig_s, _e34);
    colUnder = _e35.xyzw;
    let _e38 = vUv_1;
    let _e39 = textureSample(tDiffuse_t, tDiffuse_s, _e38);
    colOver = _e39.xyzw;
    let _e42 = global.time;
    let _e43 = speed2_;
    let _e45 = global.seed;
    stepT = floor(((_e42 * _e43) + (_e45 * 1794.48f)));
    let _e51 = vUv_1;
    uv = _e51;
    let _e53 = uv;
    uv = (_e53 - vec2(0.5f));
    let _e58 = uv;
    let _e60 = global.resolution;
    let _e62 = global.resolution;
    uv.x = (_e58.x * (_e60.x / _e62.y));
    let _e66 = uv;
    let _e67 = scale2_;
    let _e70 = scale2_;
    uv = (floor((_e66 * _e67)) / vec2(_e70));
    let _e73 = uv;
    let _e75 = uv;
    let _e77 = scale2_;
    cellid = (_e73.x + (_e75.y * _e77));
    let _e81 = uv;
    uv = (_e81 + vec2(0.5f));
    let _e85 = cellid;
    let _e86 = stepT;
    let _e87 = rand(_e86);
    let _e91 = noise((_e85 + (_e87 * 100f)));
    mask = _e91;
    let _e93 = amount2_;
    let _e94 = mask;
    mask = step(_e93, _e94);
    let _e99 = global.b_showMask;
    if (_e99 > 0.5f) {
        {
            let _e102 = mask;
            col = vec4<f32>(_e102, 0f, 0f, 1f);
        }
    } else {
        {
            let _e107 = colUnder;
            let _e108 = colOver;
            let _e109 = mask;
            col = mix(_e107, _e108, vec4(_e109));
        }
    }
    let _e112 = col;
    o_color = _e112;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e29 = o_color;
    return FragmentOutput(_e29);
}
