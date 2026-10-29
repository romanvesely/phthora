struct U {
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

fn main_1() {
    var amount2_: f32;
    var base: vec4<f32>;
    var lumCoeff: vec3<f32> = vec3<f32>(0.25f, 0.65f, 0.1f);
    var lum: f32;
    var blend: vec3<f32>;
    var L: f32;
    var result1_: vec3<f32>;
    var result2_: vec3<f32>;
    var newColor: vec3<f32>;
    var A2_: f32;
    var mixRGB: vec3<f32>;

    let _e6 = global.amount;
    amount2_ = (_e6 * 1.6f);
    let _e10 = vUv_1;
    let _e11 = textureSample(tDiffuse_t, tDiffuse_s, _e10);
    base = _e11;
    let _e18 = lumCoeff;
    let _e19 = base;
    lum = dot(_e18, _e19.xyz);
    let _e23 = lum;
    blend = vec3(_e23);
    let _e29 = lum;
    L = min(1f, max(0f, (10f * (_e29 - 0.45f))));
    let _e37 = base;
    let _e40 = blend;
    result1_ = ((2f * _e37.xyz) * _e40);
    let _e46 = blend;
    let _e51 = base;
    result2_ = (vec3(1f) - ((2f * (vec3(1f) - _e46)) * (vec3(1f) - _e51.xyz)));
    let _e59 = result1_;
    let _e60 = result2_;
    let _e61 = L;
    newColor = mix(_e59, _e60, vec3(_e61));
    let _e65 = amount2_;
    let _e66 = base;
    A2_ = (_e65 * _e66.w);
    let _e70 = A2_;
    let _e71 = newColor;
    mixRGB = (_e70 * _e71.xyz);
    let _e75 = mixRGB;
    let _e77 = A2_;
    let _e79 = base;
    mixRGB = (_e75 + ((1f - _e77) * _e79.xyz));
    let _e83 = mixRGB;
    let _e84 = base;
    o_color = vec4<f32>(_e83.x, _e83.y, _e83.z, _e84.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e13 = o_color;
    return FragmentOutput(_e13);
}
