struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;
@group(0) @binding(3) 
var tPrevious_t: texture_2d<f32>;
@group(0) @binding(4) 
var tPrevious_s: sampler;

fn main_1() {
    var cur: vec3<f32>;
    var prev: vec3<f32>;
    var diff: f32;

    let _e6 = vUv_1;
    let _e7 = textureSample(tDiffuse_t, tDiffuse_s, _e6);
    cur = _e7.xyz;
    let _e10 = vUv_1;
    let _e11 = textureSample(tPrevious_t, tPrevious_s, _e10);
    prev = _e11.xyz;
    let _e14 = cur;
    let _e15 = prev;
    diff = length((_e14 - _e15));
    let _e19 = diff;
    o_color = vec4<f32>(_e19, 0f, 0f, 1f);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e15 = o_color;
    return FragmentOutput(_e15);
}
