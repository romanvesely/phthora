struct U {
    amount: f32,
    scale: f32,
    warp: f32,
    rotate: f32,
    hueShift: f32,
    exportProgress: f32,
    seed: f32,
    resolution: vec2<f32>,
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
var tPrev_t: texture_2d<f32>;
@group(0) @binding(4) 
var tPrev_s: sampler;

fn hash(x: vec2<f32>) -> vec2<f32> {
    var x_1: vec2<f32>;
    var k: vec2<f32> = vec2<f32>(0.3183099f, 0.3678794f);

    x_1 = x;
    let _e28 = x_1;
    let _e29 = k;
    let _e31 = k;
    x_1 = ((_e28 * _e29) + _e31.yx);
    let _e38 = k;
    let _e40 = x_1;
    let _e42 = x_1;
    let _e45 = x_1;
    let _e47 = x_1;
    return (vec2(-1f) + (2f * fract(((16f * _e38) * fract(((_e40.x * _e42.y) * (_e45.x + _e47.y)))))));
}

fn noise2dgrad(p: vec2<f32>) -> f32 {
    var p_1: vec2<f32>;
    var i: vec2<f32>;
    var f: vec2<f32>;
    var u: vec2<f32>;
    var n: f32;

    p_1 = p;
    let _e24 = p_1;
    i = floor(_e24);
    let _e27 = p_1;
    f = fract(_e27);
    let _e30 = f;
    let _e31 = f;
    let _e35 = f;
    u = ((_e30 * _e31) * (vec2(3f) - (2f * _e35)));
    let _e41 = i;
    let _e46 = hash((_e41 + vec2<f32>(0f, 0f)));
    let _e47 = f;
    let _e53 = i;
    let _e58 = hash((_e53 + vec2<f32>(1f, 0f)));
    let _e59 = f;
    let _e65 = u;
    let _e68 = i;
    let _e73 = hash((_e68 + vec2<f32>(0f, 1f)));
    let _e74 = f;
    let _e80 = i;
    let _e85 = hash((_e80 + vec2<f32>(1f, 1f)));
    let _e86 = f;
    let _e92 = u;
    let _e95 = u;
    n = mix(mix(dot(_e46, (_e47 - vec2<f32>(0f, 0f))), dot(_e58, (_e59 - vec2<f32>(1f, 0f))), _e65.x), mix(dot(_e73, (_e74 - vec2<f32>(0f, 1f))), dot(_e85, (_e86 - vec2<f32>(1f, 1f))), _e92.x), _e95.y);
    let _e99 = n;
    return ((_e99 * 0.5f) + 0.5f);
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
    let _e28 = n_2;
    let _e32 = seed_1;
    x_2 = (cos((_e28 * 6.2831855f)) + _e32);
    let _e35 = n_2;
    let _e39 = seed_1;
    y = (sin((_e35 * 6.2831855f)) + _e39);
    let _e42 = x_2;
    let _e43 = y;
    let _e45 = scale_1;
    let _e47 = noise2dgrad((vec2<f32>(_e42, _e43) * _e45));
    return _e47;
}

fn random1d(n_3: f32) -> f32 {
    var n_4: f32;

    n_4 = n_3;
    let _e24 = n_4;
    return fract((sin(_e24) * 43758.547f));
}

fn random2d(n_5: vec2<f32>) -> f32 {
    var n_6: vec2<f32>;

    n_6 = n_5;
    let _e24 = n_6;
    return fract((sin(dot(_e24, vec2<f32>(12.9898f, 4.1414f))) * 43758.547f));
}

fn rotate2d(coords: vec2<f32>, angle: f32) -> vec2<f32> {
    var coords_1: vec2<f32>;
    var angle_1: f32;
    var sin_factor: f32;
    var cos_factor: f32;

    coords_1 = coords;
    angle_1 = angle;
    let _e26 = angle_1;
    sin_factor = sin(_e26);
    let _e29 = angle_1;
    cos_factor = cos(_e29);
    let _e32 = coords_1;
    let _e36 = coords_1;
    let _e41 = cos_factor;
    let _e42 = sin_factor;
    let _e43 = sin_factor;
    let _e45 = cos_factor;
    coords_1 = (vec2<f32>((_e32.x - 0.5f), (_e36.y - 0.5f)) * mat2x2<f32>(vec2<f32>(_e41, _e42), vec2<f32>(-(_e43), _e45)));
    let _e50 = coords_1;
    coords_1 = (_e50 + vec2(0.5f));
    let _e54 = coords_1;
    return _e54;
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    var c_1: vec3<f32>;
    var K: vec4<f32> = vec4<f32>(1f, 0.6666667f, 0.33333334f, 3f);
    var p_2: vec3<f32>;

    c_1 = c;
    let _e34 = c_1;
    let _e36 = K;
    let _e42 = K;
    p_2 = abs(((fract((_e34.xxx + _e36.xyz)) * 6f) - _e42.www));
    let _e47 = c_1;
    let _e49 = K;
    let _e51 = p_2;
    let _e52 = K;
    let _e60 = c_1;
    return (_e47.z * mix(_e49.xxx, clamp((_e51 - _e52.xxx), vec3(0f), vec3(1f)), vec3(_e60.y)));
}

fn rgb2hsv(c_2: vec3<f32>) -> vec3<f32> {
    var c_3: vec3<f32>;
    var K_1: vec4<f32> = vec4<f32>(0f, -0.33333334f, 0.6666667f, -1f);
    var p_3: vec4<f32>;
    var q: vec4<f32>;
    var d: f32;
    var e: f32 = 0.0000000001f;

    c_3 = c_2;
    let _e36 = c_3;
    let _e37 = _e36.zy;
    let _e38 = K_1;
    let _e39 = _e38.wz;
    let _e45 = c_3;
    let _e46 = _e45.yz;
    let _e47 = K_1;
    let _e48 = _e47.xy;
    let _e54 = c_3;
    let _e56 = c_3;
    p_3 = mix(vec4<f32>(_e37.x, _e37.y, _e39.x, _e39.y), vec4<f32>(_e46.x, _e46.y, _e48.x, _e48.y), vec4(step(_e54.z, _e56.y)));
    let _e62 = p_3;
    let _e63 = _e62.xyw;
    let _e64 = c_3;
    let _e70 = c_3;
    let _e72 = p_3;
    let _e73 = _e72.yzx;
    let _e78 = p_3;
    let _e80 = c_3;
    q = mix(vec4<f32>(_e63.x, _e63.y, _e63.z, _e64.x), vec4<f32>(_e70.x, _e73.x, _e73.y, _e73.z), vec4(step(_e78.x, _e80.x)));
    let _e86 = q;
    let _e88 = q;
    let _e90 = q;
    d = (_e86.x - min(_e88.w, _e90.y));
    let _e97 = q;
    let _e99 = q;
    let _e101 = q;
    let _e105 = d;
    let _e107 = e;
    let _e112 = d;
    let _e113 = q;
    let _e115 = e;
    let _e118 = q;
    return vec3<f32>(abs((_e97.z + ((_e99.w - _e101.y) / ((6f * _e105) + _e107)))), (_e112 / (_e113.x + _e115)), _e118.x);
}

fn main_1() {
    var new_: vec4<f32>;
    var uvOld: vec2<f32>;
    var uNoise: f32;
    var vNoise: f32;
    var old: vec4<f32>;
    var hsv: vec3<f32>;
    var col: vec4<f32>;

    let _e22 = vUv_1;
    let _e23 = textureSample(tDiffuse_t, tDiffuse_s, _e22);
    new_ = _e23;
    let _e25 = vUv_1;
    uvOld = _e25;
    let _e27 = uvOld;
    uvOld = (_e27 - vec2(0.5f));
    let _e31 = uvOld;
    let _e33 = global.scale;
    uvOld = (_e31 * (1f - (_e33 * 0.02f)));
    let _e38 = uvOld;
    uvOld = (_e38 + vec2(0.5f));
    let _e42 = uvOld;
    let _e44 = global.rotate;
    let _e46 = rotate2d(_e42, (0.04f * _e44));
    uvOld = _e46;
    let _e47 = vUv_1;
    let _e49 = global.exportProgress;
    let _e53 = global.seed;
    let _e55 = noiseLoop1d((_e47.y + _e49), 0.5f, (1.23412f + _e53));
    uNoise = ((_e55 - 0.5f) * 2f);
    let _e61 = vUv_1;
    let _e63 = global.exportProgress;
    let _e67 = global.seed;
    let _e69 = noiseLoop1d((_e61.y + _e63), 0.5f, (99.4214f + _e67));
    vNoise = ((_e69 - 0.5f) * 2f);
    let _e75 = uvOld;
    let _e76 = uNoise;
    let _e77 = vNoise;
    let _e79 = global.warp;
    uvOld = (_e75 + ((vec2<f32>(_e76, _e77) * _e79) * 0.02f));
    let _e84 = uvOld;
    let _e85 = textureSample(tPrev_t, tPrev_s, _e84);
    old = _e85;
    let _e87 = old;
    let _e89 = rgb2hsv(_e87.xyz);
    hsv = _e89;
    let _e92 = hsv;
    let _e95 = global.hueShift;
    hsv.x = (_e92.x + (0.04f * _e95));
    let _e98 = old;
    let _e100 = hsv;
    let _e101 = hsv2rgb(_e100);
    old.x = _e101.x;
    old.y = _e101.y;
    old.z = _e101.z;
    let _e108 = old;
    let _e110 = global.amount;
    old = (_e108 * (0.5f + (_e110 * 0.48f)));
    let _e115 = old;
    let _e116 = new_;
    col = max(_e115, _e116);
    let _e119 = col;
    o_color = _e119;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e31 = o_color;
    return FragmentOutput(_e31);
}
