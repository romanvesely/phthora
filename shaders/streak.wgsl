struct U {
    amount: f32,
    time: f32,
    shift: f32,
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

fn mod289_(x: vec3<f32>) -> vec3<f32> {
    var x_1: vec3<f32>;

    x_1 = x;
    let _e16 = x_1;
    let _e17 = x_1;
    return (_e16 - (floor((_e17 * 0.0034602077f)) * 289f));
}

fn mod289_1(x_2: vec4<f32>) -> vec4<f32> {
    var x_3: vec4<f32>;

    x_3 = x_2;
    let _e16 = x_3;
    let _e17 = x_3;
    return (_e16 - (floor((_e17 * 0.0034602077f)) * 289f));
}

fn permute(x_4: vec4<f32>) -> vec4<f32> {
    var x_5: vec4<f32>;

    x_5 = x_4;
    let _e16 = x_5;
    let _e22 = x_5;
    let _e24 = mod289_1((((_e16 * 34f) + vec4(1f)) * _e22));
    return _e24;
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    var r_1: vec4<f32>;

    r_1 = r;
    let _e18 = r_1;
    return (vec4(1.7928429f) - (0.85373473f * _e18));
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
    let _e30 = v_1;
    let _e31 = v_1;
    let _e32 = C;
    i = floor((_e30 + vec3(dot(_e31, _e32.yyy))));
    let _e39 = v_1;
    let _e40 = i;
    let _e42 = i;
    let _e43 = C;
    x0_ = ((_e39 - _e40) + vec3(dot(_e42, _e43.xxx)));
    let _e49 = x0_;
    let _e51 = x0_;
    g = step(_e49.yzx, _e51.xyz);
    let _e56 = g;
    l = (vec3(1f) - _e56);
    let _e60 = g;
    let _e62 = l;
    i1_ = min(_e60.xyz, _e62.zxy);
    let _e66 = g;
    let _e68 = l;
    i2_ = max(_e66.xyz, _e68.zxy);
    let _e72 = x0_;
    let _e73 = i1_;
    let _e75 = C;
    x1_ = ((_e72 - _e73) + _e75.xxx);
    let _e79 = x0_;
    let _e80 = i2_;
    let _e82 = C;
    x2_ = ((_e79 - _e80) + _e82.yyy);
    let _e86 = x0_;
    let _e87 = D;
    x3_ = (_e86 - _e87.yyy);
    let _e91 = i;
    let _e92 = mod289_(_e91);
    i = _e92;
    let _e93 = i;
    let _e96 = i1_;
    let _e98 = i2_;
    let _e104 = permute((vec4(_e93.z) + vec4<f32>(0f, _e96.z, _e98.z, 1f)));
    let _e105 = i;
    let _e110 = i1_;
    let _e112 = i2_;
    let _e117 = permute(((_e104 + vec4(_e105.y)) + vec4<f32>(0f, _e110.y, _e112.y, 1f)));
    let _e118 = i;
    let _e123 = i1_;
    let _e125 = i2_;
    let _e130 = permute(((_e117 + vec4(_e118.x)) + vec4<f32>(0f, _e123.x, _e125.x, 1f)));
    p = _e130;
    let _e134 = n;
    let _e135 = D;
    let _e138 = D;
    ns = ((_e134 * _e135.wyz) - _e138.xzx);
    let _e142 = p;
    let _e144 = p;
    let _e145 = ns;
    let _e148 = ns;
    j = (_e142 - (49f * floor(((_e144 * _e145.z) * _e148.z))));
    let _e155 = j;
    let _e156 = ns;
    x_6 = floor((_e155 * _e156.z));
    let _e161 = j;
    let _e163 = x_6;
    y = floor((_e161 - (7f * _e163)));
    let _e168 = x_6;
    let _e169 = ns;
    let _e172 = ns;
    x_7 = ((_e168 * _e169.x) + _e172.yyyy);
    let _e176 = y;
    let _e177 = ns;
    let _e180 = ns;
    y_1 = ((_e176 * _e177.x) + _e180.yyyy);
    let _e185 = x_7;
    let _e189 = y_1;
    h = ((vec4(1f) - abs(_e185)) - abs(_e189));
    let _e193 = x_7;
    let _e194 = _e193.xy;
    let _e195 = y_1;
    let _e196 = _e195.xy;
    b0_ = vec4<f32>(_e194.x, _e194.y, _e196.x, _e196.y);
    let _e203 = x_7;
    let _e204 = _e203.zw;
    let _e205 = y_1;
    let _e206 = _e205.zw;
    b1_ = vec4<f32>(_e204.x, _e204.y, _e206.x, _e206.y);
    let _e213 = b0_;
    s0_ = ((floor(_e213) * 2f) + vec4(1f));
    let _e221 = b1_;
    s1_ = ((floor(_e221) * 2f) + vec4(1f));
    let _e229 = h;
    sh = -(step(_e229, vec4(0f)));
    let _e235 = b0_;
    let _e237 = s0_;
    let _e239 = sh;
    a0_ = (_e235.xzyw + (_e237.xzyw * _e239.xxyy));
    let _e244 = b1_;
    let _e246 = s1_;
    let _e248 = sh;
    a1_ = (_e244.xzyw + (_e246.xzyw * _e248.zzww));
    let _e253 = a0_;
    let _e254 = _e253.xy;
    let _e255 = h;
    p0_ = vec3<f32>(_e254.x, _e254.y, _e255.x);
    let _e261 = a0_;
    let _e262 = _e261.zw;
    let _e263 = h;
    p1_ = vec3<f32>(_e262.x, _e262.y, _e263.y);
    let _e269 = a1_;
    let _e270 = _e269.xy;
    let _e271 = h;
    p2_ = vec3<f32>(_e270.x, _e270.y, _e271.z);
    let _e277 = a1_;
    let _e278 = _e277.zw;
    let _e279 = h;
    p3_ = vec3<f32>(_e278.x, _e278.y, _e279.w);
    let _e285 = p0_;
    let _e286 = p0_;
    let _e288 = p1_;
    let _e289 = p1_;
    let _e291 = p2_;
    let _e292 = p2_;
    let _e294 = p3_;
    let _e295 = p3_;
    let _e298 = taylorInvSqrt(vec4<f32>(dot(_e285, _e286), dot(_e288, _e289), dot(_e291, _e292), dot(_e294, _e295)));
    norm = _e298;
    let _e300 = p0_;
    let _e301 = norm;
    p0_ = (_e300 * _e301.x);
    let _e304 = p1_;
    let _e305 = norm;
    p1_ = (_e304 * _e305.y);
    let _e308 = p2_;
    let _e309 = norm;
    p2_ = (_e308 * _e309.z);
    let _e312 = p3_;
    let _e313 = norm;
    p3_ = (_e312 * _e313.w);
    let _e317 = x0_;
    let _e318 = x0_;
    let _e320 = x1_;
    let _e321 = x1_;
    let _e323 = x2_;
    let _e324 = x2_;
    let _e326 = x3_;
    let _e327 = x3_;
    m = max((vec4(0.6f) - vec4<f32>(dot(_e317, _e318), dot(_e320, _e321), dot(_e323, _e324), dot(_e326, _e327))), vec4(0f));
    let _e336 = m;
    let _e337 = m;
    m = (_e336 * _e337);
    let _e340 = m;
    let _e341 = m;
    let _e343 = p0_;
    let _e344 = x0_;
    let _e346 = p1_;
    let _e347 = x1_;
    let _e349 = p2_;
    let _e350 = x2_;
    let _e352 = p3_;
    let _e353 = x3_;
    return (42f * dot((_e340 * _e341), vec4<f32>(dot(_e343, _e344), dot(_e346, _e347), dot(_e349, _e350), dot(_e352, _e353))));
}

fn mod289P(x_8: vec3<f32>) -> vec3<f32> {
    var x_9: vec3<f32>;

    x_9 = x_8;
    let _e16 = x_9;
    let _e17 = x_9;
    return (_e16 - (floor((_e17 * 0.0034602077f)) * 289f));
}

fn mod289P_1(x_10: vec4<f32>) -> vec4<f32> {
    var x_11: vec4<f32>;

    x_11 = x_10;
    let _e16 = x_11;
    let _e17 = x_11;
    return (_e16 - (floor((_e17 * 0.0034602077f)) * 289f));
}

fn permuteP(x_12: vec4<f32>) -> vec4<f32> {
    var x_13: vec4<f32>;

    x_13 = x_12;
    let _e16 = x_13;
    let _e22 = x_13;
    let _e24 = mod289P_1((((_e16 * 34f) + vec4(1f)) * _e22));
    return _e24;
}

fn taylorInvSqrtP(r_2: vec4<f32>) -> vec4<f32> {
    var r_3: vec4<f32>;

    r_3 = r_2;
    let _e18 = r_3;
    return (vec4(1.7928429f) - (0.85373473f * _e18));
}

fn fade(t: vec3<f32>) -> vec3<f32> {
    var t_1: vec3<f32>;

    t_1 = t;
    let _e16 = t_1;
    let _e17 = t_1;
    let _e19 = t_1;
    let _e21 = t_1;
    let _e22 = t_1;
    return (((_e16 * _e17) * _e19) * ((_e21 * ((_e22 * 6f) - vec3(15f))) + vec3(10f)));
}

fn pnoise3d(P: vec3<f32>, rep: vec3<f32>) -> f32 {
    var P_1: vec3<f32>;
    var rep_1: vec3<f32>;
    var Pi0_: vec3<f32>;
    var Pi1_: vec3<f32>;
    var Pf0_: vec3<f32>;
    var Pf1_: vec3<f32>;
    var ix: vec4<f32>;
    var iy: vec4<f32>;
    var iz0_: vec4<f32>;
    var iz1_: vec4<f32>;
    var ixy: vec4<f32>;
    var ixy0_: vec4<f32>;
    var ixy1_: vec4<f32>;
    var gx0_: vec4<f32>;
    var gy0_: vec4<f32>;
    var gz0_: vec4<f32>;
    var sz0_: vec4<f32>;
    var gx1_: vec4<f32>;
    var gy1_: vec4<f32>;
    var gz1_: vec4<f32>;
    var sz1_: vec4<f32>;
    var g000_: vec3<f32>;
    var g100_: vec3<f32>;
    var g010_: vec3<f32>;
    var g110_: vec3<f32>;
    var g001_: vec3<f32>;
    var g101_: vec3<f32>;
    var g011_: vec3<f32>;
    var g111_: vec3<f32>;
    var norm0_: vec4<f32>;
    var norm1_: vec4<f32>;
    var n000_: f32;
    var n100_: f32;
    var n010_: f32;
    var n110_: f32;
    var n001_: f32;
    var n101_: f32;
    var n011_: f32;
    var n111_: f32;
    var fade_xyz: vec3<f32>;
    var n_z: vec4<f32>;
    var n_yz: vec2<f32>;
    var n_xyz: f32;

    P_1 = P;
    rep_1 = rep;
    let _e18 = P_1;
    let _e19 = floor(_e18);
    let _e20 = rep_1;
    Pi0_ = (_e19 - (floor((_e19 / _e20)) * _e20));
    let _e26 = Pi0_;
    let _e29 = (_e26 + vec3(1f));
    let _e30 = rep_1;
    Pi1_ = (_e29 - (floor((_e29 / _e30)) * _e30));
    let _e36 = Pi0_;
    let _e37 = mod289P(_e36);
    Pi0_ = _e37;
    let _e38 = Pi1_;
    let _e39 = mod289P(_e38);
    Pi1_ = _e39;
    let _e40 = P_1;
    Pf0_ = fract(_e40);
    let _e43 = Pf0_;
    Pf1_ = (_e43 - vec3(1f));
    let _e48 = Pi0_;
    let _e50 = Pi1_;
    let _e52 = Pi0_;
    let _e54 = Pi1_;
    ix = vec4<f32>(_e48.x, _e50.x, _e52.x, _e54.x);
    let _e58 = Pi0_;
    let _e59 = _e58.yy;
    let _e60 = Pi1_;
    let _e61 = _e60.yy;
    iy = vec4<f32>(_e59.x, _e59.y, _e61.x, _e61.y);
    let _e68 = Pi0_;
    iz0_ = _e68.zzzz;
    let _e71 = Pi1_;
    iz1_ = _e71.zzzz;
    let _e74 = ix;
    let _e75 = permuteP(_e74);
    let _e76 = iy;
    let _e78 = permuteP((_e75 + _e76));
    ixy = _e78;
    let _e80 = ixy;
    let _e81 = iz0_;
    let _e83 = permuteP((_e80 + _e81));
    ixy0_ = _e83;
    let _e85 = ixy;
    let _e86 = iz1_;
    let _e88 = permuteP((_e85 + _e86));
    ixy1_ = _e88;
    let _e90 = ixy0_;
    gx0_ = (_e90 * 0.14285715f);
    let _e96 = gx0_;
    gy0_ = (fract((floor(_e96) * 0.14285715f)) - vec4(0.5f));
    let _e107 = gx0_;
    gx0_ = fract(_e107);
    let _e111 = gx0_;
    let _e114 = gy0_;
    gz0_ = ((vec4(0.5f) - abs(_e111)) - abs(_e114));
    let _e118 = gz0_;
    sz0_ = step(_e118, vec4(0f));
    let _e123 = gx0_;
    let _e124 = sz0_;
    let _e126 = gx0_;
    gx0_ = (_e123 - (_e124 * (step(vec4(0f), _e126) - vec4(0.5f))));
    let _e134 = gy0_;
    let _e135 = sz0_;
    let _e137 = gy0_;
    gy0_ = (_e134 - (_e135 * (step(vec4(0f), _e137) - vec4(0.5f))));
    let _e145 = ixy1_;
    gx1_ = (_e145 * 0.14285715f);
    let _e151 = gx1_;
    gy1_ = (fract((floor(_e151) * 0.14285715f)) - vec4(0.5f));
    let _e162 = gx1_;
    gx1_ = fract(_e162);
    let _e166 = gx1_;
    let _e169 = gy1_;
    gz1_ = ((vec4(0.5f) - abs(_e166)) - abs(_e169));
    let _e173 = gz1_;
    sz1_ = step(_e173, vec4(0f));
    let _e178 = gx1_;
    let _e179 = sz1_;
    let _e181 = gx1_;
    gx1_ = (_e178 - (_e179 * (step(vec4(0f), _e181) - vec4(0.5f))));
    let _e189 = gy1_;
    let _e190 = sz1_;
    let _e192 = gy1_;
    gy1_ = (_e189 - (_e190 * (step(vec4(0f), _e192) - vec4(0.5f))));
    let _e200 = gx0_;
    let _e202 = gy0_;
    let _e204 = gz0_;
    g000_ = vec3<f32>(_e200.x, _e202.x, _e204.x);
    let _e208 = gx0_;
    let _e210 = gy0_;
    let _e212 = gz0_;
    g100_ = vec3<f32>(_e208.y, _e210.y, _e212.y);
    let _e216 = gx0_;
    let _e218 = gy0_;
    let _e220 = gz0_;
    g010_ = vec3<f32>(_e216.z, _e218.z, _e220.z);
    let _e224 = gx0_;
    let _e226 = gy0_;
    let _e228 = gz0_;
    g110_ = vec3<f32>(_e224.w, _e226.w, _e228.w);
    let _e232 = gx1_;
    let _e234 = gy1_;
    let _e236 = gz1_;
    g001_ = vec3<f32>(_e232.x, _e234.x, _e236.x);
    let _e240 = gx1_;
    let _e242 = gy1_;
    let _e244 = gz1_;
    g101_ = vec3<f32>(_e240.y, _e242.y, _e244.y);
    let _e248 = gx1_;
    let _e250 = gy1_;
    let _e252 = gz1_;
    g011_ = vec3<f32>(_e248.z, _e250.z, _e252.z);
    let _e256 = gx1_;
    let _e258 = gy1_;
    let _e260 = gz1_;
    g111_ = vec3<f32>(_e256.w, _e258.w, _e260.w);
    let _e264 = g000_;
    let _e265 = g000_;
    let _e267 = g010_;
    let _e268 = g010_;
    let _e270 = g100_;
    let _e271 = g100_;
    let _e273 = g110_;
    let _e274 = g110_;
    let _e277 = taylorInvSqrtP(vec4<f32>(dot(_e264, _e265), dot(_e267, _e268), dot(_e270, _e271), dot(_e273, _e274)));
    norm0_ = _e277;
    let _e279 = g000_;
    let _e280 = norm0_;
    g000_ = (_e279 * _e280.x);
    let _e283 = g010_;
    let _e284 = norm0_;
    g010_ = (_e283 * _e284.y);
    let _e287 = g100_;
    let _e288 = norm0_;
    g100_ = (_e287 * _e288.z);
    let _e291 = g110_;
    let _e292 = norm0_;
    g110_ = (_e291 * _e292.w);
    let _e295 = g001_;
    let _e296 = g001_;
    let _e298 = g011_;
    let _e299 = g011_;
    let _e301 = g101_;
    let _e302 = g101_;
    let _e304 = g111_;
    let _e305 = g111_;
    let _e308 = taylorInvSqrtP(vec4<f32>(dot(_e295, _e296), dot(_e298, _e299), dot(_e301, _e302), dot(_e304, _e305)));
    norm1_ = _e308;
    let _e310 = g001_;
    let _e311 = norm1_;
    g001_ = (_e310 * _e311.x);
    let _e314 = g011_;
    let _e315 = norm1_;
    g011_ = (_e314 * _e315.y);
    let _e318 = g101_;
    let _e319 = norm1_;
    g101_ = (_e318 * _e319.z);
    let _e322 = g111_;
    let _e323 = norm1_;
    g111_ = (_e322 * _e323.w);
    let _e326 = g000_;
    let _e327 = Pf0_;
    n000_ = dot(_e326, _e327);
    let _e330 = g100_;
    let _e331 = Pf1_;
    let _e333 = Pf0_;
    let _e334 = _e333.yz;
    n100_ = dot(_e330, vec3<f32>(_e331.x, _e334.x, _e334.y));
    let _e340 = g010_;
    let _e341 = Pf0_;
    let _e343 = Pf1_;
    let _e345 = Pf0_;
    n010_ = dot(_e340, vec3<f32>(_e341.x, _e343.y, _e345.z));
    let _e350 = g110_;
    let _e351 = Pf1_;
    let _e352 = _e351.xy;
    let _e353 = Pf0_;
    n110_ = dot(_e350, vec3<f32>(_e352.x, _e352.y, _e353.z));
    let _e360 = g001_;
    let _e361 = Pf0_;
    let _e362 = _e361.xy;
    let _e363 = Pf1_;
    n001_ = dot(_e360, vec3<f32>(_e362.x, _e362.y, _e363.z));
    let _e370 = g101_;
    let _e371 = Pf1_;
    let _e373 = Pf0_;
    let _e375 = Pf1_;
    n101_ = dot(_e370, vec3<f32>(_e371.x, _e373.y, _e375.z));
    let _e380 = g011_;
    let _e381 = Pf0_;
    let _e383 = Pf1_;
    let _e384 = _e383.yz;
    n011_ = dot(_e380, vec3<f32>(_e381.x, _e384.x, _e384.y));
    let _e390 = g111_;
    let _e391 = Pf1_;
    n111_ = dot(_e390, _e391);
    let _e394 = Pf0_;
    let _e395 = fade(_e394);
    fade_xyz = _e395;
    let _e397 = n000_;
    let _e398 = n100_;
    let _e399 = n010_;
    let _e400 = n110_;
    let _e402 = n001_;
    let _e403 = n101_;
    let _e404 = n011_;
    let _e405 = n111_;
    let _e407 = fade_xyz;
    n_z = mix(vec4<f32>(_e397, _e398, _e399, _e400), vec4<f32>(_e402, _e403, _e404, _e405), vec4(_e407.z));
    let _e412 = n_z;
    let _e414 = n_z;
    let _e416 = fade_xyz;
    n_yz = mix(_e412.xy, _e414.zw, vec2(_e416.y));
    let _e421 = n_yz;
    let _e423 = n_yz;
    let _e425 = fade_xyz;
    n_xyz = mix(_e421.x, _e423.y, _e425.x);
    let _e430 = n_xyz;
    return (2.2f * _e430);
}

fn hash(x_14: vec2<f32>) -> vec2<f32> {
    var x_15: vec2<f32>;
    var k: vec2<f32> = vec2<f32>(0.3183099f, 0.3678794f);

    x_15 = x_14;
    let _e20 = x_15;
    let _e21 = k;
    let _e23 = k;
    x_15 = ((_e20 * _e21) + _e23.yx);
    let _e30 = k;
    let _e32 = x_15;
    let _e34 = x_15;
    let _e37 = x_15;
    let _e39 = x_15;
    return (vec2(-1f) + (2f * fract(((16f * _e30) * fract(((_e32.x * _e34.y) * (_e37.x + _e39.y)))))));
}

fn noise2dgrad(p_1: vec2<f32>) -> f32 {
    var p_2: vec2<f32>;
    var i_1: vec2<f32>;
    var f: vec2<f32>;
    var u: vec2<f32>;
    var n_1: f32;

    p_2 = p_1;
    let _e16 = p_2;
    i_1 = floor(_e16);
    let _e19 = p_2;
    f = fract(_e19);
    let _e22 = f;
    let _e23 = f;
    let _e27 = f;
    u = ((_e22 * _e23) * (vec2(3f) - (2f * _e27)));
    let _e33 = i_1;
    let _e38 = hash((_e33 + vec2<f32>(0f, 0f)));
    let _e39 = f;
    let _e45 = i_1;
    let _e50 = hash((_e45 + vec2<f32>(1f, 0f)));
    let _e51 = f;
    let _e57 = u;
    let _e60 = i_1;
    let _e65 = hash((_e60 + vec2<f32>(0f, 1f)));
    let _e66 = f;
    let _e72 = i_1;
    let _e77 = hash((_e72 + vec2<f32>(1f, 1f)));
    let _e78 = f;
    let _e84 = u;
    let _e87 = u;
    n_1 = mix(mix(dot(_e38, (_e39 - vec2<f32>(0f, 0f))), dot(_e50, (_e51 - vec2<f32>(1f, 0f))), _e57.x), mix(dot(_e65, (_e66 - vec2<f32>(0f, 1f))), dot(_e77, (_e78 - vec2<f32>(1f, 1f))), _e84.x), _e87.y);
    let _e91 = n_1;
    return ((_e91 * 0.5f) + 0.5f);
}

fn noiseLoop1d(n_2: f32, scale: f32, seed: f32) -> f32 {
    var n_3: f32;
    var scale_1: f32;
    var seed_1: f32;
    var x_16: f32;
    var y_2: f32;

    n_3 = n_2;
    scale_1 = scale;
    seed_1 = seed;
    let _e20 = n_3;
    let _e24 = seed_1;
    x_16 = (cos((_e20 * 6.2831855f)) + _e24);
    let _e27 = n_3;
    let _e31 = seed_1;
    y_2 = (sin((_e27 * 6.2831855f)) + _e31);
    let _e34 = x_16;
    let _e35 = y_2;
    let _e37 = scale_1;
    let _e39 = noise2dgrad((vec2<f32>(_e34, _e35) * _e37));
    return _e39;
}

fn rotate2D(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m_1: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e19 = theta_1;
    let _e21 = theta_1;
    let _e24 = theta_1;
    let _e26 = theta_1;
    m_1 = mat2x2<f32>(vec2<f32>(cos(_e19), -(sin(_e21))), vec2<f32>(sin(_e24), cos(_e26)));
    let _e32 = m_1;
    let _e33 = position_1;
    return (_e32 * _e33);
}

fn random1d(n_4: f32) -> f32 {
    var n_5: f32;

    n_5 = n_4;
    let _e17 = n_5;
    return fract((sin(_e17) * 43758.547f));
}

fn main_1() {
    var amount2_: f32;
    var col: vec4<f32>;
    var sUv: vec2<f32>;
    var offset: f32;
    var sum: vec4<f32> = vec4(0f);
    var count: f32 = 6f;
    var i_2: f32 = 0f;
    var nUv: vec2<f32>;
    var nScale: f32 = 4f;
    var timeFac: f32 = 3f;
    var period: vec3<f32>;
    var n_6: f32;

    let _e15 = global.amount;
    amount2_ = (_e15 * 0.6f);
    let _e19 = vUv_1;
    let _e20 = textureSample(tDiffuse_t, tDiffuse_s, _e19);
    col = _e20.xyzw;
    let _e23 = vUv_1;
    sUv = _e23;
    let _e25 = sUv;
    sUv = (_e25 - vec2(0.5f));
    let _e29 = sUv;
    let _e30 = global.angle;
    let _e32 = rotate2D(_e29, (_e30 * TWO_PI));
    sUv = _e32;
    let _e33 = sUv;
    sUv = (_e33 + vec2(0.5f));
    let _e37 = sUv;
    let _e39 = global.time;
    let _e42 = global.seed;
    let _e43 = random1d(_e42);
    let _e44 = noiseLoop1d((_e37.y + _e39), 1f, _e43);
    let _e47 = global.shift;
    offset = (((_e44 - 0.5f) * _e47) * 0.3f);
    let _e53 = sUv;
    let _e55 = offset;
    sUv.y = (_e53.y + _e55);
    loop {
        let _e64 = i_2;
        let _e65 = count;
        if !((_e64 < _e65)) {
            break;
        }
        {
            let _e71 = sum;
            let _e72 = i_2;
            let _e75 = count;
            let _e77 = sUv;
            let _e80 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(((_e72 * 1f) / _e75), _e77.y));
            sum = (_e71 + _e80.xyzw);
        }
        continuing {
            let _e68 = i_2;
            i_2 = (_e68 + 1f);
        }
    }
    let _e83 = vUv_1;
    nUv = _e83;
    let _e87 = nUv;
    nUv = (_e87 - vec2(0.5f));
    let _e91 = nUv;
    let _e92 = nScale;
    nUv = (_e91 * _e92);
    let _e94 = nUv;
    nUv = (_e94 + vec2(0.5f));
    let _e100 = nScale;
    let _e103 = nScale;
    let _e106 = timeFac;
    period = vec3<f32>((_e100 * 2f), (_e103 * 2f), _e106);
    let _e109 = nUv;
    let _e110 = global.time;
    let _e111 = timeFac;
    let _e116 = period;
    let _e117 = pnoise3d(vec3<f32>(_e109.x, _e109.y, (_e110 * _e111)), _e116);
    n_6 = ((_e117 / 2f) + 0.5f);
    let _e123 = n_6;
    n_6 = clamp(_e123, 0f, 1f);
    let _e127 = col;
    let _e128 = sum;
    let _e129 = amount2_;
    let _e131 = n_6;
    col = (_e127 + ((_e128 * _e129) * _e131));
    let _e134 = col;
    o_color = _e134;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e23 = o_color;
    return FragmentOutput(_e23);
}
