struct U {
    amount: f32,
    colorize: f32,
    speed: f32,
    time: f32,
    scale: f32,
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

fn mod289_(x: vec3<f32>) -> vec3<f32> {
    var x_1: vec3<f32>;

    x_1 = x;
    let _e18 = x_1;
    let _e19 = x_1;
    return (_e18 - (floor((_e19 * 0.0034602077f)) * 289f));
}

fn mod289_1(x_2: vec4<f32>) -> vec4<f32> {
    var x_3: vec4<f32>;

    x_3 = x_2;
    let _e18 = x_3;
    let _e19 = x_3;
    return (_e18 - (floor((_e19 * 0.0034602077f)) * 289f));
}

fn permute(x_4: vec4<f32>) -> vec4<f32> {
    var x_5: vec4<f32>;

    x_5 = x_4;
    let _e18 = x_5;
    let _e24 = x_5;
    let _e26 = mod289_1((((_e18 * 34f) + vec4(1f)) * _e24));
    return _e26;
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    var r_1: vec4<f32>;

    r_1 = r;
    let _e20 = r_1;
    return (vec4(1.7928429f) - (0.85373473f * _e20));
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
    let _e32 = v_1;
    let _e33 = v_1;
    let _e34 = C;
    i = floor((_e32 + vec3(dot(_e33, _e34.yyy))));
    let _e41 = v_1;
    let _e42 = i;
    let _e44 = i;
    let _e45 = C;
    x0_ = ((_e41 - _e42) + vec3(dot(_e44, _e45.xxx)));
    let _e51 = x0_;
    let _e53 = x0_;
    g = step(_e51.yzx, _e53.xyz);
    let _e58 = g;
    l = (vec3(1f) - _e58);
    let _e62 = g;
    let _e64 = l;
    i1_ = min(_e62.xyz, _e64.zxy);
    let _e68 = g;
    let _e70 = l;
    i2_ = max(_e68.xyz, _e70.zxy);
    let _e74 = x0_;
    let _e75 = i1_;
    let _e77 = C;
    x1_ = ((_e74 - _e75) + _e77.xxx);
    let _e81 = x0_;
    let _e82 = i2_;
    let _e84 = C;
    x2_ = ((_e81 - _e82) + _e84.yyy);
    let _e88 = x0_;
    let _e89 = D;
    x3_ = (_e88 - _e89.yyy);
    let _e93 = i;
    let _e94 = mod289_(_e93);
    i = _e94;
    let _e95 = i;
    let _e98 = i1_;
    let _e100 = i2_;
    let _e106 = permute((vec4(_e95.z) + vec4<f32>(0f, _e98.z, _e100.z, 1f)));
    let _e107 = i;
    let _e112 = i1_;
    let _e114 = i2_;
    let _e119 = permute(((_e106 + vec4(_e107.y)) + vec4<f32>(0f, _e112.y, _e114.y, 1f)));
    let _e120 = i;
    let _e125 = i1_;
    let _e127 = i2_;
    let _e132 = permute(((_e119 + vec4(_e120.x)) + vec4<f32>(0f, _e125.x, _e127.x, 1f)));
    p = _e132;
    let _e136 = n;
    let _e137 = D;
    let _e140 = D;
    ns = ((_e136 * _e137.wyz) - _e140.xzx);
    let _e144 = p;
    let _e146 = p;
    let _e147 = ns;
    let _e150 = ns;
    j = (_e144 - (49f * floor(((_e146 * _e147.z) * _e150.z))));
    let _e157 = j;
    let _e158 = ns;
    x_6 = floor((_e157 * _e158.z));
    let _e163 = j;
    let _e165 = x_6;
    y = floor((_e163 - (7f * _e165)));
    let _e170 = x_6;
    let _e171 = ns;
    let _e174 = ns;
    x_7 = ((_e170 * _e171.x) + _e174.yyyy);
    let _e178 = y;
    let _e179 = ns;
    let _e182 = ns;
    y_1 = ((_e178 * _e179.x) + _e182.yyyy);
    let _e187 = x_7;
    let _e191 = y_1;
    h = ((vec4(1f) - abs(_e187)) - abs(_e191));
    let _e195 = x_7;
    let _e196 = _e195.xy;
    let _e197 = y_1;
    let _e198 = _e197.xy;
    b0_ = vec4<f32>(_e196.x, _e196.y, _e198.x, _e198.y);
    let _e205 = x_7;
    let _e206 = _e205.zw;
    let _e207 = y_1;
    let _e208 = _e207.zw;
    b1_ = vec4<f32>(_e206.x, _e206.y, _e208.x, _e208.y);
    let _e215 = b0_;
    s0_ = ((floor(_e215) * 2f) + vec4(1f));
    let _e223 = b1_;
    s1_ = ((floor(_e223) * 2f) + vec4(1f));
    let _e231 = h;
    sh = -(step(_e231, vec4(0f)));
    let _e237 = b0_;
    let _e239 = s0_;
    let _e241 = sh;
    a0_ = (_e237.xzyw + (_e239.xzyw * _e241.xxyy));
    let _e246 = b1_;
    let _e248 = s1_;
    let _e250 = sh;
    a1_ = (_e246.xzyw + (_e248.xzyw * _e250.zzww));
    let _e255 = a0_;
    let _e256 = _e255.xy;
    let _e257 = h;
    p0_ = vec3<f32>(_e256.x, _e256.y, _e257.x);
    let _e263 = a0_;
    let _e264 = _e263.zw;
    let _e265 = h;
    p1_ = vec3<f32>(_e264.x, _e264.y, _e265.y);
    let _e271 = a1_;
    let _e272 = _e271.xy;
    let _e273 = h;
    p2_ = vec3<f32>(_e272.x, _e272.y, _e273.z);
    let _e279 = a1_;
    let _e280 = _e279.zw;
    let _e281 = h;
    p3_ = vec3<f32>(_e280.x, _e280.y, _e281.w);
    let _e287 = p0_;
    let _e288 = p0_;
    let _e290 = p1_;
    let _e291 = p1_;
    let _e293 = p2_;
    let _e294 = p2_;
    let _e296 = p3_;
    let _e297 = p3_;
    let _e300 = taylorInvSqrt(vec4<f32>(dot(_e287, _e288), dot(_e290, _e291), dot(_e293, _e294), dot(_e296, _e297)));
    norm = _e300;
    let _e302 = p0_;
    let _e303 = norm;
    p0_ = (_e302 * _e303.x);
    let _e306 = p1_;
    let _e307 = norm;
    p1_ = (_e306 * _e307.y);
    let _e310 = p2_;
    let _e311 = norm;
    p2_ = (_e310 * _e311.z);
    let _e314 = p3_;
    let _e315 = norm;
    p3_ = (_e314 * _e315.w);
    let _e319 = x0_;
    let _e320 = x0_;
    let _e322 = x1_;
    let _e323 = x1_;
    let _e325 = x2_;
    let _e326 = x2_;
    let _e328 = x3_;
    let _e329 = x3_;
    m = max((vec4(0.6f) - vec4<f32>(dot(_e319, _e320), dot(_e322, _e323), dot(_e325, _e326), dot(_e328, _e329))), vec4(0f));
    let _e338 = m;
    let _e339 = m;
    m = (_e338 * _e339);
    let _e342 = m;
    let _e343 = m;
    let _e345 = p0_;
    let _e346 = x0_;
    let _e348 = p1_;
    let _e349 = x1_;
    let _e351 = p2_;
    let _e352 = x2_;
    let _e354 = p3_;
    let _e355 = x3_;
    return (42f * dot((_e342 * _e343), vec4<f32>(dot(_e345, _e346), dot(_e348, _e349), dot(_e351, _e352), dot(_e354, _e355))));
}

fn nfielder(uv: vec2<f32>, time: f32) -> f32 {
    var uv_1: vec2<f32>;
    var time_1: f32;
    var n_1: f32;

    uv_1 = uv;
    time_1 = time;
    let _e20 = uv_1;
    let _e23 = time_1;
    let _e27 = ((_e20 * 2f) + vec2((_e23 * 0.4f)));
    let _e28 = time_1;
    let _e34 = noise3d(vec3<f32>(_e27.x, _e27.y, (_e28 * 1.6f)));
    n_1 = ((_e34 / 2f) + 0.5f);
    let _e40 = n_1;
    return pow(_e40, 3f);
}

fn random2d(n_2: vec2<f32>) -> f32 {
    var n_3: vec2<f32>;

    n_3 = n_2;
    let _e18 = n_3;
    return fract((sin(dot(_e18, vec2<f32>(12.9898f, 4.1414f))) * 43758.547f));
}

fn steppedT(time_2: f32) -> f32 {
    var time_3: f32;
    var fps: f32 = 30f;

    time_3 = time_2;
    let _e20 = time_3;
    let _e21 = fps;
    return floor((_e20 * _e21));
}

fn getRndOffset(uv_2: vec2<f32>, time_4: f32, chunks: f32, steppedT_1: f32, cutoff: f32) -> vec2<f32> {
    var uv_3: vec2<f32>;
    var time_5: f32;
    var chunks_1: f32;
    var steppedT_2: f32;
    var cutoff_1: f32;
    var uvChunks: vec2<f32>;
    var nfield: f32;
    var chunkRnd: f32;
    var off: vec2<f32>;
    var maxOffset: f32;

    uv_3 = uv_2;
    time_5 = time_4;
    chunks_1 = chunks;
    steppedT_2 = steppedT_1;
    cutoff_1 = cutoff;
    let _e26 = uv_3;
    let _e27 = chunks_1;
    let _e30 = chunks_1;
    uvChunks = (floor((_e26 * _e27)) / vec2(_e30));
    let _e34 = uvChunks;
    let _e35 = time_5;
    let _e36 = nfielder(_e34, _e35);
    nfield = _e36;
    let _e38 = uvChunks;
    let _e39 = steppedT_2;
    let _e41 = random2d((_e38 * _e39));
    chunkRnd = _e41;
    let _e43 = uvChunks;
    let _e44 = steppedT_2;
    let _e46 = random2d((_e43 * _e44));
    let _e47 = uvChunks;
    let _e48 = steppedT_2;
    let _e52 = random2d(((_e47 * _e48) * 84.86f));
    off = (vec2<f32>(_e46, _e52) - vec2(0.5f));
    let _e59 = chunks_1;
    maxOffset = ((1f / _e59) * 0.4f);
    let _e64 = off;
    let _e65 = maxOffset;
    off = (_e64 * _e65);
    let _e67 = off;
    let _e68 = nfield;
    off = (_e67 * (_e68 * 4f));
    let _e72 = off;
    return _e72;
}

fn main_1() {
    var time2_: f32;
    var st: f32;
    var uv_4: vec2<f32>;
    var off_1: vec2<f32>;
    var col: vec4<f32>;

    let _e16 = global.time;
    let _e17 = global.speed;
    let _e21 = global.seed;
    time2_ = (((_e16 * _e17) * 2f) + _e21);
    let _e24 = time2_;
    let _e25 = steppedT(_e24);
    st = _e25;
    let _e27 = vUv_1;
    uv_4 = (_e27 - vec2(0.5f));
    let _e32 = uv_4;
    let _e33 = global.scale;
    uv_4 = (_e32 * (_e33 * 2f));
    let _e37 = uv_4;
    uv_4 = (_e37 + vec2(0.5f));
    let _e41 = uv_4;
    let _e42 = time2_;
    let _e44 = st;
    let _e46 = getRndOffset(_e41, _e42, 65f, _e44, 0.4f);
    off_1 = _e46;
    let _e48 = off_1;
    let _e49 = global.amount;
    off_1 = (_e48 * (_e49 * 2f));
    let _e53 = vUv_1;
    let _e54 = off_1;
    let _e56 = textureSample(tDiffuse_t, tDiffuse_s, (_e53 + _e54));
    col = _e56;
    let _e59 = col;
    let _e61 = off_1;
    let _e65 = global.colorize;
    col.z = (_e59.z + ((length(_e61) * 25f) * _e65));
    let _e68 = col;
    o_color = _e68;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e23 = o_color;
    return FragmentOutput(_e23);
}
