struct U {
    amount: f32,
    scale: f32,
    curve: f32,
    vignette: f32,
    brightness: f32,
    resolution: vec2<f32>,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const SHADOWMASK_VERTGAPWIDTH: f32 = 0.02f;
const SHADOWMASK_VERTHARDNESS: f32 = 0.1f;
const SHADOWMASK_HORIZGAPWIDTH: f32 = -1f;
const SHADOWMASK_HORIZARDNESS: f32 = 0.8f;
const SHADOWMASK_RCOL_OFFSET: f32 = 0f;
const SHADOWMASK_GCOL_OFFSET: f32 = -0.3f;
const SHADOWMASK_BCOL_OFFSET: f32 = -0.6f;
const SHADOWMASK_UV_SCALE: f32 = 0.1f;
const SINE_SCALE: f32 = 6.28318f;

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;

fn Grille(x: f32, offset: f32, multiplier: f32) -> f32 {
    var x_1: f32;
    var offset_1: f32;
    var multiplier_1: f32;

    x_1 = x;
    offset_1 = offset;
    multiplier_1 = multiplier;
    let _e33 = x_1;
    let _e36 = multiplier_1;
    let _e38 = offset_1;
    return smoothstep(0f, 1f, ((sin((_e33 * SINE_SCALE)) * _e36) + _e38));
}

fn ShadowMaskRows(uv: vec2<f32>) -> f32 {
    var uv_1: vec2<f32>;

    uv_1 = uv;
    let _e28 = uv_1;
    uv_1.x = (_e28.x * 0.5f);
    let _e33 = uv_1;
    let _e35 = uv_1;
    uv_1.x = (_e33.x - floor((_e35.x + 0.5f)));
    let _e41 = uv_1;
    if (_e41.x < 0f) {
        let _e46 = uv_1;
        uv_1.y = (_e46.y + 0.5f);
    }
    let _e50 = uv_1;
    let _e54 = Grille(_e50.y, 1f, SHADOWMASK_HORIZARDNESS);
    return _e54;
}

fn ShadowMaskSingleCol(x_2: f32) -> f32 {
    var x_3: f32;

    x_3 = x_2;
    let _e27 = x_3;
    let _e30 = Grille(_e27, -0.02f, SHADOWMASK_VERTHARDNESS);
    return _e30;
}

fn ShadowMaskRGBCols(x_4: f32) -> vec3<f32> {
    var x_5: f32;

    x_5 = x_4;
    let _e27 = x_5;
    let _e29 = ShadowMaskSingleCol((_e27 + SHADOWMASK_RCOL_OFFSET));
    let _e30 = x_5;
    let _e32 = ShadowMaskSingleCol((_e30 + SHADOWMASK_GCOL_OFFSET));
    let _e33 = x_5;
    let _e35 = ShadowMaskSingleCol((_e33 + SHADOWMASK_BCOL_OFFSET));
    return vec3<f32>(_e29, _e32, _e35);
}

fn ShadowMask(uv_2: vec2<f32>) -> vec3<f32> {
    var uv_3: vec2<f32>;

    uv_3 = uv_2;
    let _e27 = uv_3;
    let _e29 = ShadowMaskRGBCols(_e27.x);
    let _e30 = uv_3;
    let _e31 = ShadowMaskRows(_e30);
    return (_e29 * _e31);
}

fn curveUV(uv_4: vec2<f32>, bend: f32) -> vec2<f32> {
    var uv_5: vec2<f32>;
    var bend_1: f32;

    uv_5 = uv_4;
    bend_1 = bend;
    let _e29 = uv_5;
    uv_5 = ((_e29 - vec2(0.5f)) * 2f);
    let _e36 = uv_5;
    let _e39 = uv_5;
    let _e42 = bend_1;
    uv_5.x = (_e36.x * (1f + pow((abs(_e39.y) / _e42), 2f)));
    let _e49 = uv_5;
    let _e52 = uv_5;
    let _e55 = bend_1;
    uv_5.y = (_e49.y * (1f + pow((((abs(_e52.x) / _e55) * 4f) / 5f), 2f)));
    let _e65 = uv_5;
    uv_5 = ((_e65 / vec2(2f)) + vec2(0.5f));
    let _e72 = uv_5;
    return _e72;
}

fn sdBox(p: vec2<f32>, b: vec2<f32>) -> f32 {
    var p_1: vec2<f32>;
    var b_1: vec2<f32>;
    var d: vec2<f32>;

    p_1 = p;
    b_1 = b;
    let _e29 = p_1;
    let _e31 = b_1;
    d = (abs(_e29) - _e31);
    let _e34 = d;
    let _e39 = d;
    let _e41 = d;
    return (length(max(_e34, vec2(0f))) + min(max(_e39.x, _e41.y), 0f));
}

fn main_1() {
    var uv_6: vec2<f32>;
    var scale2_: f32;
    var curve2_: f32;
    var brightness2_: f32;
    var sampleUV: vec2<f32>;
    var orig: vec4<f32>;
    var color: vec4<f32>;
    var width: f32;
    var d_1: f32;
    var vigOuter: f32;
    var shadowMaskUV: vec2<f32>;

    let _e25 = vUv_1;
    uv_6 = _e25;
    let _e29 = global.scale;
    scale2_ = mix(0.2f, 0.02f, _e29);
    let _e34 = global.curve;
    curve2_ = mix(6f, 1.6f, _e34);
    let _e39 = global.brightness;
    brightness2_ = mix(50f, 300f, _e39);
    let _e42 = uv_6;
    let _e43 = curve2_;
    let _e44 = curveUV(_e42, _e43);
    sampleUV = _e44;
    let _e46 = uv_6;
    let _e47 = textureSample(tDiffuse_t, tDiffuse_s, _e46);
    orig = _e47;
    let _e49 = sampleUV;
    let _e50 = textureSample(tDiffuse_t, tDiffuse_s, _e49);
    color = _e50;
    let _e52 = color;
    let _e54 = color;
    let _e58 = pow(_e54.xyz, vec3(2.2f));
    color.x = _e58.x;
    color.y = _e58.y;
    color.z = _e58.z;
    let _e65 = color;
    let _e67 = color;
    let _e70 = (_e67.xyz * 2.2f);
    color.x = _e70.x;
    color.y = _e70.y;
    color.z = _e70.z;
    let _e78 = global.vignette;
    width = (0.55f - (_e78 * 0.17f));
    let _e83 = sampleUV;
    let _e87 = width;
    let _e89 = sdBox((_e83 - vec2(0.5f)), vec2(_e87));
    d_1 = _e89;
    let _e94 = d_1;
    vigOuter = (1f - smoothstep(0f, 0.1f, _e94));
    let _e98 = color;
    let _e100 = color;
    let _e102 = vigOuter;
    let _e103 = (_e100.xyz * _e102);
    color.x = _e103.x;
    color.y = _e103.y;
    color.z = _e103.z;
    let _e110 = sampleUV;
    let _e111 = global.resolution;
    let _e113 = scale2_;
    shadowMaskUV = ((_e110 * _e111) * _e113);
    let _e116 = color;
    let _e118 = color;
    let _e120 = shadowMaskUV;
    let _e121 = ShadowMask(_e120);
    let _e122 = brightness2_;
    let _e124 = (_e118.xyz * (_e121 * _e122));
    color.x = _e124.x;
    color.y = _e124.y;
    color.z = _e124.z;
    let _e131 = color;
    let _e133 = color;
    let _e139 = pow(_e133.xyz, vec3(0.45454544f));
    color.x = _e139.x;
    color.y = _e139.y;
    color.z = _e139.z;
    let _e146 = orig;
    let _e147 = color;
    let _e148 = global.amount;
    o_color = mix(_e146, _e147, vec4(_e148));
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e41 = o_color;
    return FragmentOutput(_e41);
}
