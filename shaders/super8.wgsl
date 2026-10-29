struct U {
    burn: f32,
    dust: f32,
    frame: f32,
    roll: f32,
    time: f32,
    seed: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const PI: f32 = 3.1415927f;

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;
@group(0) @binding(3) 
var tDust_t: texture_2d<f32>;
@group(0) @binding(4) 
var tDust_s: sampler;
@group(0) @binding(5) 
var tBurn_t: texture_2d<f32>;
@group(0) @binding(6) 
var tBurn_s: sampler;

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e22 = n_1;
    return fract((sin(_e22) * 10000f));
}

fn noise1d(p: f32) -> f32 {
    var p_1: f32;
    var i: f32;
    var f: f32;
    var u: f32;

    p_1 = p;
    let _e22 = p_1;
    i = floor(_e22);
    let _e25 = p_1;
    f = fract(_e25);
    let _e28 = f;
    let _e29 = f;
    let _e33 = f;
    u = ((_e28 * _e29) * (3f - (2f * _e33)));
    let _e38 = i;
    let _e39 = random1d(_e38);
    let _e40 = i;
    let _e43 = random1d((_e40 + 1f));
    let _e44 = u;
    return mix(_e39, _e43, _e44);
}

fn roundRect(p_2: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    var p_3: vec2<f32>;
    var b_1: vec2<f32>;
    var r_1: f32;

    p_3 = p_2;
    b_1 = b;
    r_1 = r;
    let _e26 = p_3;
    let _e28 = b_1;
    let _e34 = r_1;
    return (length(max((abs(_e26) - _e28), vec2(0f))) - _e34);
}

fn smoothedge(v: f32) -> f32 {
    var v_1: f32;

    v_1 = v;
    let _e24 = v_1;
    return smoothstep(0f, 0.02f, _e24);
}

fn backInOut(t: f32) -> f32 {
    var t_1: f32;
    var local: f32;
    var f_1: f32;
    var g: f32;
    var local_1: f32;

    t_1 = t;
    let _e23 = t_1;
    if (_e23 < 0.5f) {
        let _e27 = t_1;
        local = (2f * _e27);
    } else {
        let _e31 = t_1;
        local = (1f - ((2f * _e31) - 1f));
    }
    let _e37 = local;
    f_1 = _e37;
    let _e39 = f_1;
    let _e42 = f_1;
    let _e43 = f_1;
    g = (pow(_e39, 3f) - (_e42 * sin((_e43 * PI))));
    let _e49 = t_1;
    if (_e49 < 0.5f) {
        let _e53 = g;
        local_1 = (0.5f * _e53);
    } else {
        let _e57 = g;
        local_1 = ((0.5f * (1f - _e57)) + 0.5f);
    }
    let _e63 = local_1;
    return _e63;
}

fn randomRange(seed: f32, min_: f32, max_: f32) -> f32 {
    var seed_1: f32;
    var min_1: f32;
    var max_1: f32;

    seed_1 = seed;
    min_1 = min_;
    max_1 = max_;
    let _e27 = min_1;
    let _e28 = seed_1;
    let _e29 = random1d(_e28);
    let _e30 = max_1;
    let _e31 = min_1;
    return (_e27 + (_e29 * (_e30 - _e31)));
}

fn main_1() {
    var uv: vec2<f32>;
    var rollT: f32;
    var stepT: f32;
    var rollAmt: f32 = 0f;
    var gotoPos: f32;
    var orig: vec4<f32>;
    var colDust: vec3<f32>;
    var colBurn: vec3<f32>;
    var col: vec3<f32>;
    var frameWidth: f32;
    var d: f32;
    var burnAmt: f32;

    let _e21 = vUv_1;
    uv = _e21;
    let _e23 = global.time;
    rollT = (_e23 * 4f);
    let _e27 = rollT;
    stepT = floor(_e27);
    let _e32 = stepT;
    let _e33 = random1d(_e32);
    let _e34 = global.roll;
    if (_e33 < (_e34 * 0.5f)) {
        {
            let _e38 = stepT;
            let _e44 = randomRange((_e38 + 99.99f), -3f, 3f);
            gotoPos = floor(_e44);
            let _e47 = rollT;
            let _e49 = backInOut(fract(_e47));
            let _e50 = gotoPos;
            rollAmt = (_e49 * _e50);
        }
    }
    let _e53 = uv;
    let _e55 = rollAmt;
    uv.y = fract((_e53.y - _e55));
    let _e58 = uv;
    let _e59 = textureSample(tDiffuse_t, tDiffuse_s, _e58);
    orig = _e59;
    let _e61 = uv;
    let _e62 = textureSample(tDust_t, tDust_s, _e61);
    colDust = _e62.xyz;
    let _e65 = uv;
    let _e66 = textureSample(tBurn_t, tBurn_s, _e65);
    colBurn = _e66.xyz;
    let _e69 = orig;
    col = _e69.xyz;
    let _e72 = col;
    let _e75 = colDust;
    let _e76 = global.dust;
    col = (_e72 * mix(vec3(1f), _e75, vec3(_e76)));
    let _e82 = global.frame;
    frameWidth = mix(0.48f, 0.4f, _e82);
    let _e85 = uv;
    let _e89 = frameWidth;
    let _e92 = roundRect((_e85 - vec2(0.5f)), vec2(_e89), 0.05f);
    d = _e92;
    let _e94 = col;
    let _e96 = d;
    let _e97 = smoothedge(_e96);
    col = (_e94 * (1f - _e97));
    let _e102 = global.time;
    let _e105 = global.seed;
    let _e109 = noise1d(((_e102 * 4f) + (_e105 * 47.381f)));
    burnAmt = smoothstep(0.6f, 1f, _e109);
    let _e112 = col;
    let _e113 = colBurn;
    let _e114 = burnAmt;
    let _e116 = global.burn;
    col = (_e112 + ((_e113 * _e114) * _e116));
    let _e119 = col;
    let _e120 = orig;
    o_color = vec4<f32>(_e119.x, _e119.y, _e119.z, _e120.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e33 = o_color;
    return FragmentOutput(_e33);
}
