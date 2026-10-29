struct U {
    resolution: vec2<f32>,
    scale: f32,
    angle: f32,
    amount: f32,
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

fn luma(color: vec3<f32>) -> f32 {
    var color_1: vec3<f32>;

    color_1 = color;
    let _e14 = color_1;
    return dot(_e14, vec3<f32>(0.299f, 0.587f, 0.114f));
}

fn main_1() {
    var orig: vec4<f32>;
    var center: vec2<f32> = vec2(0.5f);
    var radius: f32 = 0.5f;
    var d: vec2<f32>;
    var r: f32;
    var a: f32;
    var uvt: vec2<f32>;
    var c: f32;
    var l: f32;
    var f: f32;
    var color_2: vec4<f32>;

    let _e12 = vUv_1;
    let _e13 = textureSample(tDiffuse_t, tDiffuse_s, _e12);
    orig = _e13;
    let _e20 = vUv_1;
    let _e21 = center;
    d = (_e20 - _e21);
    let _e24 = d;
    let _e26 = global.resolution;
    let _e28 = global.resolution;
    let _e34 = global.scale;
    r = (length((_e24 * vec2<f32>(1f, (_e26.y / _e28.x)))) * _e34);
    let _e37 = d;
    let _e39 = d;
    let _e42 = radius;
    let _e43 = r;
    let _e45 = radius;
    let _e48 = global.angle;
    a = ((atan2(_e37.y, _e39.x) + ((_e42 - _e43) / _e45)) + (_e48 * 3.1415927f));
    let _e53 = center;
    let _e54 = r;
    let _e55 = a;
    let _e57 = a;
    uvt = (_e53 + (_e54 * vec2<f32>(cos(_e55), sin(_e57))));
    let _e65 = uvt;
    c = (0.75f + (0.25f * sin((_e65.x * 1000f))));
    let _e73 = orig;
    let _e75 = luma(_e73.xyz);
    l = _e75;
    let _e78 = c;
    let _e80 = c;
    let _e81 = l;
    f = smoothstep((0.5f * _e78), _e80, _e81);
    let _e86 = f;
    f = smoothstep(0f, 0.5f, _e86);
    let _e88 = f;
    let _e89 = f;
    let _e90 = f;
    let _e91 = orig;
    color_2 = vec4<f32>(_e88, _e89, _e90, _e91.w);
    let _e95 = orig;
    let _e96 = color_2;
    let _e97 = global.amount;
    color_2 = mix(_e95, _e96, vec4(_e97));
    let _e100 = color_2;
    o_color = _e100;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
