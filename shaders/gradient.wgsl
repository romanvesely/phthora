struct U {
    numStops: i32,
    color1_: vec3<f32>,
    color2_: vec3<f32>,
    color3_: vec3<f32>,
    color4_: vec3<f32>,
    color5_: vec3<f32>,
    color6_: vec3<f32>,
    stop1_: f32,
    stop2_: f32,
    stop3_: f32,
    stop4_: f32,
    stop5_: f32,
    stop6_: f32,
    amount: f32,
    angle: f32,
    position: f32,
    scale: f32,
    b_radial: f32,
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
    let _e46 = blend_1;
    return _e46;
}

fn blendNormal_1(base_2: vec3<f32>, blend_2: vec3<f32>, opacity: f32) -> vec3<f32> {
    var base_3: vec3<f32>;
    var blend_3: vec3<f32>;
    var opacity_1: f32;

    base_3 = base_2;
    blend_3 = blend_2;
    opacity_1 = opacity;
    let _e48 = base_3;
    let _e49 = blend_3;
    let _e50 = blendNormal(_e48, _e49);
    let _e51 = opacity_1;
    let _e53 = base_3;
    let _e55 = opacity_1;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn blendMultiply(base_4: vec3<f32>, blend_4: vec3<f32>) -> vec3<f32> {
    var base_5: vec3<f32>;
    var blend_5: vec3<f32>;

    base_5 = base_4;
    blend_5 = blend_4;
    let _e46 = base_5;
    let _e47 = blend_5;
    return (_e46 * _e47);
}

fn blendMultiply_1(base_6: vec3<f32>, blend_6: vec3<f32>, opacity_2: f32) -> vec3<f32> {
    var base_7: vec3<f32>;
    var blend_7: vec3<f32>;
    var opacity_3: f32;

    base_7 = base_6;
    blend_7 = blend_6;
    opacity_3 = opacity_2;
    let _e48 = base_7;
    let _e49 = blend_7;
    let _e50 = blendMultiply(_e48, _e49);
    let _e51 = opacity_3;
    let _e53 = base_7;
    let _e55 = opacity_3;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn blendAdd(base_8: vec3<f32>, blend_8: vec3<f32>) -> vec3<f32> {
    var base_9: vec3<f32>;
    var blend_9: vec3<f32>;

    base_9 = base_8;
    blend_9 = blend_8;
    let _e46 = base_9;
    let _e47 = blend_9;
    return min((_e46 + _e47), vec3(1f));
}

fn blendAdd_1(base_10: vec3<f32>, blend_10: vec3<f32>, opacity_4: f32) -> vec3<f32> {
    var base_11: vec3<f32>;
    var blend_11: vec3<f32>;
    var opacity_5: f32;

    base_11 = base_10;
    blend_11 = blend_10;
    opacity_5 = opacity_4;
    let _e48 = base_11;
    let _e49 = blend_11;
    let _e50 = blendAdd(_e48, _e49);
    let _e51 = opacity_5;
    let _e53 = base_11;
    let _e55 = opacity_5;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn blendScreen(base_12: f32, blend_12: f32) -> f32 {
    var base_13: f32;
    var blend_13: f32;

    base_13 = base_12;
    blend_13 = blend_12;
    let _e48 = base_13;
    let _e51 = blend_13;
    return (1f - ((1f - _e48) * (1f - _e51)));
}

fn blendScreen_1(base_14: vec3<f32>, blend_14: vec3<f32>) -> vec3<f32> {
    var base_15: vec3<f32>;
    var blend_15: vec3<f32>;

    base_15 = base_14;
    blend_15 = blend_14;
    let _e46 = base_15;
    let _e48 = blend_15;
    let _e50 = blendScreen(_e46.x, _e48.x);
    let _e51 = base_15;
    let _e53 = blend_15;
    let _e55 = blendScreen(_e51.y, _e53.y);
    let _e56 = base_15;
    let _e58 = blend_15;
    let _e60 = blendScreen(_e56.z, _e58.z);
    return vec3<f32>(_e50, _e55, _e60);
}

fn blendScreen_2(base_16: vec3<f32>, blend_16: vec3<f32>, opacity_6: f32) -> vec3<f32> {
    var base_17: vec3<f32>;
    var blend_17: vec3<f32>;
    var opacity_7: f32;

    base_17 = base_16;
    blend_17 = blend_16;
    opacity_7 = opacity_6;
    let _e48 = base_17;
    let _e49 = blend_17;
    let _e50 = blendScreen_1(_e48, _e49);
    let _e51 = opacity_7;
    let _e53 = base_17;
    let _e55 = opacity_7;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn blendSoftLight(base_18: f32, blend_18: f32) -> f32 {
    var base_19: f32;
    var blend_19: f32;
    var local: f32;

    base_19 = base_18;
    blend_19 = blend_18;
    let _e46 = blend_19;
    if (_e46 < 0.5f) {
        let _e50 = base_19;
        let _e52 = blend_19;
        let _e54 = base_19;
        let _e55 = base_19;
        let _e59 = blend_19;
        local = (((2f * _e50) * _e52) + ((_e54 * _e55) * (1f - (2f * _e59))));
    } else {
        let _e64 = base_19;
        let _e67 = blend_19;
        let _e73 = base_19;
        let _e76 = blend_19;
        local = ((sqrt(_e64) * ((2f * _e67) - 1f)) + ((2f * _e73) * (1f - _e76)));
    }
    let _e81 = local;
    return _e81;
}

fn blendSoftLight_1(base_20: vec3<f32>, blend_20: vec3<f32>) -> vec3<f32> {
    var base_21: vec3<f32>;
    var blend_21: vec3<f32>;

    base_21 = base_20;
    blend_21 = blend_20;
    let _e46 = base_21;
    let _e48 = blend_21;
    let _e50 = blendSoftLight(_e46.x, _e48.x);
    let _e51 = base_21;
    let _e53 = blend_21;
    let _e55 = blendSoftLight(_e51.y, _e53.y);
    let _e56 = base_21;
    let _e58 = blend_21;
    let _e60 = blendSoftLight(_e56.z, _e58.z);
    return vec3<f32>(_e50, _e55, _e60);
}

fn blendSoftLight_2(base_22: vec3<f32>, blend_22: vec3<f32>, opacity_8: f32) -> vec3<f32> {
    var base_23: vec3<f32>;
    var blend_23: vec3<f32>;
    var opacity_9: f32;

    base_23 = base_22;
    blend_23 = blend_22;
    opacity_9 = opacity_8;
    let _e48 = base_23;
    let _e49 = blend_23;
    let _e50 = blendSoftLight_1(_e48, _e49);
    let _e51 = opacity_9;
    let _e53 = base_23;
    let _e55 = opacity_9;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn blendDifference(base_24: vec3<f32>, blend_24: vec3<f32>) -> vec3<f32> {
    var base_25: vec3<f32>;
    var blend_25: vec3<f32>;

    base_25 = base_24;
    blend_25 = blend_24;
    let _e46 = base_25;
    let _e47 = blend_25;
    return abs((_e46 - _e47));
}

fn blendDifference_1(base_26: vec3<f32>, blend_26: vec3<f32>, opacity_10: f32) -> vec3<f32> {
    var base_27: vec3<f32>;
    var blend_27: vec3<f32>;
    var opacity_11: f32;

    base_27 = base_26;
    blend_27 = blend_26;
    opacity_11 = opacity_10;
    let _e48 = base_27;
    let _e49 = blend_27;
    let _e50 = blendDifference(_e48, _e49);
    let _e51 = opacity_11;
    let _e53 = base_27;
    let _e55 = opacity_11;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn blendOverlay(base_28: f32, blend_28: f32) -> f32 {
    var base_29: f32;
    var blend_29: f32;
    var local_1: f32;

    base_29 = base_28;
    blend_29 = blend_28;
    let _e46 = base_29;
    if (_e46 < 0.5f) {
        let _e50 = base_29;
        let _e52 = blend_29;
        local_1 = ((2f * _e50) * _e52);
    } else {
        let _e57 = base_29;
        let _e61 = blend_29;
        local_1 = (1f - ((2f * (1f - _e57)) * (1f - _e61)));
    }
    let _e66 = local_1;
    return _e66;
}

fn blendOverlay_1(base_30: vec3<f32>, blend_30: vec3<f32>) -> vec3<f32> {
    var base_31: vec3<f32>;
    var blend_31: vec3<f32>;

    base_31 = base_30;
    blend_31 = blend_30;
    let _e46 = base_31;
    let _e48 = blend_31;
    let _e50 = blendOverlay(_e46.x, _e48.x);
    let _e51 = base_31;
    let _e53 = blend_31;
    let _e55 = blendOverlay(_e51.y, _e53.y);
    let _e56 = base_31;
    let _e58 = blend_31;
    let _e60 = blendOverlay(_e56.z, _e58.z);
    return vec3<f32>(_e50, _e55, _e60);
}

fn blendOverlay_2(base_32: vec3<f32>, blend_32: vec3<f32>, opacity_12: f32) -> vec3<f32> {
    var base_33: vec3<f32>;
    var blend_33: vec3<f32>;
    var opacity_13: f32;

    base_33 = base_32;
    blend_33 = blend_32;
    opacity_13 = opacity_12;
    let _e48 = base_33;
    let _e49 = blend_33;
    let _e50 = blendOverlay_1(_e48, _e49);
    let _e51 = opacity_13;
    let _e53 = base_33;
    let _e55 = opacity_13;
    return ((_e50 * _e51) + (_e53 * (1f - _e55)));
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e46 = theta_1;
    let _e48 = theta_1;
    let _e51 = theta_1;
    let _e53 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e46), -(sin(_e48))), vec2<f32>(sin(_e51), cos(_e53)));
    let _e59 = m;
    let _e60 = position_1;
    return (_e59 * _e60);
}

fn getColor(i: i32) -> vec3<f32> {
    var i_1: i32;

    i_1 = i;
    let _e44 = i_1;
    if (_e44 == 0i) {
        let _e47 = global.color1_;
        return _e47;
    }
    let _e48 = i_1;
    if (_e48 == 1i) {
        let _e51 = global.color2_;
        return _e51;
    }
    let _e52 = i_1;
    if (_e52 == 2i) {
        let _e55 = global.color3_;
        return _e55;
    }
    let _e56 = i_1;
    if (_e56 == 3i) {
        let _e59 = global.color4_;
        return _e59;
    }
    let _e60 = i_1;
    if (_e60 == 4i) {
        let _e63 = global.color5_;
        return _e63;
    }
    let _e64 = global.color6_;
    return _e64;
}

fn getStop(i_2: i32) -> f32 {
    var i_3: i32;

    i_3 = i_2;
    let _e44 = i_3;
    if (_e44 == 0i) {
        let _e47 = global.stop1_;
        return _e47;
    }
    let _e48 = i_3;
    if (_e48 == 1i) {
        let _e51 = global.stop2_;
        return _e51;
    }
    let _e52 = i_3;
    if (_e52 == 2i) {
        let _e55 = global.stop3_;
        return _e55;
    }
    let _e56 = i_3;
    if (_e56 == 3i) {
        let _e59 = global.stop4_;
        return _e59;
    }
    let _e60 = i_3;
    if (_e60 == 4i) {
        let _e63 = global.stop5_;
        return _e63;
    }
    let _e64 = global.stop6_;
    return _e64;
}

fn gradient(t: f32) -> vec3<f32> {
    var t_1: f32;
    var i_4: i32 = 0i;
    var currentStop: f32;
    var nextStop: f32;
    var currentColor: vec3<f32>;
    var nextColor: vec3<f32>;

    t_1 = t;
    loop {
        let _e46 = i_4;
        if !((_e46 < 5i)) {
            break;
        }
        {
            let _e53 = i_4;
            let _e54 = global.numStops;
            if (_e53 >= (_e54 - 1i)) {
                break;
            }
            let _e58 = i_4;
            let _e59 = getStop(_e58);
            currentStop = _e59;
            let _e61 = i_4;
            let _e64 = getStop((_e61 + 1i));
            nextStop = _e64;
            let _e66 = t_1;
            let _e67 = nextStop;
            let _e69 = i_4;
            let _e70 = global.numStops;
            if ((_e66 <= _e67) || (_e69 == (_e70 - 2i))) {
                {
                    let _e75 = i_4;
                    let _e76 = getColor(_e75);
                    currentColor = _e76;
                    let _e78 = i_4;
                    let _e81 = getColor((_e78 + 1i));
                    nextColor = _e81;
                    let _e83 = currentColor;
                    let _e84 = nextColor;
                    let _e85 = currentStop;
                    let _e86 = nextStop;
                    let _e87 = t_1;
                    return mix(_e83, _e84, vec3(smoothstep(_e85, _e86, _e87)));
                }
            }
        }
        continuing {
            let _e50 = i_4;
            i_4 = (_e50 + 1i);
        }
    }
    let _e91 = global.numStops;
    let _e94 = getColor((_e91 - 1i));
    return _e94;
}

fn main_1() {
    var orig: vec4<f32>;
    var uv: vec2<f32>;
    var t_2: f32;
    var gradColor: vec3<f32>;
    var outCol: vec3<f32>;
    var amountCapped: f32;
    var outOpac: f32;

    let _e42 = vUv_1;
    let _e43 = textureSample(tDiffuse_t, tDiffuse_s, _e42);
    orig = _e43;
    let _e45 = vUv_1;
    uv = (_e45 - vec2(0.5f));
    let _e50 = uv;
    let _e51 = global.angle;
    let _e54 = rotate2d(_e50, (_e51 * 6.2831855f));
    uv = _e54;
    let _e56 = global.b_radial;
    if (_e56 > 0.5f) {
        {
            let _e59 = uv;
            t_2 = (length(_e59) * 1.414f);
        }
    } else {
        {
            let _e63 = uv;
            t_2 = (_e63.x + 0.5f);
        }
    }
    let _e67 = t_2;
    let _e68 = global.scale;
    t_2 = (_e67 * _e68);
    let _e70 = t_2;
    let _e71 = global.position;
    t_2 = (_e70 + _e71);
    let _e73 = t_2;
    t_2 = fract(_e73);
    let _e75 = t_2;
    let _e76 = gradient(_e75);
    gradColor = _e76;
    let _e79 = global.amount;
    amountCapped = clamp(_e79, 0f, 1f);
    let _e84 = global.blendModeIndex;
    if (_e84 == 0f) {
        {
            let _e87 = orig;
            let _e89 = gradColor;
            let _e90 = amountCapped;
            let _e91 = blendNormal_1(_e87.xyz, _e89, _e90);
            outCol = _e91;
        }
    } else {
        let _e92 = global.blendModeIndex;
        if (_e92 == 1f) {
            {
                let _e95 = orig;
                let _e97 = gradColor;
                let _e98 = amountCapped;
                let _e99 = blendAdd_1(_e95.xyz, _e97, _e98);
                outCol = _e99;
            }
        } else {
            let _e100 = global.blendModeIndex;
            if (_e100 == 2f) {
                {
                    let _e103 = orig;
                    let _e105 = gradColor;
                    let _e106 = amountCapped;
                    let _e107 = blendMultiply_1(_e103.xyz, _e105, _e106);
                    outCol = _e107;
                }
            } else {
                let _e108 = global.blendModeIndex;
                if (_e108 == 3f) {
                    {
                        let _e111 = orig;
                        let _e113 = gradColor;
                        let _e114 = amountCapped;
                        let _e115 = blendScreen_2(_e111.xyz, _e113, _e114);
                        outCol = _e115;
                    }
                } else {
                    let _e116 = global.blendModeIndex;
                    if (_e116 == 4f) {
                        {
                            let _e119 = orig;
                            let _e121 = gradColor;
                            let _e122 = amountCapped;
                            let _e123 = blendSoftLight_2(_e119.xyz, _e121, _e122);
                            outCol = _e123;
                        }
                    } else {
                        let _e124 = global.blendModeIndex;
                        if (_e124 == 5f) {
                            {
                                let _e127 = orig;
                                let _e129 = gradColor;
                                let _e130 = amountCapped;
                                let _e131 = blendDifference_1(_e127.xyz, _e129, _e130);
                                outCol = _e131;
                            }
                        }
                    }
                }
            }
        }
    }
    let _e132 = amountCapped;
    let _e133 = orig;
    outOpac = (_e132 + _e133.w);
    let _e137 = outCol;
    let _e138 = outOpac;
    o_color = vec4<f32>(_e137.x, _e137.y, _e137.z, _e138);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e49 = o_color;
    return FragmentOutput(_e49);
}
