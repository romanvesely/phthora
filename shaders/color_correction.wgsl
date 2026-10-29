struct U {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    hue: f32,
    b_invert: f32,
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
    var brightness2_: f32;
    var contrast2_: f32;
    var average: f32;
    var sat2_: f32;
    var angle: f32;
    var s: f32;
    var c: f32;
    var weights: vec3<f32>;
    var len: f32;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    o_color = _e15;
    let _e19 = global.brightness;
    brightness2_ = mix(-1f, 1f, _e19);
    let _e22 = o_color;
    let _e24 = o_color;
    let _e26 = brightness2_;
    let _e28 = (_e24.xyz + vec3(_e26));
    o_color.x = _e28.x;
    o_color.y = _e28.y;
    o_color.z = _e28.z;
    let _e37 = global.contrast;
    contrast2_ = mix(0f, 0.9f, _e37);
    let _e40 = o_color;
    let _e42 = o_color;
    let _e48 = contrast2_;
    let _e54 = (((_e42.xyz - vec3(0.5f)) / vec3((1f - _e48))) + vec3(0.5f));
    o_color.x = _e54.x;
    o_color.y = _e54.y;
    o_color.z = _e54.z;
    let _e61 = o_color;
    let _e63 = o_color;
    let _e66 = o_color;
    average = (((_e61.x + _e63.y) + _e66.z) / 3f);
    let _e72 = global.saturation;
    sat2_ = ((_e72 - 0.5f) * 2f);
    let _e78 = sat2_;
    if (_e78 > 0f) {
        {
            let _e81 = o_color;
            let _e83 = o_color;
            let _e85 = average;
            let _e86 = o_color;
            let _e93 = sat2_;
            let _e98 = (_e83.xyz + ((vec3(_e85) - _e86.xyz) * (1f - (1f / (1.001f - _e93)))));
            o_color.x = _e98.x;
            o_color.y = _e98.y;
            o_color.z = _e98.z;
        }
    } else {
        {
            let _e105 = o_color;
            let _e107 = o_color;
            let _e109 = average;
            let _e110 = o_color;
            let _e114 = sat2_;
            let _e117 = (_e107.xyz + ((vec3(_e109) - _e110.xyz) * -(_e114)));
            o_color.x = _e117.x;
            o_color.y = _e117.y;
            o_color.z = _e117.z;
        }
    }
    let _e124 = global.hue;
    angle = (_e124 * 6.2831855f);
    let _e128 = angle;
    s = sin(_e128);
    let _e131 = angle;
    c = cos(_e131);
    let _e135 = c;
    let _e140 = s;
    let _e142 = c;
    let _e146 = s;
    let _e148 = c;
    weights = ((vec3<f32>((2f * _e135), ((-1.7320508f * _e140) - _e142), ((1.7320508f * _e146) - _e148)) + vec3(1f)) / vec3(3f));
    let _e158 = o_color;
    len = length(_e158.xyz);
    let _e162 = o_color;
    let _e164 = o_color;
    let _e166 = weights;
    let _e169 = o_color;
    let _e171 = weights;
    let _e174 = o_color;
    let _e176 = weights;
    let _e179 = vec3<f32>(dot(_e164.xyz, _e166.xyz), dot(_e169.xyz, _e171.zxy), dot(_e174.xyz, _e176.yzx));
    o_color.x = _e179.x;
    o_color.y = _e179.y;
    o_color.z = _e179.z;
    let _e186 = global.b_invert;
    if (_e186 > 0.5f) {
        let _e189 = o_color;
        let _e192 = o_color;
        let _e195 = (vec3(1f) - _e192.xyz);
        o_color.x = _e195.x;
        o_color.y = _e195.y;
        o_color.z = _e195.z;
        return;
    } else {
        return;
    }
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
