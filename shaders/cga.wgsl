struct U {
    resolution: vec2<f32>,
    colLight: vec3<f32>,
    colDark: vec3<f32>,
    colWhite: vec3<f32>,
    scale: f32,
    amount: f32,
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
var<private> gl_FragCoord_1: vec4<f32>;

fn main_1() {
    var orig: vec4<f32>;
    var size: f32;
    var dSize: f32;
    var amount2_: f32;
    var d: f32;
    var ar: f32;
    var sx: f32;
    var sy: f32;
    var base: vec4<f32>;
    var lum: f32;
    var o: f32;
    var c1_: vec3<f32>;
    var c2_: vec3<f32>;
    var black: vec3<f32> = vec3(0f);
    var color: vec4<f32>;

    let _e16 = vUv_1;
    let _e17 = textureSample(tDiffuse_t, tDiffuse_s, _e16);
    orig = _e17;
    let _e19 = global.scale;
    size = _e19;
    let _e22 = size;
    dSize = (2f * _e22);
    let _e25 = global.resolution;
    let _e27 = size;
    amount2_ = (_e25.x / _e27);
    let _e31 = amount2_;
    d = (1f / _e31);
    let _e34 = global.resolution;
    let _e36 = global.resolution;
    ar = (_e34.x / _e36.y);
    let _e40 = vUv_1;
    let _e42 = d;
    let _e45 = d;
    sx = (floor((_e40.x / _e42)) * _e45);
    let _e48 = ar;
    let _e49 = amount2_;
    d = (_e48 / _e49);
    let _e51 = vUv_1;
    let _e53 = d;
    let _e56 = d;
    sy = (floor((_e51.y / _e53)) * _e56);
    let _e59 = sx;
    let _e60 = sy;
    let _e62 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(_e59, _e60));
    base = _e62;
    let _e65 = base;
    let _e69 = base;
    let _e74 = base;
    lum = (((0.2126f * _e65.x) + (0.7152f * _e69.y)) + (0.0722f * _e74.z));
    let _e80 = lum;
    o = floor((6f * _e80));
    let _e89 = o;
    if (_e89 == 0f) {
        {
            let _e92 = black;
            c1_ = _e92;
            let _e93 = c1_;
            c2_ = _e93;
        }
    }
    let _e94 = o;
    if (_e94 == 1f) {
        {
            let _e97 = black;
            c1_ = _e97;
            let _e98 = global.colDark;
            c2_ = _e98;
        }
    }
    let _e99 = o;
    if (_e99 == 2f) {
        {
            let _e102 = global.colDark;
            c1_ = _e102;
            let _e103 = c1_;
            c2_ = _e103;
        }
    }
    let _e104 = o;
    if (_e104 == 3f) {
        {
            let _e107 = global.colDark;
            c1_ = _e107;
            let _e108 = global.colLight;
            c2_ = _e108;
        }
    }
    let _e109 = o;
    if (_e109 == 4f) {
        {
            let _e112 = global.colLight;
            c1_ = _e112;
            let _e113 = c1_;
            c2_ = _e113;
        }
    }
    let _e114 = o;
    if (_e114 == 5f) {
        {
            let _e117 = global.colLight;
            c1_ = _e117;
            let _e118 = global.colWhite;
            c2_ = _e118;
        }
    }
    let _e119 = o;
    if (_e119 == 6f) {
        {
            let _e122 = global.colWhite;
            c1_ = _e122;
            let _e123 = c1_;
            c2_ = _e123;
        }
    }
    let _e125 = gl_FragCoord_1;
    let _e127 = dSize;
    let _e132 = size;
    if ((_e125.x - (floor((_e125.x / _e127)) * _e127)) > _e132) {
        {
            let _e134 = gl_FragCoord_1;
            let _e136 = dSize;
            let _e141 = size;
            if ((_e134.y - (floor((_e134.y / _e136)) * _e136)) > _e141) {
                {
                    let _e143 = base;
                    let _e145 = c1_;
                    base.x = _e145.x;
                    base.y = _e145.y;
                    base.z = _e145.z;
                }
            } else {
                {
                    let _e152 = base;
                    let _e154 = c2_;
                    base.x = _e154.x;
                    base.y = _e154.y;
                    base.z = _e154.z;
                }
            }
        }
    } else {
        {
            let _e161 = gl_FragCoord_1;
            let _e163 = dSize;
            let _e168 = size;
            if ((_e161.y - (floor((_e161.y / _e163)) * _e163)) > _e168) {
                {
                    let _e170 = base;
                    let _e172 = c2_;
                    base.x = _e172.x;
                    base.y = _e172.y;
                    base.z = _e172.z;
                }
            } else {
                {
                    let _e179 = base;
                    let _e181 = c1_;
                    base.x = _e181.x;
                    base.y = _e181.y;
                    base.z = _e181.z;
                }
            }
        }
    }
    let _e188 = orig;
    let _e189 = base;
    let _e190 = global.amount;
    color = mix(_e188, _e189, vec4(_e190));
    let _e194 = color;
    o_color = _e194;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>, @builtin(position) gl_FragCoord: vec4<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    gl_FragCoord_1 = gl_FragCoord;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
