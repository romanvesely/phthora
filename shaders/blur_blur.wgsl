struct U {
    resolution: vec2<f32>,
    delta: vec2<f32>,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var source_t: texture_2d<f32>;
@group(0) @binding(2) 
var source_s: sampler;

fn blur9_(uv: vec2<f32>, resolution: vec2<f32>, delta: vec2<f32>) -> vec4<f32> {
    var uv_1: vec2<f32>;
    var resolution_1: vec2<f32>;
    var delta_1: vec2<f32>;
    var color: vec4<f32> = vec4(0f);
    var off1_: vec2<f32>;
    var off2_: vec2<f32>;

    uv_1 = uv;
    resolution_1 = resolution;
    delta_1 = delta;
    let _e19 = delta_1;
    off1_ = (vec2(1.3846154f) * _e19);
    let _e24 = delta_1;
    off2_ = (vec2(3.2307692f) * _e24);
    let _e27 = color;
    let _e28 = uv_1;
    let _e29 = textureSample(source_t, source_s, _e28);
    color = (_e27 + (_e29 * 0.22702703f));
    let _e33 = color;
    let _e34 = uv_1;
    let _e35 = off1_;
    let _e36 = resolution_1;
    let _e39 = textureSample(source_t, source_s, (_e34 + (_e35 / _e36)));
    color = (_e33 + (_e39 * 0.31621623f));
    let _e43 = color;
    let _e44 = uv_1;
    let _e45 = off1_;
    let _e46 = resolution_1;
    let _e49 = textureSample(source_t, source_s, (_e44 - (_e45 / _e46)));
    color = (_e43 + (_e49 * 0.31621623f));
    let _e53 = color;
    let _e54 = uv_1;
    let _e55 = off2_;
    let _e56 = resolution_1;
    let _e59 = textureSample(source_t, source_s, (_e54 + (_e55 / _e56)));
    color = (_e53 + (_e59 * 0.07027027f));
    let _e63 = color;
    let _e64 = uv_1;
    let _e65 = off2_;
    let _e66 = resolution_1;
    let _e69 = textureSample(source_t, source_s, (_e64 - (_e65 / _e66)));
    color = (_e63 + (_e69 * 0.07027027f));
    let _e73 = color;
    return _e73;
}

fn main_1() {
    var b: vec4<f32>;

    let _e8 = vUv_1;
    let _e9 = global.resolution;
    let _e10 = global.delta;
    let _e11 = blur9_(_e8, _e9, _e10);
    b = _e11;
    let _e13 = b;
    o_color = _e13;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e15 = o_color;
    return FragmentOutput(_e15);
}
