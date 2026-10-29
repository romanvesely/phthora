struct U {
    keyStrength: f32,
    blackBoost: f32,
    b_despill: f32,
    chromaCol: vec3<f32>,
    lowerRange: f32,
    threshold: f32,
    b_showMask: f32,
    maskFromIndex: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tOrig_t: texture_2d<f32>;
@group(0) @binding(2) 
var tOrig_s: sampler;
@group(0) @binding(3) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(4) 
var tDiffuse_s: sampler;
@group(0) @binding(5) 
var tSource_t: texture_2d<f32>;
@group(0) @binding(6) 
var tSource_s: sampler;
var<private> RGBtoYUV: mat4x4<f32> = mat4x4<f32>(vec4<f32>(0.257f, 0.439f, -0.148f, 0f), vec4<f32>(0.504f, -0.368f, -0.291f, 0f), vec4<f32>(0.098f, -0.071f, 0.439f, 0f), vec4<f32>(0.0625f, 0.5f, 0.5f, 1f));

fn colorclose(yuv: vec3<f32>, keyYuv: vec3<f32>, tol: vec2<f32>) -> f32 {
    var yuv_1: vec3<f32>;
    var keyYuv_1: vec3<f32>;
    var tol_1: vec2<f32>;
    var tmp: f32;

    yuv_1 = yuv;
    keyYuv_1 = keyYuv;
    tol_1 = tol;
    let _e25 = keyYuv_1;
    let _e27 = yuv_1;
    let _e32 = keyYuv_1;
    let _e34 = yuv_1;
    tmp = sqrt((pow((_e25.y - _e27.y), 2f) + pow((_e32.z - _e34.z), 2f)));
    let _e42 = tmp;
    let _e43 = tol_1;
    if (_e42 < _e43.x) {
        return 0f;
    } else {
        let _e47 = tmp;
        let _e48 = tol_1;
        if (_e47 < _e48.y) {
            let _e51 = tmp;
            let _e52 = tol_1;
            let _e55 = tol_1;
            let _e57 = tol_1;
            return ((_e51 - _e52.x) / (_e55.y - _e57.x));
        } else {
            return 1f;
        }
    }
}

fn main_1() {
    var colUnder: vec4<f32>;
    var colOver: vec4<f32>;
    var colKey: vec4<f32>;
    var lowerRange: f32 = 0.005f;
    var chromaKey: vec4<f32>;
    var keyYUV: vec4<f32>;
    var yuv_2: vec4<f32>;
    var mask: f32;

    let _e19 = vUv_1;
    let _e20 = textureSample(tOrig_t, tOrig_s, _e19);
    colUnder = _e20;
    let _e22 = vUv_1;
    let _e23 = textureSample(tDiffuse_t, tDiffuse_s, _e22);
    colOver = _e23;
    // the key colour is searched in the effect input or in the original source
    let colSource = textureSample(tSource_t, tSource_s, _e22);
    colKey = select(_e23, colSource, global.maskFromIndex > 0.5f);
    let _e27 = global.chromaCol;
    chromaKey = vec4<f32>(_e27.x, _e27.y, _e27.z, 1f);
    let _e34 = RGBtoYUV;
    let _e35 = chromaKey;
    keyYUV = (_e34 * _e35);
    let _e38 = RGBtoYUV;
    let _e39 = colKey;
    yuv_2 = (_e38 * _e39);
    let _e43 = yuv_2;
    let _e45 = keyYUV;
    let _e47 = lowerRange;
    let _e48 = global.threshold;
    let _e50 = colorclose(_e43.xyz, _e45.xyz, vec2<f32>(_e47, _e48));
    mask = (1f - _e50);
    if (global.b_showMask > 0.5f) {
        o_color = vec4<f32>(vec3(mask), 1f);
        return;
    }
    let _e53 = colOver;
    let _e54 = mask;
    let _e55 = chromaKey;
    let _e61 = colUnder;
    let _e62 = mask;
    o_color = (max((_e53 - (_e54 * _e55)), vec4(0f)) + (_e61 * _e62));
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e54 = o_color;
    return FragmentOutput(_e54);
}
