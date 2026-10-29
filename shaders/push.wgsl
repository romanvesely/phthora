struct U {
    b_showMask: f32,
    b_rgb: f32,
    b_animate: f32,
    amount: f32,
    exportProgress: f32,
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
@group(0) @binding(3) 
var tMask_t: texture_2d<f32>;
@group(0) @binding(4) 
var tMask_s: sampler;

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

fn main_1() {
    var displace: vec2<f32>;
    var d2_: vec2<f32>;
    var col: vec4<f32>;
    var r: f32;
    var g: f32;
    var b: f32;
    var a: f32;

    let _e18 = vUv_1;
    let _e19 = textureSample(tMask_t, tMask_s, _e18);
    displace = _e19.xy;
    let _e22 = displace;
    let _e26 = global.amount;
    d2_ = (((_e22 - vec2(0.5f)) * _e26) * 0.5f);
    let _e32 = d2_;
    d2_.y = -(_e32.y);
    let _e35 = global.b_animate;
    if (_e35 > 0.5f) {
        {
            let _e39 = d2_;
            let _e42 = vUv_1;
            let _e46 = global.exportProgress;
            let _e50 = global.seed;
            let _e52 = noiseLoop1d(((_e42.y * 0.5f) + _e46), 0.5f, (1f + _e50));
            d2_.x = (_e39.x * (1f + ((_e52 - 0.5f) * 4f)));
            let _e60 = d2_;
            let _e63 = vUv_1;
            let _e67 = global.exportProgress;
            let _e71 = global.seed;
            let _e73 = noiseLoop1d(((_e63.x * 0.5f) + _e67), 0.5f, (99f + _e71));
            d2_.y = (_e60.y * (1f + ((_e73 - 0.5f) * 4f)));
        }
    }
    let _e80 = vUv_1;
    let _e81 = d2_;
    let _e83 = textureSample(tDiffuse_t, tDiffuse_s, (_e80 + _e81));
    col = _e83;
    let _e85 = global.b_rgb;
    if (_e85 > 0.5f) {
        {
            let _e88 = vUv_1;
            let _e89 = d2_;
            let _e91 = textureSample(tDiffuse_t, tDiffuse_s, (_e88 + _e89));
            r = _e91.x;
            let _e94 = vUv_1;
            let _e95 = d2_;
            let _e97 = textureSample(tDiffuse_t, tDiffuse_s, (_e94 - _e95));
            g = _e97.y;
            let _e100 = vUv_1;
            let _e101 = textureSample(tDiffuse_t, tDiffuse_s, _e100);
            b = _e101.z;
            let _e104 = col;
            a = _e104.w;
            let _e107 = r;
            let _e108 = g;
            let _e109 = b;
            let _e110 = a;
            col = vec4<f32>(_e107, _e108, _e109, _e110);
        }
    }
    let _e112 = global.b_showMask;
    if (_e112 > 0.5f) {
        {
            let _e115 = displace;
            col = vec4<f32>(_e115.x, _e115.y, 0f, 1f);
        }
    }
    let _e121 = col;
    o_color = _e121;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e27 = o_color;
    return FragmentOutput(_e27);
}
