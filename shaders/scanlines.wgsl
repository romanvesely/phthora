struct U {
    count: f32,
    amount: f32,
    resolution: vec2<f32>,
    angle: f32,
    blendModeIndex: f32,
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

fn blendNormal(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    var base_1: vec3<f32>;
    var blend_1: vec3<f32>;

    base_1 = base;
    blend_1 = blend;
    let _e18 = blend_1;
    return _e18;
}

fn blendNormal_1(base_2: vec3<f32>, blend_2: vec3<f32>, opacity: f32) -> vec3<f32> {
    var base_3: vec3<f32>;
    var blend_3: vec3<f32>;
    var opacity_1: f32;

    base_3 = base_2;
    blend_3 = blend_2;
    opacity_1 = opacity;
    let _e20 = base_3;
    let _e21 = blend_3;
    let _e22 = blendNormal(_e20, _e21);
    let _e23 = opacity_1;
    let _e25 = base_3;
    let _e27 = opacity_1;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn blendMultiply(base_4: vec3<f32>, blend_4: vec3<f32>) -> vec3<f32> {
    var base_5: vec3<f32>;
    var blend_5: vec3<f32>;

    base_5 = base_4;
    blend_5 = blend_4;
    let _e18 = base_5;
    let _e19 = blend_5;
    return (_e18 * _e19);
}

fn blendMultiply_1(base_6: vec3<f32>, blend_6: vec3<f32>, opacity_2: f32) -> vec3<f32> {
    var base_7: vec3<f32>;
    var blend_7: vec3<f32>;
    var opacity_3: f32;

    base_7 = base_6;
    blend_7 = blend_6;
    opacity_3 = opacity_2;
    let _e20 = base_7;
    let _e21 = blend_7;
    let _e22 = blendMultiply(_e20, _e21);
    let _e23 = opacity_3;
    let _e25 = base_7;
    let _e27 = opacity_3;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn blendAdd(base_8: vec3<f32>, blend_8: vec3<f32>) -> vec3<f32> {
    var base_9: vec3<f32>;
    var blend_9: vec3<f32>;

    base_9 = base_8;
    blend_9 = blend_8;
    let _e18 = base_9;
    let _e19 = blend_9;
    return min((_e18 + _e19), vec3(1f));
}

fn blendAdd_1(base_10: vec3<f32>, blend_10: vec3<f32>, opacity_4: f32) -> vec3<f32> {
    var base_11: vec3<f32>;
    var blend_11: vec3<f32>;
    var opacity_5: f32;

    base_11 = base_10;
    blend_11 = blend_10;
    opacity_5 = opacity_4;
    let _e20 = base_11;
    let _e21 = blend_11;
    let _e22 = blendAdd(_e20, _e21);
    let _e23 = opacity_5;
    let _e25 = base_11;
    let _e27 = opacity_5;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn blendScreen(base_12: f32, blend_12: f32) -> f32 {
    var base_13: f32;
    var blend_13: f32;

    base_13 = base_12;
    blend_13 = blend_12;
    let _e20 = base_13;
    let _e23 = blend_13;
    return (1f - ((1f - _e20) * (1f - _e23)));
}

fn blendScreen_1(base_14: vec3<f32>, blend_14: vec3<f32>) -> vec3<f32> {
    var base_15: vec3<f32>;
    var blend_15: vec3<f32>;

    base_15 = base_14;
    blend_15 = blend_14;
    let _e18 = base_15;
    let _e20 = blend_15;
    let _e22 = blendScreen(_e18.x, _e20.x);
    let _e23 = base_15;
    let _e25 = blend_15;
    let _e27 = blendScreen(_e23.y, _e25.y);
    let _e28 = base_15;
    let _e30 = blend_15;
    let _e32 = blendScreen(_e28.z, _e30.z);
    return vec3<f32>(_e22, _e27, _e32);
}

fn blendScreen_2(base_16: vec3<f32>, blend_16: vec3<f32>, opacity_6: f32) -> vec3<f32> {
    var base_17: vec3<f32>;
    var blend_17: vec3<f32>;
    var opacity_7: f32;

    base_17 = base_16;
    blend_17 = blend_16;
    opacity_7 = opacity_6;
    let _e20 = base_17;
    let _e21 = blend_17;
    let _e22 = blendScreen_1(_e20, _e21);
    let _e23 = opacity_7;
    let _e25 = base_17;
    let _e27 = opacity_7;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn blendSoftLight(base_18: f32, blend_18: f32) -> f32 {
    var base_19: f32;
    var blend_19: f32;
    var local: f32;

    base_19 = base_18;
    blend_19 = blend_18;
    let _e18 = blend_19;
    if (_e18 < 0.5f) {
        let _e22 = base_19;
        let _e24 = blend_19;
        let _e26 = base_19;
        let _e27 = base_19;
        let _e31 = blend_19;
        local = (((2f * _e22) * _e24) + ((_e26 * _e27) * (1f - (2f * _e31))));
    } else {
        let _e36 = base_19;
        let _e39 = blend_19;
        let _e45 = base_19;
        let _e48 = blend_19;
        local = ((sqrt(_e36) * ((2f * _e39) - 1f)) + ((2f * _e45) * (1f - _e48)));
    }
    let _e53 = local;
    return _e53;
}

fn blendSoftLight_1(base_20: vec3<f32>, blend_20: vec3<f32>) -> vec3<f32> {
    var base_21: vec3<f32>;
    var blend_21: vec3<f32>;

    base_21 = base_20;
    blend_21 = blend_20;
    let _e18 = base_21;
    let _e20 = blend_21;
    let _e22 = blendSoftLight(_e18.x, _e20.x);
    let _e23 = base_21;
    let _e25 = blend_21;
    let _e27 = blendSoftLight(_e23.y, _e25.y);
    let _e28 = base_21;
    let _e30 = blend_21;
    let _e32 = blendSoftLight(_e28.z, _e30.z);
    return vec3<f32>(_e22, _e27, _e32);
}

fn blendSoftLight_2(base_22: vec3<f32>, blend_22: vec3<f32>, opacity_8: f32) -> vec3<f32> {
    var base_23: vec3<f32>;
    var blend_23: vec3<f32>;
    var opacity_9: f32;

    base_23 = base_22;
    blend_23 = blend_22;
    opacity_9 = opacity_8;
    let _e20 = base_23;
    let _e21 = blend_23;
    let _e22 = blendSoftLight_1(_e20, _e21);
    let _e23 = opacity_9;
    let _e25 = base_23;
    let _e27 = opacity_9;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn blendDifference(base_24: vec3<f32>, blend_24: vec3<f32>) -> vec3<f32> {
    var base_25: vec3<f32>;
    var blend_25: vec3<f32>;

    base_25 = base_24;
    blend_25 = blend_24;
    let _e18 = base_25;
    let _e19 = blend_25;
    return abs((_e18 - _e19));
}

fn blendDifference_1(base_26: vec3<f32>, blend_26: vec3<f32>, opacity_10: f32) -> vec3<f32> {
    var base_27: vec3<f32>;
    var blend_27: vec3<f32>;
    var opacity_11: f32;

    base_27 = base_26;
    blend_27 = blend_26;
    opacity_11 = opacity_10;
    let _e20 = base_27;
    let _e21 = blend_27;
    let _e22 = blendDifference(_e20, _e21);
    let _e23 = opacity_11;
    let _e25 = base_27;
    let _e27 = opacity_11;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn blendOverlay(base_28: f32, blend_28: f32) -> f32 {
    var base_29: f32;
    var blend_29: f32;
    var local_1: f32;

    base_29 = base_28;
    blend_29 = blend_28;
    let _e18 = base_29;
    if (_e18 < 0.5f) {
        let _e22 = base_29;
        let _e24 = blend_29;
        local_1 = ((2f * _e22) * _e24);
    } else {
        let _e29 = base_29;
        let _e33 = blend_29;
        local_1 = (1f - ((2f * (1f - _e29)) * (1f - _e33)));
    }
    let _e38 = local_1;
    return _e38;
}

fn blendOverlay_1(base_30: vec3<f32>, blend_30: vec3<f32>) -> vec3<f32> {
    var base_31: vec3<f32>;
    var blend_31: vec3<f32>;

    base_31 = base_30;
    blend_31 = blend_30;
    let _e18 = base_31;
    let _e20 = blend_31;
    let _e22 = blendOverlay(_e18.x, _e20.x);
    let _e23 = base_31;
    let _e25 = blend_31;
    let _e27 = blendOverlay(_e23.y, _e25.y);
    let _e28 = base_31;
    let _e30 = blend_31;
    let _e32 = blendOverlay(_e28.z, _e30.z);
    return vec3<f32>(_e22, _e27, _e32);
}

fn blendOverlay_2(base_32: vec3<f32>, blend_32: vec3<f32>, opacity_12: f32) -> vec3<f32> {
    var base_33: vec3<f32>;
    var blend_33: vec3<f32>;
    var opacity_13: f32;

    base_33 = base_32;
    blend_33 = blend_32;
    opacity_13 = opacity_12;
    let _e20 = base_33;
    let _e21 = blend_33;
    let _e22 = blendOverlay_1(_e20, _e21);
    let _e23 = opacity_13;
    let _e25 = base_33;
    let _e27 = opacity_13;
    return ((_e22 * _e23) + (_e25 * (1f - _e27)));
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e18 = theta_1;
    let _e20 = theta_1;
    let _e23 = theta_1;
    let _e25 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e18), -(sin(_e20))), vec2<f32>(sin(_e23), cos(_e25)));
    let _e31 = m;
    let _e32 = position_1;
    return (_e31 * _e32);
}

fn main_1() {
    var col: vec4<f32>;
    var uv: vec2<f32>;
    var lineCount: f32;
    var sc: vec2<f32>;
    var lines: vec3<f32>;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    col = _e15;
    let _e17 = vUv_1;
    uv = _e17;
    let _e19 = uv;
    uv = (_e19 - vec2(0.5f));
    let _e23 = uv;
    let _e24 = global.angle;
    let _e27 = rotate2d(_e23, (_e24 * 3.1415927f));
    uv = _e27;
    let _e28 = uv;
    uv = (_e28 + vec2(0.5f));
    let _e32 = global.resolution;
    let _e38 = global.count;
    lineCount = (((_e32.y * 3.1415927f) / 2f) * _e38);
    let _e41 = uv;
    let _e43 = lineCount;
    let _e46 = uv;
    let _e48 = lineCount;
    sc = vec2<f32>(sin((_e41.y * _e43)), cos((_e46.y * _e48)));
    let _e53 = sc;
    let _e55 = sc;
    let _e57 = sc;
    lines = vec3<f32>(_e53.x, _e55.y, _e57.x);
    let _e61 = lines;
    lines = (_e61 * 0.5f);
    let _e64 = global.blendModeIndex;
    if (_e64 == 0f) {
        {
            let _e67 = col;
            let _e69 = col;
            let _e71 = lines;
            let _e72 = global.amount;
            let _e73 = blendAdd_1(_e69.xyz, _e71, _e72);
            col.x = _e73.x;
            col.y = _e73.y;
            col.z = _e73.z;
        }
    } else {
        let _e80 = global.blendModeIndex;
        if (_e80 == 1f) {
            {
                let _e83 = lines;
                lines = (_e83 + vec3(0.5f));
                let _e87 = col;
                let _e89 = col;
                let _e91 = lines;
                let _e92 = global.amount;
                let _e93 = blendMultiply_1(_e89.xyz, _e91, _e92);
                col.x = _e93.x;
                col.y = _e93.y;
                col.z = _e93.z;
            }
        } else {
            let _e100 = global.blendModeIndex;
            if (_e100 == 2f) {
                {
                    let _e103 = col;
                    let _e105 = col;
                    let _e107 = lines;
                    let _e108 = global.amount;
                    let _e109 = blendScreen_2(_e105.xyz, _e107, _e108);
                    col.x = _e109.x;
                    col.y = _e109.y;
                    col.z = _e109.z;
                }
            } else {
                let _e116 = global.blendModeIndex;
                if (_e116 == 3f) {
                    {
                        let _e119 = col;
                        let _e121 = col;
                        let _e123 = lines;
                        let _e124 = global.amount;
                        let _e125 = blendSoftLight_2(_e121.xyz, _e123, _e124);
                        col.x = _e125.x;
                        col.y = _e125.y;
                        col.z = _e125.z;
                    }
                }
            }
        }
    }
    let _e132 = col;
    o_color = vec4<f32>(_e132);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
