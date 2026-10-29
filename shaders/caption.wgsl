struct U {
    opacity: f32,
    blendModeIndex: f32,
    x: f32,
    y: f32,
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
var tCaption_t: texture_2d<f32>;
@group(0) @binding(4) 
var tCaption_s: sampler;

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

fn main_1() {
    var baseCol: vec4<f32>;
    var cUv: vec2<f32>;
    var captionCol: vec4<f32>;
    var outCol: vec3<f32>;
    var blendOpac: f32;
    var outOpac: f32;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    baseCol = _e15;
    let _e17 = vUv_1;
    cUv = _e17;
    let _e19 = cUv;
    let _e20 = global.x;
    let _e22 = global.y;
    cUv = (_e19 + vec2<f32>(-(_e20), _e22));
    let _e25 = cUv;
    let _e26 = textureSample(tCaption_t, tCaption_s, _e25);
    captionCol = _e26;
    let _e28 = baseCol;
    outCol = _e28.xyz;
    let _e31 = global.opacity;
    let _e32 = captionCol;
    blendOpac = (_e31 * _e32.w);
    let _e36 = global.blendModeIndex;
    if (_e36 == 0f) {
        {
            let _e39 = baseCol;
            let _e41 = captionCol;
            let _e43 = blendOpac;
            let _e44 = blendNormal_1(_e39.xyz, _e41.xyz, _e43);
            outCol = _e44;
        }
    } else {
        let _e45 = global.blendModeIndex;
        if (_e45 == 1f) {
            {
                let _e48 = baseCol;
                let _e50 = captionCol;
                let _e52 = blendOpac;
                let _e53 = blendAdd_1(_e48.xyz, _e50.xyz, _e52);
                outCol = _e53;
            }
        } else {
            let _e54 = global.blendModeIndex;
            if (_e54 == 2f) {
                {
                    let _e57 = baseCol;
                    let _e59 = captionCol;
                    let _e61 = blendOpac;
                    let _e62 = blendMultiply_1(_e57.xyz, _e59.xyz, _e61);
                    outCol = _e62;
                }
            } else {
                let _e63 = global.blendModeIndex;
                if (_e63 == 3f) {
                    {
                        let _e66 = baseCol;
                        let _e68 = captionCol;
                        let _e70 = blendOpac;
                        let _e71 = blendScreen_2(_e66.xyz, _e68.xyz, _e70);
                        outCol = _e71;
                    }
                } else {
                    let _e72 = global.blendModeIndex;
                    if (_e72 == 4f) {
                        {
                            let _e75 = baseCol;
                            let _e77 = captionCol;
                            let _e79 = blendOpac;
                            let _e80 = blendSoftLight_2(_e75.xyz, _e77.xyz, _e79);
                            outCol = _e80;
                        }
                    } else {
                        let _e81 = global.blendModeIndex;
                        if (_e81 == 5f) {
                            {
                                let _e84 = baseCol;
                                let _e86 = captionCol;
                                let _e88 = blendOpac;
                                let _e89 = blendDifference_1(_e84.xyz, _e86.xyz, _e88);
                                outCol = _e89;
                            }
                        }
                    }
                }
            }
        }
    }
    let _e90 = blendOpac;
    let _e91 = baseCol;
    outOpac = (_e90 + _e91.w);
    let _e95 = outCol;
    let _e96 = outOpac;
    o_color = vec4<f32>(_e95.x, _e95.y, _e95.z, _e96);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e23 = o_color;
    return FragmentOutput(_e23);
}
