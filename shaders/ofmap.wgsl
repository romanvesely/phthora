struct U {
    resolution: vec2<f32>,
    offset: f32,
    lambda: f32,
    distortion: f32,
    persistence: f32,
    speed: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const coeffs: vec4<f32> = vec4<f32>(0.2126f, 0.7152f, 0.0722f, 0f);

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
@group(0) @binding(5) 
var tMapPrevious_t: texture_2d<f32>;
@group(0) @binding(6) 
var tMapPrevious_s: sampler;

fn gray(n: vec4<f32>) -> f32 {
    var n_1: vec4<f32>;

    n_1 = n;
    let _e23 = n_1;
    let _e25 = n_1;
    let _e28 = n_1;
    return (((_e23.x + _e25.y) + _e28.z) / 3f);
}

fn main_1() {
    var uv: vec2<f32>;
    var a: vec4<f32>;
    var brightness: f32;
    var b: vec4<f32>;
    var curdif: vec4<f32>;
    var off: vec2<f32>;
    var gradX: vec4<f32>;
    var gradY: vec4<f32>;
    var gradMag: vec4<f32>;
    var vx: vec4<f32>;
    var vxd: f32;
    var xout: vec2<f32>;
    var vy: vec4<f32>;
    var vyd: f32;
    var yout: vec2<f32>;
    var map: vec4<f32>;
    var d: vec2<f32>;
    var left_coord: vec2<f32>;
    var right_coord: vec2<f32>;
    var above_coord: vec2<f32>;
    var below_coord: vec2<f32>;
    var lefta_coord: vec2<f32>;
    var righta_coord: vec2<f32>;
    var leftb_coord: vec2<f32>;
    var rightb_coord: vec2<f32>;
    var color: vec4<f32>;
    var colorL: vec4<f32>;
    var colorR: vec4<f32>;
    var colorA: vec4<f32>;
    var colorB: vec4<f32>;
    var colorLA: vec4<f32>;
    var colorRA: vec4<f32>;
    var colorLB: vec4<f32>;
    var colorRB: vec4<f32>;
    var blurVector: vec4<f32>;

    let _e21 = vUv_1;
    uv = _e21;
    let _e23 = uv;
    let _e24 = textureSample(tDiffuse_t, tDiffuse_s, _e23);
    a = (_e24 * coeffs);
    let _e27 = a;
    let _e28 = gray(_e27);
    brightness = _e28;
    let _e30 = brightness;
    a = vec4(_e30);
    let _e32 = uv;
    let _e33 = textureSample(tPrevious_t, tPrevious_s, _e32);
    b = (_e33 * coeffs);
    let _e36 = b;
    let _e37 = gray(_e36);
    brightness = _e37;
    let _e38 = brightness;
    b = vec4(_e38);
    let _e40 = b;
    let _e41 = a;
    curdif = (_e40 - _e41);
    let _e44 = global.offset;
    off = vec2<f32>(_e44, 0f);
    let _e48 = uv;
    let _e49 = off;
    let _e52 = textureSample(tDiffuse_t, tDiffuse_s, (_e48 + _e49.xy));
    let _e53 = uv;
    let _e54 = off;
    let _e57 = textureSample(tDiffuse_t, tDiffuse_s, (_e53 - _e54.xy));
    gradX = (_e52 - _e57);
    let _e60 = gradX;
    let _e61 = uv;
    let _e62 = off;
    let _e65 = textureSample(tPrevious_t, tPrevious_s, (_e61 + _e62.xy));
    let _e66 = uv;
    let _e67 = off;
    let _e70 = textureSample(tPrevious_t, tPrevious_s, (_e66 - _e67.xy));
    gradX = (_e60 + (_e65 - _e70));
    let _e73 = uv;
    let _e74 = off;
    let _e77 = textureSample(tDiffuse_t, tDiffuse_s, (_e73 + _e74.yx));
    let _e78 = uv;
    let _e79 = off;
    let _e82 = textureSample(tDiffuse_t, tDiffuse_s, (_e78 - _e79.yx));
    gradY = (_e77 - _e82);
    let _e85 = gradY;
    let _e86 = uv;
    let _e87 = off;
    let _e90 = textureSample(tPrevious_t, tPrevious_s, (_e86 + _e87.yx));
    let _e91 = uv;
    let _e92 = off;
    let _e95 = textureSample(tPrevious_t, tPrevious_s, (_e91 - _e92.yx));
    gradY = (_e85 + (_e90 - _e95));
    let _e98 = gradX;
    let _e99 = gradX;
    let _e101 = gradY;
    let _e102 = gradY;
    let _e105 = global.lambda;
    gradMag = sqrt((((_e98 * _e99) + (_e101 * _e102)) + vec4(_e105)));
    let _e110 = curdif;
    let _e111 = gradX;
    let _e112 = gradMag;
    vx = (_e110 * (_e111 / _e112));
    let _e116 = vx;
    let _e117 = gray(_e116);
    vxd = _e117;
    let _e119 = vxd;
    let _e122 = vxd;
    let _e127 = global.distortion;
    xout = ((vec2<f32>(max(_e119, 0f), abs(min(_e122, 0f))) * _e127) * 5f);
    let _e132 = curdif;
    let _e133 = gradY;
    let _e134 = gradMag;
    vy = (_e132 * (_e133 / _e134));
    let _e138 = vy;
    let _e139 = gray(_e138);
    vyd = _e139;
    let _e141 = vyd;
    let _e144 = vyd;
    let _e149 = global.distortion;
    yout = ((vec2<f32>(max(_e141, 0f), abs(min(_e144, 0f))) * _e149) * 5f);
    let _e154 = xout;
    let _e155 = _e154.xy;
    let _e156 = yout;
    let _e157 = _e156.xy;
    map = clamp(vec4<f32>(_e155.x, _e155.y, _e157.x, _e157.y), vec4(0f), vec4(1f));
    let _e169 = uv;
    uv = (_e169 - vec2(0.5f));
    let _e173 = uv;
    let _e175 = global.speed;
    uv = (_e173 * (1f - (_e175 * 0.04f)));
    let _e180 = uv;
    uv = (_e180 + vec2(0.5f));
    let _e185 = global.resolution;
    d = (vec2(4f) / _e185);
    let _e189 = uv;
    let _e190 = d;
    left_coord = clamp(vec2<f32>((_e189 + vec2<f32>(-(_e190.x), 0f))), vec2(0f), vec2(1f));
    let _e204 = uv;
    let _e205 = d;
    right_coord = clamp(vec2<f32>((_e204 + vec2<f32>(_e205.x, 0f))), vec2(0f), vec2(1f));
    let _e218 = uv;
    let _e220 = d;
    above_coord = clamp(vec2<f32>((_e218 + vec2<f32>(0f, _e220.y))), vec2(0f), vec2(1f));
    let _e232 = uv;
    let _e234 = d;
    below_coord = clamp(vec2<f32>((_e232 + vec2<f32>(0f, -(_e234.y)))), vec2(0f), vec2(1f));
    let _e247 = uv;
    let _e248 = d;
    let _e251 = d;
    lefta_coord = clamp(vec2<f32>((_e247 + vec2<f32>(-(_e248.x), _e251.x))), vec2(0f), vec2(1f));
    let _e262 = uv;
    let _e263 = d;
    let _e265 = d;
    righta_coord = clamp(vec2<f32>((_e262 + vec2<f32>(_e263.x, _e265.x))), vec2(0f), vec2(1f));
    let _e276 = uv;
    let _e277 = d;
    let _e280 = d;
    leftb_coord = clamp(vec2<f32>((_e276 + vec2<f32>(-(_e277.x), -(_e280.x)))), vec2(0f), vec2(1f));
    let _e292 = uv;
    let _e293 = d;
    let _e295 = d;
    rightb_coord = clamp(vec2<f32>((_e292 + vec2<f32>(_e293.x, -(_e295.x)))), vec2(0f), vec2(1f));
    let _e307 = uv;
    let _e308 = textureSample(tMapPrevious_t, tMapPrevious_s, _e307);
    color = _e308;
    let _e310 = left_coord;
    let _e311 = textureSample(tMapPrevious_t, tMapPrevious_s, _e310);
    colorL = _e311;
    let _e313 = right_coord;
    let _e314 = textureSample(tMapPrevious_t, tMapPrevious_s, _e313);
    colorR = _e314;
    let _e316 = above_coord;
    let _e317 = textureSample(tMapPrevious_t, tMapPrevious_s, _e316);
    colorA = _e317;
    let _e319 = below_coord;
    let _e320 = textureSample(tMapPrevious_t, tMapPrevious_s, _e319);
    colorB = _e320;
    let _e322 = lefta_coord;
    let _e323 = textureSample(tMapPrevious_t, tMapPrevious_s, _e322);
    colorLA = _e323;
    let _e325 = righta_coord;
    let _e326 = textureSample(tMapPrevious_t, tMapPrevious_s, _e325);
    colorRA = _e326;
    let _e328 = leftb_coord;
    let _e329 = textureSample(tMapPrevious_t, tMapPrevious_s, _e328);
    colorLB = _e329;
    let _e331 = rightb_coord;
    let _e332 = textureSample(tMapPrevious_t, tMapPrevious_s, _e331);
    colorRB = _e332;
    let _e334 = color;
    let _e335 = colorL;
    let _e337 = colorR;
    let _e339 = colorA;
    let _e341 = colorB;
    let _e343 = colorLA;
    let _e345 = colorRA;
    let _e347 = colorLB;
    let _e349 = colorRB;
    blurVector = (((((((((_e334 + _e335) + _e337) + _e339) + _e341) + _e343) + _e345) + _e347) + _e349) / vec4(9f));
    let _e355 = map;
    let _e356 = global.persistence;
    let _e357 = blurVector;
    o_color = (_e355 + (_e356 * _e357));
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e33 = o_color;
    return FragmentOutput(_e33);
}
