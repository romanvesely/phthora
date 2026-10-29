struct U {
    blend: f32,
    fade: f32,
    scaleUp: f32,
    flow: f32,
    amount: f32,
    direction: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const m_2: mat3x3<f32> = mat3x3<f32>(vec3<f32>(0f, 0.8f, 0.6f), vec3<f32>(-0.8f, 0.36f, -0.48f), vec3<f32>(-0.6f, -0.48f, 0.64f));

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;
@group(0) @binding(3) 
var tPrevious_t: texture_2d<f32>;
@group(0) @binding(4) 
var tPrevious_s: sampler;

fn mod289_(x: vec3<f32>) -> vec3<f32> {
    var x_1: vec3<f32>;

    x_1 = x;
    let _e20 = x_1;
    let _e21 = x_1;
    return (_e20 - (floor((_e21 * 0.0034602077f)) * 289f));
}

fn mod289_1(x_2: vec4<f32>) -> vec4<f32> {
    var x_3: vec4<f32>;

    x_3 = x_2;
    let _e20 = x_3;
    let _e21 = x_3;
    return (_e20 - (floor((_e21 * 0.0034602077f)) * 289f));
}

fn permute(x_4: vec4<f32>) -> vec4<f32> {
    var x_5: vec4<f32>;

    x_5 = x_4;
    let _e20 = x_5;
    let _e26 = x_5;
    let _e28 = mod289_1((((_e20 * 34f) + vec4(1f)) * _e26));
    return _e28;
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    var r_1: vec4<f32>;

    r_1 = r;
    let _e22 = r_1;
    return (vec4(1.7928429f) - (0.85373473f * _e22));
}

fn noise3d(v: vec3<f32>) -> f32 {
    var v_1: vec3<f32>;
    var C: vec2<f32> = vec2<f32>(0.16666667f, 0.33333334f);
    var D: vec4<f32> = vec4<f32>(0f, 0.5f, 1f, 2f);
    var i: vec3<f32>;
    var x0_: vec3<f32>;
    var g: vec3<f32>;
    var l: vec3<f32>;
    var i1_: vec3<f32>;
    var i2_: vec3<f32>;
    var x1_: vec3<f32>;
    var x2_: vec3<f32>;
    var x3_: vec3<f32>;
    var p: vec4<f32>;
    var n: f32 = 0.14285715f;
    var ns: vec3<f32>;
    var j: vec4<f32>;
    var x_6: vec4<f32>;
    var y: vec4<f32>;
    var x_7: vec4<f32>;
    var y_1: vec4<f32>;
    var h: vec4<f32>;
    var b0_: vec4<f32>;
    var b1_: vec4<f32>;
    var s0_: vec4<f32>;
    var s1_: vec4<f32>;
    var sh: vec4<f32>;
    var a0_: vec4<f32>;
    var a1_: vec4<f32>;
    var p0_: vec3<f32>;
    var p1_: vec3<f32>;
    var p2_: vec3<f32>;
    var p3_: vec3<f32>;
    var norm: vec4<f32>;
    var m: vec4<f32>;

    v_1 = v;
    let _e34 = v_1;
    let _e35 = v_1;
    let _e36 = C;
    i = floor((_e34 + vec3(dot(_e35, _e36.yyy))));
    let _e43 = v_1;
    let _e44 = i;
    let _e46 = i;
    let _e47 = C;
    x0_ = ((_e43 - _e44) + vec3(dot(_e46, _e47.xxx)));
    let _e53 = x0_;
    let _e55 = x0_;
    g = step(_e53.yzx, _e55.xyz);
    let _e60 = g;
    l = (vec3(1f) - _e60);
    let _e64 = g;
    let _e66 = l;
    i1_ = min(_e64.xyz, _e66.zxy);
    let _e70 = g;
    let _e72 = l;
    i2_ = max(_e70.xyz, _e72.zxy);
    let _e76 = x0_;
    let _e77 = i1_;
    let _e79 = C;
    x1_ = ((_e76 - _e77) + _e79.xxx);
    let _e83 = x0_;
    let _e84 = i2_;
    let _e86 = C;
    x2_ = ((_e83 - _e84) + _e86.yyy);
    let _e90 = x0_;
    let _e91 = D;
    x3_ = (_e90 - _e91.yyy);
    let _e95 = i;
    let _e96 = mod289_(_e95);
    i = _e96;
    let _e97 = i;
    let _e100 = i1_;
    let _e102 = i2_;
    let _e108 = permute((vec4(_e97.z) + vec4<f32>(0f, _e100.z, _e102.z, 1f)));
    let _e109 = i;
    let _e114 = i1_;
    let _e116 = i2_;
    let _e121 = permute(((_e108 + vec4(_e109.y)) + vec4<f32>(0f, _e114.y, _e116.y, 1f)));
    let _e122 = i;
    let _e127 = i1_;
    let _e129 = i2_;
    let _e134 = permute(((_e121 + vec4(_e122.x)) + vec4<f32>(0f, _e127.x, _e129.x, 1f)));
    p = _e134;
    let _e138 = n;
    let _e139 = D;
    let _e142 = D;
    ns = ((_e138 * _e139.wyz) - _e142.xzx);
    let _e146 = p;
    let _e148 = p;
    let _e149 = ns;
    let _e152 = ns;
    j = (_e146 - (49f * floor(((_e148 * _e149.z) * _e152.z))));
    let _e159 = j;
    let _e160 = ns;
    x_6 = floor((_e159 * _e160.z));
    let _e165 = j;
    let _e167 = x_6;
    y = floor((_e165 - (7f * _e167)));
    let _e172 = x_6;
    let _e173 = ns;
    let _e176 = ns;
    x_7 = ((_e172 * _e173.x) + _e176.yyyy);
    let _e180 = y;
    let _e181 = ns;
    let _e184 = ns;
    y_1 = ((_e180 * _e181.x) + _e184.yyyy);
    let _e189 = x_7;
    let _e193 = y_1;
    h = ((vec4(1f) - abs(_e189)) - abs(_e193));
    let _e197 = x_7;
    let _e198 = _e197.xy;
    let _e199 = y_1;
    let _e200 = _e199.xy;
    b0_ = vec4<f32>(_e198.x, _e198.y, _e200.x, _e200.y);
    let _e207 = x_7;
    let _e208 = _e207.zw;
    let _e209 = y_1;
    let _e210 = _e209.zw;
    b1_ = vec4<f32>(_e208.x, _e208.y, _e210.x, _e210.y);
    let _e217 = b0_;
    s0_ = ((floor(_e217) * 2f) + vec4(1f));
    let _e225 = b1_;
    s1_ = ((floor(_e225) * 2f) + vec4(1f));
    let _e233 = h;
    sh = -(step(_e233, vec4(0f)));
    let _e239 = b0_;
    let _e241 = s0_;
    let _e243 = sh;
    a0_ = (_e239.xzyw + (_e241.xzyw * _e243.xxyy));
    let _e248 = b1_;
    let _e250 = s1_;
    let _e252 = sh;
    a1_ = (_e248.xzyw + (_e250.xzyw * _e252.zzww));
    let _e257 = a0_;
    let _e258 = _e257.xy;
    let _e259 = h;
    p0_ = vec3<f32>(_e258.x, _e258.y, _e259.x);
    let _e265 = a0_;
    let _e266 = _e265.zw;
    let _e267 = h;
    p1_ = vec3<f32>(_e266.x, _e266.y, _e267.y);
    let _e273 = a1_;
    let _e274 = _e273.xy;
    let _e275 = h;
    p2_ = vec3<f32>(_e274.x, _e274.y, _e275.z);
    let _e281 = a1_;
    let _e282 = _e281.zw;
    let _e283 = h;
    p3_ = vec3<f32>(_e282.x, _e282.y, _e283.w);
    let _e289 = p0_;
    let _e290 = p0_;
    let _e292 = p1_;
    let _e293 = p1_;
    let _e295 = p2_;
    let _e296 = p2_;
    let _e298 = p3_;
    let _e299 = p3_;
    let _e302 = taylorInvSqrt(vec4<f32>(dot(_e289, _e290), dot(_e292, _e293), dot(_e295, _e296), dot(_e298, _e299)));
    norm = _e302;
    let _e304 = p0_;
    let _e305 = norm;
    p0_ = (_e304 * _e305.x);
    let _e308 = p1_;
    let _e309 = norm;
    p1_ = (_e308 * _e309.y);
    let _e312 = p2_;
    let _e313 = norm;
    p2_ = (_e312 * _e313.z);
    let _e316 = p3_;
    let _e317 = norm;
    p3_ = (_e316 * _e317.w);
    let _e321 = x0_;
    let _e322 = x0_;
    let _e324 = x1_;
    let _e325 = x1_;
    let _e327 = x2_;
    let _e328 = x2_;
    let _e330 = x3_;
    let _e331 = x3_;
    m = max((vec4(0.6f) - vec4<f32>(dot(_e321, _e322), dot(_e324, _e325), dot(_e327, _e328), dot(_e330, _e331))), vec4(0f));
    let _e340 = m;
    let _e341 = m;
    m = (_e340 * _e341);
    let _e344 = m;
    let _e345 = m;
    let _e347 = p0_;
    let _e348 = x0_;
    let _e350 = p1_;
    let _e351 = x1_;
    let _e353 = p2_;
    let _e354 = x2_;
    let _e356 = p3_;
    let _e357 = x3_;
    return (42f * dot((_e344 * _e345), vec4<f32>(dot(_e347, _e348), dot(_e350, _e351), dot(_e353, _e354), dot(_e356, _e357))));
}

fn fbm(p_1: vec3<f32>) -> f32 {
    var p_2: vec3<f32>;
    var f: f32 = 0f;

    p_2 = p_1;
    let _e23 = f;
    let _e25 = p_2;
    let _e26 = noise3d(_e25);
    f = (_e23 + (0.5f * _e26));
    let _e29 = p_2;
    p_2 = ((m_2 * _e29) * 2.02f);
    let _e33 = f;
    let _e35 = p_2;
    let _e36 = noise3d(_e35);
    f = (_e33 + (0.25f * _e36));
    let _e39 = p_2;
    p_2 = ((m_2 * _e39) * 2.03f);
    let _e43 = f;
    let _e45 = p_2;
    let _e46 = noise3d(_e45);
    f = (_e43 + (0.125f * _e46));
    let _e49 = p_2;
    p_2 = ((m_2 * _e49) * 2.01f);
    let _e53 = f;
    let _e55 = p_2;
    let _e56 = noise3d(_e55);
    f = (_e53 + (0.0625f * _e56));
    let _e59 = f;
    return (_e59 / 0.9375f);
}

fn map(value: f32, inMin: f32, inMax: f32, outMin: f32, outMax: f32) -> f32 {
    var value_1: f32;
    var inMin_1: f32;
    var inMax_1: f32;
    var outMin_1: f32;
    var outMax_1: f32;

    value_1 = value;
    inMin_1 = inMin;
    inMax_1 = inMax;
    outMin_1 = outMin;
    outMax_1 = outMax;
    let _e29 = outMin_1;
    let _e30 = outMax_1;
    let _e31 = outMin_1;
    let _e33 = value_1;
    let _e34 = inMin_1;
    let _e37 = inMax_1;
    let _e38 = inMin_1;
    return (_e29 + (((_e30 - _e31) * (_e33 - _e34)) / (_e37 - _e38)));
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m_1: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e23 = theta_1;
    let _e25 = theta_1;
    let _e28 = theta_1;
    let _e30 = theta_1;
    m_1 = mat2x2<f32>(vec2<f32>(cos(_e23), -(sin(_e25))), vec2<f32>(sin(_e28), cos(_e30)));
    let _e36 = m_1;
    let _e37 = position_1;
    return (_e36 * _e37);
}

fn main_1() {
    var amount2_: f32;
    var flow2_: f32;
    var uv: vec2<f32>;
    var orig: vec4<f32>;
    var intensity: vec3<f32>;
    var vidSample: f32;
    var vidSampleDx: f32;
    var vidSampleDy: f32;
    var flow: vec2<f32>;
    var cc: vec3<f32>;
    var col: vec4<f32>;
    var amount3_: f32;

    let _e19 = global.amount;
    amount2_ = (_e19 * 0.015f);
    let _e23 = global.flow;
    let _e28 = map(_e23, 0f, 1f, 0.2f, 0.01f);
    flow2_ = _e28;
    let _e30 = vUv_1;
    uv = _e30;
    let _e32 = uv;
    let _e33 = textureSample(tDiffuse_t, tDiffuse_s, _e32);
    orig = _e33;
    let _e36 = orig;
    intensity = (vec3(1f) - _e36.xyz);
    let _e43 = uv;
    let _e44 = textureSample(tDiffuse_t, tDiffuse_s, _e43);
    vidSample = dot(vec3(1f), _e44.xyz);
    let _e50 = uv;
    let _e51 = amount2_;
    let _e55 = textureSample(tDiffuse_t, tDiffuse_s, (_e50 + vec2<f32>(_e51, 0f)));
    vidSampleDx = dot(vec3(1f), _e55.xyz);
    let _e61 = uv;
    let _e63 = amount2_;
    let _e66 = textureSample(tDiffuse_t, tDiffuse_s, (_e61 + vec2<f32>(0f, _e63)));
    vidSampleDy = dot(vec3(1f), _e66.xyz);
    let _e70 = amount2_;
    let _e71 = vidSampleDy;
    let _e72 = vidSample;
    let _e74 = vidSample;
    let _e75 = vidSampleDx;
    flow = (_e70 * vec2<f32>((_e71 - _e72), (_e74 - _e75)));
    let _e80 = flow;
    let _e81 = global.direction;
    let _e84 = rotate2d(_e80, (_e81 * 6.2831855f));
    flow = _e84;
    let _e85 = uv;
    let _e90 = flow;
    let _e93 = textureSample(tPrevious_t, tPrevious_s, (_e85 + (vec2<f32>(-1f, 1f) * _e90)));
    cc = _e93.xyz;
    let _e96 = flow2_;
    let _e97 = intensity;
    let _e100 = flow2_;
    let _e103 = cc;
    intensity = ((_e96 * _e97) + ((1f - _e100) * (vec3(1f) - _e103)));
    let _e109 = intensity;
    let _e111 = (vec3(1f) - _e109);
    let _e112 = orig;
    col = vec4<f32>(_e111.x, _e111.y, _e111.z, _e112.w);
    let _e121 = global.amount;
    amount3_ = smoothstep(0f, 0.1f, _e121);
    let _e124 = orig;
    let _e125 = col;
    let _e126 = amount3_;
    col = mix(_e124, _e125, vec4(_e126));
    let _e129 = col;
    o_color = _e129;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e29 = o_color;
    return FragmentOutput(_e29);
}
