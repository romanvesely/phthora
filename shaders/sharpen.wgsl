struct U {
    time: f32,
    speed: f32,
    amount: f32,
    spread: f32,
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
var<private> gl_FragCoord_1: vec4<f32>;

fn texSample(x: i32, y: i32, fragCoord: vec2<f32>) -> vec3<f32> {
    var fragCoord_1: vec2<f32>;
    var x2_: f32;
    var y2_: f32;
    var uv: vec2<f32>;

    fragCoord_1 = fragCoord;
    let _e19 = global.spread;
    x2_ = ((f32(x) * _e19) * 4f);
    let _e25 = global.spread;
    y2_ = ((f32(y) * _e25) * 4f);
    let _e30 = fragCoord_1;
    let _e31 = x2_;
    let _e32 = y2_;
    let _e35 = global.resolution;
    uv = ((_e30 + vec2<f32>(_e31, _e32)) / _e35);
    let _e38 = uv;
    let _e39 = textureSample(tDiffuse_t, tDiffuse_s, _e38);
    return _e39.xyz;
}

fn sharpenFilter(fragCoord_2: vec2<f32>, strength: f32) -> vec3<f32> {
    var fragCoord_3: vec2<f32>;
    var strength_1: f32;
    var f: vec3<f32>;

    fragCoord_3 = fragCoord_2;
    strength_1 = strength;
    let _e22 = fragCoord_3;
    let _e23 = texSample(-1i, -1i, _e22);
    let _e30 = fragCoord_3;
    let _e31 = texSample(0i, -1i, _e30);
    let _e39 = fragCoord_3;
    let _e40 = texSample(1i, -1i, _e39);
    let _e48 = fragCoord_3;
    let _e49 = texSample(-1i, 0i, _e48);
    let _e56 = fragCoord_3;
    let _e57 = texSample(0i, 0i, _e56);
    let _e63 = fragCoord_3;
    let _e64 = texSample(1i, 0i, _e63);
    let _e72 = fragCoord_3;
    let _e73 = texSample(-1i, 1i, _e72);
    let _e80 = fragCoord_3;
    let _e81 = texSample(0i, 1i, _e80);
    let _e88 = fragCoord_3;
    let _e89 = texSample(1i, 1i, _e88);
    f = (((((((((_e23 * -1f) + (_e31 * -1f)) + (_e40 * -1f)) + (_e49 * -1f)) + (_e57 * 9f)) + (_e64 * -1f)) + (_e73 * -1f)) + (_e81 * -1f)) + (_e89 * -1f));
    let _e97 = fragCoord_3;
    let _e98 = texSample(0i, 0i, _e97);
    let _e99 = f;
    let _e100 = strength_1;
    return mix(_e98, _e99, vec3(_e100));
}

fn main_1() {
    var orig: vec4<f32>;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    orig = _e15;
    let _e18 = gl_FragCoord_1;
    let _e20 = global.amount;
    let _e23 = sharpenFilter(_e18.xy, (_e20 * 4f));
    let _e24 = orig;
    o_color = vec4<f32>(_e23.x, _e23.y, _e23.z, _e24.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>, @builtin(position) gl_FragCoord: vec4<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    gl_FragCoord_1 = gl_FragCoord;
    main_1();
    let _e23 = o_color;
    return FragmentOutput(_e23);
}
