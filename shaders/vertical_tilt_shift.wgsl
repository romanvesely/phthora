struct U {
    v: f32,
    r: f32,
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
    var sum: vec4<f32> = vec4(0f);
    var vv: f32;

    let _e11 = global.v;
    let _e14 = global.r;
    let _e15 = vUv_1;
    vv = ((_e11 * 0.02f) * abs((_e14 - _e15.y)));
    let _e21 = sum;
    let _e22 = vUv_1;
    let _e24 = vUv_1;
    let _e27 = vv;
    let _e31 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e22.x, (_e24.y - (4f * _e27))));
    sum = (_e21 + (_e31 * 0.051f));
    let _e35 = sum;
    let _e36 = vUv_1;
    let _e38 = vUv_1;
    let _e41 = vv;
    let _e45 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e36.x, (_e38.y - (3f * _e41))));
    sum = (_e35 + (_e45 * 0.0918f));
    let _e49 = sum;
    let _e50 = vUv_1;
    let _e52 = vUv_1;
    let _e55 = vv;
    let _e59 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e50.x, (_e52.y - (2f * _e55))));
    sum = (_e49 + (_e59 * 0.12245f));
    let _e63 = sum;
    let _e64 = vUv_1;
    let _e66 = vUv_1;
    let _e69 = vv;
    let _e73 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e64.x, (_e66.y - (1f * _e69))));
    sum = (_e63 + (_e73 * 0.1531f));
    let _e77 = sum;
    let _e78 = vUv_1;
    let _e80 = vUv_1;
    let _e83 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e78.x, _e80.y));
    sum = (_e77 + (_e83 * 0.1633f));
    let _e87 = sum;
    let _e88 = vUv_1;
    let _e90 = vUv_1;
    let _e93 = vv;
    let _e97 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e88.x, (_e90.y + (1f * _e93))));
    sum = (_e87 + (_e97 * 0.1531f));
    let _e101 = sum;
    let _e102 = vUv_1;
    let _e104 = vUv_1;
    let _e107 = vv;
    let _e111 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e102.x, (_e104.y + (2f * _e107))));
    sum = (_e101 + (_e111 * 0.12245f));
    let _e115 = sum;
    let _e116 = vUv_1;
    let _e118 = vUv_1;
    let _e121 = vv;
    let _e125 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e116.x, (_e118.y + (3f * _e121))));
    sum = (_e115 + (_e125 * 0.0918f));
    let _e129 = sum;
    let _e130 = vUv_1;
    let _e132 = vUv_1;
    let _e135 = vv;
    let _e139 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e130.x, (_e132.y + (4f * _e135))));
    sum = (_e129 + (_e139 * 0.051f));
    let _e143 = sum;
    o_color = _e143;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e15 = o_color;
    return FragmentOutput(_e15);
}
