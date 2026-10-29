struct U {
    timeOpacity: f32,
    staticOpacity: f32,
    time: f32,
    bars: f32,
    seed: f32,
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
@group(0) @binding(3) 
var tTime_t: texture_2d<f32>;
@group(0) @binding(4) 
var tTime_s: sampler;
@group(0) @binding(5) 
var tStatic_t: texture_2d<f32>;
@group(0) @binding(6) 
var tStatic_s: sampler;

fn blendNormal(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    var base_1: vec3<f32>;
    var blend_1: vec3<f32>;

    base_1 = base;
    blend_1 = blend;
    let _e22 = blend_1;
    return _e22;
}

fn blendNormal_1(base_2: vec3<f32>, blend_2: vec3<f32>, opacity: f32) -> vec3<f32> {
    var base_3: vec3<f32>;
    var blend_3: vec3<f32>;
    var opacity_1: f32;

    base_3 = base_2;
    blend_3 = blend_2;
    opacity_1 = opacity;
    let _e24 = base_3;
    let _e25 = blend_3;
    let _e26 = blendNormal(_e24, _e25);
    let _e27 = opacity_1;
    let _e29 = base_3;
    let _e31 = opacity_1;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn blendMultiply(base_4: vec3<f32>, blend_4: vec3<f32>) -> vec3<f32> {
    var base_5: vec3<f32>;
    var blend_5: vec3<f32>;

    base_5 = base_4;
    blend_5 = blend_4;
    let _e22 = base_5;
    let _e23 = blend_5;
    return (_e22 * _e23);
}

fn blendMultiply_1(base_6: vec3<f32>, blend_6: vec3<f32>, opacity_2: f32) -> vec3<f32> {
    var base_7: vec3<f32>;
    var blend_7: vec3<f32>;
    var opacity_3: f32;

    base_7 = base_6;
    blend_7 = blend_6;
    opacity_3 = opacity_2;
    let _e24 = base_7;
    let _e25 = blend_7;
    let _e26 = blendMultiply(_e24, _e25);
    let _e27 = opacity_3;
    let _e29 = base_7;
    let _e31 = opacity_3;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn blendAdd(base_8: vec3<f32>, blend_8: vec3<f32>) -> vec3<f32> {
    var base_9: vec3<f32>;
    var blend_9: vec3<f32>;

    base_9 = base_8;
    blend_9 = blend_8;
    let _e22 = base_9;
    let _e23 = blend_9;
    return min((_e22 + _e23), vec3(1f));
}

fn blendAdd_1(base_10: vec3<f32>, blend_10: vec3<f32>, opacity_4: f32) -> vec3<f32> {
    var base_11: vec3<f32>;
    var blend_11: vec3<f32>;
    var opacity_5: f32;

    base_11 = base_10;
    blend_11 = blend_10;
    opacity_5 = opacity_4;
    let _e24 = base_11;
    let _e25 = blend_11;
    let _e26 = blendAdd(_e24, _e25);
    let _e27 = opacity_5;
    let _e29 = base_11;
    let _e31 = opacity_5;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn blendScreen(base_12: f32, blend_12: f32) -> f32 {
    var base_13: f32;
    var blend_13: f32;

    base_13 = base_12;
    blend_13 = blend_12;
    let _e24 = base_13;
    let _e27 = blend_13;
    return (1f - ((1f - _e24) * (1f - _e27)));
}

fn blendScreen_1(base_14: vec3<f32>, blend_14: vec3<f32>) -> vec3<f32> {
    var base_15: vec3<f32>;
    var blend_15: vec3<f32>;

    base_15 = base_14;
    blend_15 = blend_14;
    let _e22 = base_15;
    let _e24 = blend_15;
    let _e26 = blendScreen(_e22.x, _e24.x);
    let _e27 = base_15;
    let _e29 = blend_15;
    let _e31 = blendScreen(_e27.y, _e29.y);
    let _e32 = base_15;
    let _e34 = blend_15;
    let _e36 = blendScreen(_e32.z, _e34.z);
    return vec3<f32>(_e26, _e31, _e36);
}

fn blendScreen_2(base_16: vec3<f32>, blend_16: vec3<f32>, opacity_6: f32) -> vec3<f32> {
    var base_17: vec3<f32>;
    var blend_17: vec3<f32>;
    var opacity_7: f32;

    base_17 = base_16;
    blend_17 = blend_16;
    opacity_7 = opacity_6;
    let _e24 = base_17;
    let _e25 = blend_17;
    let _e26 = blendScreen_1(_e24, _e25);
    let _e27 = opacity_7;
    let _e29 = base_17;
    let _e31 = opacity_7;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn blendSoftLight(base_18: f32, blend_18: f32) -> f32 {
    var base_19: f32;
    var blend_19: f32;
    var local: f32;

    base_19 = base_18;
    blend_19 = blend_18;
    let _e22 = blend_19;
    if (_e22 < 0.5f) {
        let _e26 = base_19;
        let _e28 = blend_19;
        let _e30 = base_19;
        let _e31 = base_19;
        let _e35 = blend_19;
        local = (((2f * _e26) * _e28) + ((_e30 * _e31) * (1f - (2f * _e35))));
    } else {
        let _e40 = base_19;
        let _e43 = blend_19;
        let _e49 = base_19;
        let _e52 = blend_19;
        local = ((sqrt(_e40) * ((2f * _e43) - 1f)) + ((2f * _e49) * (1f - _e52)));
    }
    let _e57 = local;
    return _e57;
}

fn blendSoftLight_1(base_20: vec3<f32>, blend_20: vec3<f32>) -> vec3<f32> {
    var base_21: vec3<f32>;
    var blend_21: vec3<f32>;

    base_21 = base_20;
    blend_21 = blend_20;
    let _e22 = base_21;
    let _e24 = blend_21;
    let _e26 = blendSoftLight(_e22.x, _e24.x);
    let _e27 = base_21;
    let _e29 = blend_21;
    let _e31 = blendSoftLight(_e27.y, _e29.y);
    let _e32 = base_21;
    let _e34 = blend_21;
    let _e36 = blendSoftLight(_e32.z, _e34.z);
    return vec3<f32>(_e26, _e31, _e36);
}

fn blendSoftLight_2(base_22: vec3<f32>, blend_22: vec3<f32>, opacity_8: f32) -> vec3<f32> {
    var base_23: vec3<f32>;
    var blend_23: vec3<f32>;
    var opacity_9: f32;

    base_23 = base_22;
    blend_23 = blend_22;
    opacity_9 = opacity_8;
    let _e24 = base_23;
    let _e25 = blend_23;
    let _e26 = blendSoftLight_1(_e24, _e25);
    let _e27 = opacity_9;
    let _e29 = base_23;
    let _e31 = opacity_9;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn blendDifference(base_24: vec3<f32>, blend_24: vec3<f32>) -> vec3<f32> {
    var base_25: vec3<f32>;
    var blend_25: vec3<f32>;

    base_25 = base_24;
    blend_25 = blend_24;
    let _e22 = base_25;
    let _e23 = blend_25;
    return abs((_e22 - _e23));
}

fn blendDifference_1(base_26: vec3<f32>, blend_26: vec3<f32>, opacity_10: f32) -> vec3<f32> {
    var base_27: vec3<f32>;
    var blend_27: vec3<f32>;
    var opacity_11: f32;

    base_27 = base_26;
    blend_27 = blend_26;
    opacity_11 = opacity_10;
    let _e24 = base_27;
    let _e25 = blend_27;
    let _e26 = blendDifference(_e24, _e25);
    let _e27 = opacity_11;
    let _e29 = base_27;
    let _e31 = opacity_11;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn blendOverlay(base_28: f32, blend_28: f32) -> f32 {
    var base_29: f32;
    var blend_29: f32;
    var local_1: f32;

    base_29 = base_28;
    blend_29 = blend_28;
    let _e22 = base_29;
    if (_e22 < 0.5f) {
        let _e26 = base_29;
        let _e28 = blend_29;
        local_1 = ((2f * _e26) * _e28);
    } else {
        let _e33 = base_29;
        let _e37 = blend_29;
        local_1 = (1f - ((2f * (1f - _e33)) * (1f - _e37)));
    }
    let _e42 = local_1;
    return _e42;
}

fn blendOverlay_1(base_30: vec3<f32>, blend_30: vec3<f32>) -> vec3<f32> {
    var base_31: vec3<f32>;
    var blend_31: vec3<f32>;

    base_31 = base_30;
    blend_31 = blend_30;
    let _e22 = base_31;
    let _e24 = blend_31;
    let _e26 = blendOverlay(_e22.x, _e24.x);
    let _e27 = base_31;
    let _e29 = blend_31;
    let _e31 = blendOverlay(_e27.y, _e29.y);
    let _e32 = base_31;
    let _e34 = blend_31;
    let _e36 = blendOverlay(_e32.z, _e34.z);
    return vec3<f32>(_e26, _e31, _e36);
}

fn blendOverlay_2(base_32: vec3<f32>, blend_32: vec3<f32>, opacity_12: f32) -> vec3<f32> {
    var base_33: vec3<f32>;
    var blend_33: vec3<f32>;
    var opacity_13: f32;

    base_33 = base_32;
    blend_33 = blend_32;
    opacity_13 = opacity_12;
    let _e24 = base_33;
    let _e25 = blend_33;
    let _e26 = blendOverlay_1(_e24, _e25);
    let _e27 = opacity_13;
    let _e29 = base_33;
    let _e31 = opacity_13;
    return ((_e26 * _e27) + (_e29 * (1f - _e31)));
}

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e20 = n_1;
    return fract((sin(_e20) * 43758.547f));
}

fn main_1() {
    var sampleUv: vec2<f32>;
    var stepT: f32;
    var height: f32;
    var i: f32 = 0f;
    var barPos: f32;
    var barHeight: f32;
    var col: vec4<f32>;
    var timeCol: vec4<f32>;
    var alpha: f32;
    var staticCol: vec3<f32>;

    let _e18 = vUv_1;
    sampleUv = _e18;
    let _e20 = global.time;
    let _e23 = global.seed;
    stepT = floor(((_e20 * 20f) + (_e23 * 20f)));
    let _e29 = global.bars;
    height = (_e29 * 0.1f);
    loop {
        let _e35 = i;
        if !((_e35 < 3f)) {
            break;
        }
        {
            let _e42 = i;
            let _e43 = stepT;
            let _e45 = random1d((_e42 + _e43));
            let _e46 = global.time;
            let _e50 = global.seed;
            barPos = fract(((_e45 - (_e46 * 0.6f)) + _e50));
            let _e54 = i;
            let _e55 = stepT;
            let _e57 = random1d((_e54 + _e55));
            let _e58 = height;
            barHeight = (_e57 * _e58);
            let _e61 = sampleUv;
            let _e63 = barPos;
            let _e65 = sampleUv;
            let _e67 = barPos;
            let _e68 = barHeight;
            if ((_e61.y > _e63) && (_e65.y < (_e67 + _e68))) {
                {
                    let _e73 = sampleUv;
                    let _e75 = sampleUv;
                    let _e79 = global.time;
                    sampleUv.x = (_e73.x + (sin(((_e75.y * 10f) + _e79)) * 0.002f));
                    let _e86 = barPos;
                    sampleUv.y = _e86;
                }
            }
        }
        continuing {
            let _e39 = i;
            i = (_e39 + 1f);
        }
    }
    let _e87 = sampleUv;
    let _e88 = textureSample(tDiffuse_t, tDiffuse_s, _e87);
    col = _e88;
    let _e90 = vUv_1;
    let _e91 = textureSample(tTime_t, tTime_s, _e90);
    timeCol = _e91;
    let _e93 = global.timeOpacity;
    let _e94 = timeCol;
    alpha = (_e93 * _e94.w);
    let _e98 = col;
    let _e100 = col;
    let _e102 = timeCol;
    let _e104 = global.timeOpacity;
    let _e105 = blendAdd_1(_e100.xyz, _e102.xyz, _e104);
    col.x = _e105.x;
    col.y = _e105.y;
    col.z = _e105.z;
    let _e112 = vUv_1;
    let _e113 = textureSample(tStatic_t, tStatic_s, _e112);
    staticCol = _e113.xyz;
    let _e116 = col;
    let _e118 = col;
    let _e120 = staticCol;
    let _e121 = global.staticOpacity;
    let _e123 = (_e118.xyz + (_e120 * _e121));
    col.x = _e123.x;
    col.y = _e123.y;
    col.z = _e123.z;
    let _e130 = col;
    o_color = _e130;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e29 = o_color;
    return FragmentOutput(_e29);
}
