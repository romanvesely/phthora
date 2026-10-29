struct U {
    opacity: f32,
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
@group(0) @binding(3) 
var barsTexture_t: texture_2d<f32>;
@group(0) @binding(4) 
var barsTexture_s: sampler;

fn blendNormal(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    var base_1: vec3<f32>;
    var blend_1: vec3<f32>;

    base_1 = base;
    blend_1 = blend;
    let _e14 = blend_1;
    return _e14;
}

fn blendNormal_1(base_2: vec3<f32>, blend_2: vec3<f32>, opacity: f32) -> vec3<f32> {
    var base_3: vec3<f32>;
    var blend_3: vec3<f32>;
    var opacity_1: f32;

    base_3 = base_2;
    blend_3 = blend_2;
    opacity_1 = opacity;
    let _e16 = base_3;
    let _e17 = blend_3;
    let _e18 = blendNormal(_e16, _e17);
    let _e19 = opacity_1;
    let _e21 = base_3;
    let _e23 = opacity_1;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn blendMultiply(base_4: vec3<f32>, blend_4: vec3<f32>) -> vec3<f32> {
    var base_5: vec3<f32>;
    var blend_5: vec3<f32>;

    base_5 = base_4;
    blend_5 = blend_4;
    let _e14 = base_5;
    let _e15 = blend_5;
    return (_e14 * _e15);
}

fn blendMultiply_1(base_6: vec3<f32>, blend_6: vec3<f32>, opacity_2: f32) -> vec3<f32> {
    var base_7: vec3<f32>;
    var blend_7: vec3<f32>;
    var opacity_3: f32;

    base_7 = base_6;
    blend_7 = blend_6;
    opacity_3 = opacity_2;
    let _e16 = base_7;
    let _e17 = blend_7;
    let _e18 = blendMultiply(_e16, _e17);
    let _e19 = opacity_3;
    let _e21 = base_7;
    let _e23 = opacity_3;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn blendAdd(base_8: vec3<f32>, blend_8: vec3<f32>) -> vec3<f32> {
    var base_9: vec3<f32>;
    var blend_9: vec3<f32>;

    base_9 = base_8;
    blend_9 = blend_8;
    let _e14 = base_9;
    let _e15 = blend_9;
    return min((_e14 + _e15), vec3(1f));
}

fn blendAdd_1(base_10: vec3<f32>, blend_10: vec3<f32>, opacity_4: f32) -> vec3<f32> {
    var base_11: vec3<f32>;
    var blend_11: vec3<f32>;
    var opacity_5: f32;

    base_11 = base_10;
    blend_11 = blend_10;
    opacity_5 = opacity_4;
    let _e16 = base_11;
    let _e17 = blend_11;
    let _e18 = blendAdd(_e16, _e17);
    let _e19 = opacity_5;
    let _e21 = base_11;
    let _e23 = opacity_5;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn blendScreen(base_12: f32, blend_12: f32) -> f32 {
    var base_13: f32;
    var blend_13: f32;

    base_13 = base_12;
    blend_13 = blend_12;
    let _e16 = base_13;
    let _e19 = blend_13;
    return (1f - ((1f - _e16) * (1f - _e19)));
}

fn blendScreen_1(base_14: vec3<f32>, blend_14: vec3<f32>) -> vec3<f32> {
    var base_15: vec3<f32>;
    var blend_15: vec3<f32>;

    base_15 = base_14;
    blend_15 = blend_14;
    let _e14 = base_15;
    let _e16 = blend_15;
    let _e18 = blendScreen(_e14.x, _e16.x);
    let _e19 = base_15;
    let _e21 = blend_15;
    let _e23 = blendScreen(_e19.y, _e21.y);
    let _e24 = base_15;
    let _e26 = blend_15;
    let _e28 = blendScreen(_e24.z, _e26.z);
    return vec3<f32>(_e18, _e23, _e28);
}

fn blendScreen_2(base_16: vec3<f32>, blend_16: vec3<f32>, opacity_6: f32) -> vec3<f32> {
    var base_17: vec3<f32>;
    var blend_17: vec3<f32>;
    var opacity_7: f32;

    base_17 = base_16;
    blend_17 = blend_16;
    opacity_7 = opacity_6;
    let _e16 = base_17;
    let _e17 = blend_17;
    let _e18 = blendScreen_1(_e16, _e17);
    let _e19 = opacity_7;
    let _e21 = base_17;
    let _e23 = opacity_7;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn blendSoftLight(base_18: f32, blend_18: f32) -> f32 {
    var base_19: f32;
    var blend_19: f32;
    var local: f32;

    base_19 = base_18;
    blend_19 = blend_18;
    let _e14 = blend_19;
    if (_e14 < 0.5f) {
        let _e18 = base_19;
        let _e20 = blend_19;
        let _e22 = base_19;
        let _e23 = base_19;
        let _e27 = blend_19;
        local = (((2f * _e18) * _e20) + ((_e22 * _e23) * (1f - (2f * _e27))));
    } else {
        let _e32 = base_19;
        let _e35 = blend_19;
        let _e41 = base_19;
        let _e44 = blend_19;
        local = ((sqrt(_e32) * ((2f * _e35) - 1f)) + ((2f * _e41) * (1f - _e44)));
    }
    let _e49 = local;
    return _e49;
}

fn blendSoftLight_1(base_20: vec3<f32>, blend_20: vec3<f32>) -> vec3<f32> {
    var base_21: vec3<f32>;
    var blend_21: vec3<f32>;

    base_21 = base_20;
    blend_21 = blend_20;
    let _e14 = base_21;
    let _e16 = blend_21;
    let _e18 = blendSoftLight(_e14.x, _e16.x);
    let _e19 = base_21;
    let _e21 = blend_21;
    let _e23 = blendSoftLight(_e19.y, _e21.y);
    let _e24 = base_21;
    let _e26 = blend_21;
    let _e28 = blendSoftLight(_e24.z, _e26.z);
    return vec3<f32>(_e18, _e23, _e28);
}

fn blendSoftLight_2(base_22: vec3<f32>, blend_22: vec3<f32>, opacity_8: f32) -> vec3<f32> {
    var base_23: vec3<f32>;
    var blend_23: vec3<f32>;
    var opacity_9: f32;

    base_23 = base_22;
    blend_23 = blend_22;
    opacity_9 = opacity_8;
    let _e16 = base_23;
    let _e17 = blend_23;
    let _e18 = blendSoftLight_1(_e16, _e17);
    let _e19 = opacity_9;
    let _e21 = base_23;
    let _e23 = opacity_9;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn blendDifference(base_24: vec3<f32>, blend_24: vec3<f32>) -> vec3<f32> {
    var base_25: vec3<f32>;
    var blend_25: vec3<f32>;

    base_25 = base_24;
    blend_25 = blend_24;
    let _e14 = base_25;
    let _e15 = blend_25;
    return abs((_e14 - _e15));
}

fn blendDifference_1(base_26: vec3<f32>, blend_26: vec3<f32>, opacity_10: f32) -> vec3<f32> {
    var base_27: vec3<f32>;
    var blend_27: vec3<f32>;
    var opacity_11: f32;

    base_27 = base_26;
    blend_27 = blend_26;
    opacity_11 = opacity_10;
    let _e16 = base_27;
    let _e17 = blend_27;
    let _e18 = blendDifference(_e16, _e17);
    let _e19 = opacity_11;
    let _e21 = base_27;
    let _e23 = opacity_11;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn blendOverlay(base_28: f32, blend_28: f32) -> f32 {
    var base_29: f32;
    var blend_29: f32;
    var local_1: f32;

    base_29 = base_28;
    blend_29 = blend_28;
    let _e14 = base_29;
    if (_e14 < 0.5f) {
        let _e18 = base_29;
        let _e20 = blend_29;
        local_1 = ((2f * _e18) * _e20);
    } else {
        let _e25 = base_29;
        let _e29 = blend_29;
        local_1 = (1f - ((2f * (1f - _e25)) * (1f - _e29)));
    }
    let _e34 = local_1;
    return _e34;
}

fn blendOverlay_1(base_30: vec3<f32>, blend_30: vec3<f32>) -> vec3<f32> {
    var base_31: vec3<f32>;
    var blend_31: vec3<f32>;

    base_31 = base_30;
    blend_31 = blend_30;
    let _e14 = base_31;
    let _e16 = blend_31;
    let _e18 = blendOverlay(_e14.x, _e16.x);
    let _e19 = base_31;
    let _e21 = blend_31;
    let _e23 = blendOverlay(_e19.y, _e21.y);
    let _e24 = base_31;
    let _e26 = blend_31;
    let _e28 = blendOverlay(_e24.z, _e26.z);
    return vec3<f32>(_e18, _e23, _e28);
}

fn blendOverlay_2(base_32: vec3<f32>, blend_32: vec3<f32>, opacity_12: f32) -> vec3<f32> {
    var base_33: vec3<f32>;
    var blend_33: vec3<f32>;
    var opacity_13: f32;

    base_33 = base_32;
    blend_33 = blend_32;
    opacity_13 = opacity_12;
    let _e16 = base_33;
    let _e17 = blend_33;
    let _e18 = blendOverlay_1(_e16, _e17);
    let _e19 = opacity_13;
    let _e21 = base_33;
    let _e23 = opacity_13;
    return ((_e18 * _e19) + (_e21 * (1f - _e23)));
}

fn main_1() {
    var base_34: vec4<f32>;
    var bars: vec4<f32>;
    var blendOpacity: f32;
    var outCol: vec3<f32>;
    var outOpac: f32;

    let _e10 = vUv_1;
    let _e11 = textureSample(tDiffuse_t, tDiffuse_s, _e10);
    base_34 = _e11;
    let _e13 = vUv_1;
    let _e14 = textureSample(barsTexture_t, barsTexture_s, _e13);
    bars = _e14;
    let _e16 = global.opacity;
    let _e17 = bars;
    blendOpacity = (_e16 * _e17.w);
    let _e22 = global.blendModeIndex;
    if (_e22 == 0f) {
        {
            let _e25 = base_34;
            let _e27 = bars;
            let _e29 = blendOpacity;
            let _e30 = blendNormal_1(_e25.xyz, _e27.xyz, _e29);
            outCol = _e30;
        }
    } else {
        let _e31 = global.blendModeIndex;
        if (_e31 == 1f) {
            {
                let _e34 = base_34;
                let _e36 = bars;
                let _e38 = blendOpacity;
                let _e39 = blendAdd_1(_e34.xyz, _e36.xyz, _e38);
                outCol = _e39;
            }
        } else {
            let _e40 = global.blendModeIndex;
            if (_e40 == 2f) {
                {
                    let _e43 = base_34;
                    let _e45 = bars;
                    let _e47 = blendOpacity;
                    let _e48 = blendMultiply_1(_e43.xyz, _e45.xyz, _e47);
                    outCol = _e48;
                }
            } else {
                let _e49 = global.blendModeIndex;
                if (_e49 == 3f) {
                    {
                        let _e52 = base_34;
                        let _e54 = bars;
                        let _e56 = blendOpacity;
                        let _e57 = blendScreen_2(_e52.xyz, _e54.xyz, _e56);
                        outCol = _e57;
                    }
                } else {
                    let _e58 = global.blendModeIndex;
                    if (_e58 == 4f) {
                        {
                            let _e61 = base_34;
                            let _e63 = bars;
                            let _e65 = blendOpacity;
                            let _e66 = blendSoftLight_2(_e61.xyz, _e63.xyz, _e65);
                            outCol = _e66;
                        }
                    } else {
                        let _e67 = global.blendModeIndex;
                        if (_e67 == 5f) {
                            {
                                let _e70 = base_34;
                                let _e72 = bars;
                                let _e74 = blendOpacity;
                                let _e75 = blendDifference_1(_e70.xyz, _e72.xyz, _e74);
                                outCol = _e75;
                            }
                        }
                    }
                }
            }
        }
    }
    let _e76 = blendOpacity;
    let _e77 = base_34;
    outOpac = (_e76 + _e77.w);
    let _e81 = outCol;
    let _e82 = outOpac;
    o_color = vec4<f32>(_e81.x, _e81.y, _e81.z, _e82);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
