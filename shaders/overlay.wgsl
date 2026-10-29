struct U {
    opacity: f32,
    blendModeIndex: f32,
    x: f32,
    y: f32,
    scalex: f32,
    scaley: f32,
    angle: f32,
    b_repeat: f32,
    b_maintianAspect: f32,
    resolution: vec2<f32>,
    sourceAspect: f32,
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
var tOverlay_t: texture_2d<f32>;
@group(0) @binding(4) 
var tOverlay_s: sampler;

fn blendNormal(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    var base_1: vec3<f32>;
    var blend_1: vec3<f32>;

    base_1 = base;
    blend_1 = blend;
    let _e32 = blend_1;
    return _e32;
}

fn blendNormal_1(base_2: vec3<f32>, blend_2: vec3<f32>, opacity: f32) -> vec3<f32> {
    var base_3: vec3<f32>;
    var blend_3: vec3<f32>;
    var opacity_1: f32;

    base_3 = base_2;
    blend_3 = blend_2;
    opacity_1 = opacity;
    let _e34 = base_3;
    let _e35 = blend_3;
    let _e36 = blendNormal(_e34, _e35);
    let _e37 = opacity_1;
    let _e39 = base_3;
    let _e41 = opacity_1;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn blendMultiply(base_4: vec3<f32>, blend_4: vec3<f32>) -> vec3<f32> {
    var base_5: vec3<f32>;
    var blend_5: vec3<f32>;

    base_5 = base_4;
    blend_5 = blend_4;
    let _e32 = base_5;
    let _e33 = blend_5;
    return (_e32 * _e33);
}

fn blendMultiply_1(base_6: vec3<f32>, blend_6: vec3<f32>, opacity_2: f32) -> vec3<f32> {
    var base_7: vec3<f32>;
    var blend_7: vec3<f32>;
    var opacity_3: f32;

    base_7 = base_6;
    blend_7 = blend_6;
    opacity_3 = opacity_2;
    let _e34 = base_7;
    let _e35 = blend_7;
    let _e36 = blendMultiply(_e34, _e35);
    let _e37 = opacity_3;
    let _e39 = base_7;
    let _e41 = opacity_3;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn blendAdd(base_8: vec3<f32>, blend_8: vec3<f32>) -> vec3<f32> {
    var base_9: vec3<f32>;
    var blend_9: vec3<f32>;

    base_9 = base_8;
    blend_9 = blend_8;
    let _e32 = base_9;
    let _e33 = blend_9;
    return min((_e32 + _e33), vec3(1f));
}

fn blendAdd_1(base_10: vec3<f32>, blend_10: vec3<f32>, opacity_4: f32) -> vec3<f32> {
    var base_11: vec3<f32>;
    var blend_11: vec3<f32>;
    var opacity_5: f32;

    base_11 = base_10;
    blend_11 = blend_10;
    opacity_5 = opacity_4;
    let _e34 = base_11;
    let _e35 = blend_11;
    let _e36 = blendAdd(_e34, _e35);
    let _e37 = opacity_5;
    let _e39 = base_11;
    let _e41 = opacity_5;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn blendScreen(base_12: f32, blend_12: f32) -> f32 {
    var base_13: f32;
    var blend_13: f32;

    base_13 = base_12;
    blend_13 = blend_12;
    let _e34 = base_13;
    let _e37 = blend_13;
    return (1f - ((1f - _e34) * (1f - _e37)));
}

fn blendScreen_1(base_14: vec3<f32>, blend_14: vec3<f32>) -> vec3<f32> {
    var base_15: vec3<f32>;
    var blend_15: vec3<f32>;

    base_15 = base_14;
    blend_15 = blend_14;
    let _e32 = base_15;
    let _e34 = blend_15;
    let _e36 = blendScreen(_e32.x, _e34.x);
    let _e37 = base_15;
    let _e39 = blend_15;
    let _e41 = blendScreen(_e37.y, _e39.y);
    let _e42 = base_15;
    let _e44 = blend_15;
    let _e46 = blendScreen(_e42.z, _e44.z);
    return vec3<f32>(_e36, _e41, _e46);
}

fn blendScreen_2(base_16: vec3<f32>, blend_16: vec3<f32>, opacity_6: f32) -> vec3<f32> {
    var base_17: vec3<f32>;
    var blend_17: vec3<f32>;
    var opacity_7: f32;

    base_17 = base_16;
    blend_17 = blend_16;
    opacity_7 = opacity_6;
    let _e34 = base_17;
    let _e35 = blend_17;
    let _e36 = blendScreen_1(_e34, _e35);
    let _e37 = opacity_7;
    let _e39 = base_17;
    let _e41 = opacity_7;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn blendSoftLight(base_18: f32, blend_18: f32) -> f32 {
    var base_19: f32;
    var blend_19: f32;
    var local: f32;

    base_19 = base_18;
    blend_19 = blend_18;
    let _e32 = blend_19;
    if (_e32 < 0.5f) {
        let _e36 = base_19;
        let _e38 = blend_19;
        let _e40 = base_19;
        let _e41 = base_19;
        let _e45 = blend_19;
        local = (((2f * _e36) * _e38) + ((_e40 * _e41) * (1f - (2f * _e45))));
    } else {
        let _e50 = base_19;
        let _e53 = blend_19;
        let _e59 = base_19;
        let _e62 = blend_19;
        local = ((sqrt(_e50) * ((2f * _e53) - 1f)) + ((2f * _e59) * (1f - _e62)));
    }
    let _e67 = local;
    return _e67;
}

fn blendSoftLight_1(base_20: vec3<f32>, blend_20: vec3<f32>) -> vec3<f32> {
    var base_21: vec3<f32>;
    var blend_21: vec3<f32>;

    base_21 = base_20;
    blend_21 = blend_20;
    let _e32 = base_21;
    let _e34 = blend_21;
    let _e36 = blendSoftLight(_e32.x, _e34.x);
    let _e37 = base_21;
    let _e39 = blend_21;
    let _e41 = blendSoftLight(_e37.y, _e39.y);
    let _e42 = base_21;
    let _e44 = blend_21;
    let _e46 = blendSoftLight(_e42.z, _e44.z);
    return vec3<f32>(_e36, _e41, _e46);
}

fn blendSoftLight_2(base_22: vec3<f32>, blend_22: vec3<f32>, opacity_8: f32) -> vec3<f32> {
    var base_23: vec3<f32>;
    var blend_23: vec3<f32>;
    var opacity_9: f32;

    base_23 = base_22;
    blend_23 = blend_22;
    opacity_9 = opacity_8;
    let _e34 = base_23;
    let _e35 = blend_23;
    let _e36 = blendSoftLight_1(_e34, _e35);
    let _e37 = opacity_9;
    let _e39 = base_23;
    let _e41 = opacity_9;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn blendDifference(base_24: vec3<f32>, blend_24: vec3<f32>) -> vec3<f32> {
    var base_25: vec3<f32>;
    var blend_25: vec3<f32>;

    base_25 = base_24;
    blend_25 = blend_24;
    let _e32 = base_25;
    let _e33 = blend_25;
    return abs((_e32 - _e33));
}

fn blendDifference_1(base_26: vec3<f32>, blend_26: vec3<f32>, opacity_10: f32) -> vec3<f32> {
    var base_27: vec3<f32>;
    var blend_27: vec3<f32>;
    var opacity_11: f32;

    base_27 = base_26;
    blend_27 = blend_26;
    opacity_11 = opacity_10;
    let _e34 = base_27;
    let _e35 = blend_27;
    let _e36 = blendDifference(_e34, _e35);
    let _e37 = opacity_11;
    let _e39 = base_27;
    let _e41 = opacity_11;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn blendOverlay(base_28: f32, blend_28: f32) -> f32 {
    var base_29: f32;
    var blend_29: f32;
    var local_1: f32;

    base_29 = base_28;
    blend_29 = blend_28;
    let _e32 = base_29;
    if (_e32 < 0.5f) {
        let _e36 = base_29;
        let _e38 = blend_29;
        local_1 = ((2f * _e36) * _e38);
    } else {
        let _e43 = base_29;
        let _e47 = blend_29;
        local_1 = (1f - ((2f * (1f - _e43)) * (1f - _e47)));
    }
    let _e52 = local_1;
    return _e52;
}

fn blendOverlay_1(base_30: vec3<f32>, blend_30: vec3<f32>) -> vec3<f32> {
    var base_31: vec3<f32>;
    var blend_31: vec3<f32>;

    base_31 = base_30;
    blend_31 = blend_30;
    let _e32 = base_31;
    let _e34 = blend_31;
    let _e36 = blendOverlay(_e32.x, _e34.x);
    let _e37 = base_31;
    let _e39 = blend_31;
    let _e41 = blendOverlay(_e37.y, _e39.y);
    let _e42 = base_31;
    let _e44 = blend_31;
    let _e46 = blendOverlay(_e42.z, _e44.z);
    return vec3<f32>(_e36, _e41, _e46);
}

fn blendOverlay_2(base_32: vec3<f32>, blend_32: vec3<f32>, opacity_12: f32) -> vec3<f32> {
    var base_33: vec3<f32>;
    var blend_33: vec3<f32>;
    var opacity_13: f32;

    base_33 = base_32;
    blend_33 = blend_32;
    opacity_13 = opacity_12;
    let _e34 = base_33;
    let _e35 = blend_33;
    let _e36 = blendOverlay_1(_e34, _e35);
    let _e37 = opacity_13;
    let _e39 = base_33;
    let _e41 = opacity_13;
    return ((_e36 * _e37) + (_e39 * (1f - _e41)));
}

fn insideBox(v: vec2<f32>, bottomLeft: vec2<f32>, topRight: vec2<f32>) -> f32 {
    var v_1: vec2<f32>;
    var bottomLeft_1: vec2<f32>;
    var topRight_1: vec2<f32>;
    var s: vec2<f32>;

    v_1 = v;
    bottomLeft_1 = bottomLeft;
    topRight_1 = topRight;
    let _e34 = bottomLeft_1;
    let _e35 = v_1;
    let _e37 = topRight_1;
    let _e38 = v_1;
    s = (step(_e34, _e35) - step(_e37, _e38));
    let _e42 = s;
    let _e44 = s;
    return (_e42.x * _e44.y);
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e32 = theta_1;
    let _e34 = theta_1;
    let _e37 = theta_1;
    let _e39 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e32), -(sin(_e34))), vec2<f32>(sin(_e37), cos(_e39)));
    let _e45 = m;
    let _e46 = position_1;
    return (_e45 * _e46);
}

fn main_1() {
    var overUv: vec2<f32>;
    var screenAspect: f32;
    var base_34: vec4<f32>;
    var blend_34: vec4<f32>;
    var outCol: vec3<f32>;
    var blendOpac: f32;
    var outOpac: f32;

    let _e28 = vUv_1;
    overUv = _e28;
    let _e30 = global.resolution;
    let _e32 = global.resolution;
    screenAspect = (_e30.x / _e32.y);
    let _e36 = overUv;
    let _e37 = global.x;
    let _e38 = global.y;
    let _e44 = global.scalex;
    let _e45 = global.scaley;
    overUv = (_e36 - ((vec2<f32>(_e37, _e38) - vec2(0.5f)) * (vec2(1f) + vec2<f32>(_e44, _e45))));
    let _e51 = overUv;
    overUv = (_e51 - vec2(0.5f));
    let _e56 = overUv;
    let _e58 = global.resolution;
    let _e60 = global.resolution;
    overUv.y = (_e56.y * (_e58.y / _e60.x));
    let _e64 = overUv;
    let _e65 = global.angle;
    let _e68 = rotate2d(_e64, (_e65 * 6.2831855f));
    overUv = _e68;
    let _e70 = overUv;
    let _e72 = global.resolution;
    let _e74 = global.resolution;
    overUv.y = (_e70.y * (_e72.x / _e74.y));
    let _e78 = global.b_maintianAspect;
    if (_e78 > 0.5f) {
        let _e82 = overUv;
        let _e84 = screenAspect;
        let _e85 = global.sourceAspect;
        overUv.x = (_e82.x * (_e84 / _e85));
    }
    let _e88 = overUv;
    let _e90 = global.scalex;
    let _e91 = global.scaley;
    overUv = (_e88 * (vec2(1f) / vec2<f32>(_e90, _e91)));
    let _e96 = overUv;
    overUv = (_e96 + vec2(0.5f));
    let _e100 = global.b_repeat;
    if (_e100 > 0.5f) {
        {
            let _e103 = overUv;
            overUv = fract(_e103);
        }
    }
    let _e105 = vUv_1;
    let _e106 = textureSample(tDiffuse_t, tDiffuse_s, _e105);
    base_34 = _e106;
    let _e108 = overUv;
    let _e109 = textureSample(tOverlay_t, tOverlay_s, _e108);
    blend_34 = _e109;
    let _e111 = base_34;
    outCol = _e111.xyz;
    let _e114 = global.opacity;
    let _e115 = blend_34;
    blendOpac = (_e114 * _e115.w);
    let _e119 = overUv;
    let _e126 = insideBox(_e119, vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    if (_e126 > 0.5f) {
        {
            let _e129 = global.blendModeIndex;
            if (_e129 == 0f) {
                {
                    let _e132 = base_34;
                    let _e134 = blend_34;
                    let _e136 = blendOpac;
                    let _e137 = blendNormal_1(_e132.xyz, _e134.xyz, _e136);
                    outCol = _e137;
                }
            } else {
                let _e138 = global.blendModeIndex;
                if (_e138 == 1f) {
                    {
                        let _e141 = base_34;
                        let _e143 = blend_34;
                        let _e145 = blendOpac;
                        let _e146 = blendAdd_1(_e141.xyz, _e143.xyz, _e145);
                        outCol = _e146;
                    }
                } else {
                    let _e147 = global.blendModeIndex;
                    if (_e147 == 2f) {
                        {
                            let _e150 = base_34;
                            let _e152 = blend_34;
                            let _e154 = blendOpac;
                            let _e155 = blendMultiply_1(_e150.xyz, _e152.xyz, _e154);
                            outCol = _e155;
                        }
                    } else {
                        let _e156 = global.blendModeIndex;
                        if (_e156 == 3f) {
                            {
                                let _e159 = base_34;
                                let _e161 = blend_34;
                                let _e163 = blendOpac;
                                let _e164 = blendScreen_2(_e159.xyz, _e161.xyz, _e163);
                                outCol = _e164;
                            }
                        } else {
                            let _e165 = global.blendModeIndex;
                            if (_e165 == 4f) {
                                {
                                    let _e168 = base_34;
                                    let _e170 = blend_34;
                                    let _e172 = blendOpac;
                                    let _e173 = blendSoftLight_2(_e168.xyz, _e170.xyz, _e172);
                                    outCol = _e173;
                                }
                            } else {
                                let _e174 = global.blendModeIndex;
                                if (_e174 == 5f) {
                                    {
                                        let _e177 = base_34;
                                        let _e179 = blend_34;
                                        let _e181 = blendOpac;
                                        let _e182 = blendDifference_1(_e177.xyz, _e179.xyz, _e181);
                                        outCol = _e182;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let _e183 = blendOpac;
    let _e184 = base_34;
    outOpac = (_e183 + _e184.w);
    let _e188 = outCol;
    let _e189 = outOpac;
    o_color = vec4<f32>(_e188.x, _e188.y, _e188.z, _e189);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e37 = o_color;
    return FragmentOutput(_e37);
}
