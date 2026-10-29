struct U {
    scale: f32,
    resolution: vec2<f32>,
    amount: f32,
    b_cmyk: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const ANGLE_C: f32 = 15f;
const ANGLE_M: f32 = 75f;
const ANGLE_Y: f32 = 0f;
const ANGLE_K: f32 = 45f;
const black: vec3<f32> = vec3(0f);

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e21 = theta_1;
    let _e23 = theta_1;
    let _e26 = theta_1;
    let _e28 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e21), -(sin(_e23))), vec2<f32>(sin(_e26), cos(_e28)));
    let _e34 = m;
    let _e35 = position_1;
    return (_e34 * _e35);
}

fn pattern(ang: f32) -> f32 {
    var ang_1: f32;
    var uv: vec2<f32>;

    ang_1 = ang;
    let _e19 = vUv_1;
    let _e20 = global.scale;
    let _e22 = global.resolution;
    uv = (((_e19 * _e20) * _e22) * 1.3f);
    let _e27 = uv;
    uv = (_e27 - vec2(0.5f));
    let _e31 = uv;
    let _e32 = ang_1;
    let _e37 = rotate2d(_e31, ((_e32 * 3.14159f) / 180f));
    uv = _e37;
    let _e38 = uv;
    uv = (_e38 + vec2(0.5f));
    let _e42 = uv;
    let _e45 = uv;
    return ((sin(_e42.x) * sin(_e45.y)) * 4f);
}

fn halftone(ang_2: f32, col: f32) -> f32 {
    var ang_3: f32;
    var col_1: f32;
    var pt: f32;
    var ht: f32;

    ang_3 = ang_2;
    col_1 = col;
    let _e21 = ang_3;
    let _e22 = pattern(_e21);
    pt = _e22;
    let _e24 = col_1;
    let _e30 = pt;
    ht = (((_e24 * 10f) - 5f) + (4f - _e30));
    let _e34 = ht;
    return _e34;
}

fn main_1() {
    var orig: vec4<f32>;
    var color: vec3<f32>;
    var cmyk: vec4<f32>;
    var hc: f32;
    var hm: f32;
    var hy: f32;
    var hk: f32;
    var rgbscreen: vec3<f32>;
    var ave: f32;

    let _e17 = vUv_1;
    let _e18 = textureSample(tDiffuse_t, tDiffuse_s, _e17);
    orig = _e18;
    let _e21 = global.b_cmyk;
    if (_e21 > 0.5f) {
        {
            let _e25 = cmyk;
            let _e28 = orig;
            let _e31 = (vec3(1f) - _e28.xyz);
            cmyk.x = _e31.x;
            cmyk.y = _e31.y;
            cmyk.z = _e31.z;
            let _e39 = cmyk;
            let _e41 = cmyk;
            let _e43 = cmyk;
            cmyk.w = min(_e39.x, min(_e41.y, _e43.z));
            let _e47 = cmyk;
            let _e49 = cmyk;
            let _e51 = cmyk;
            let _e54 = (_e49.xyz - vec3(_e51.w));
            cmyk.x = _e54.x;
            cmyk.y = _e54.y;
            cmyk.z = _e54.z;
            let _e61 = cmyk;
            let _e63 = halftone(ANGLE_C, _e61.x);
            hc = _e63;
            let _e65 = cmyk;
            let _e67 = halftone(ANGLE_M, _e65.y);
            hm = _e67;
            let _e69 = cmyk;
            let _e71 = halftone(ANGLE_Y, _e69.z);
            hy = _e71;
            let _e73 = cmyk;
            let _e75 = halftone(ANGLE_K, _e73.w);
            hk = _e75;
            let _e79 = hc;
            let _e80 = hm;
            let _e81 = hy;
            rgbscreen = (vec3(1f) - (0.2f * vec3<f32>(_e79, _e80, _e81)));
            let _e87 = rgbscreen;
            let _e89 = hk;
            color = mix(_e87, black, vec3((0.16f * _e89)));
        }
    } else {
        {
            let _e93 = orig;
            let _e95 = orig;
            let _e98 = orig;
            ave = (((_e93.x + _e95.y) + _e98.z) / 3f);
            let _e104 = ave;
            let _e110 = pattern(0f);
            color = vec3((((_e104 * 10f) - 5f) + _e110));
        }
    }
    let _e113 = orig;
    let _e115 = color;
    let _e116 = global.amount;
    color = mix(_e113.xyz, _e115, vec3(_e116));
    let _e119 = color;
    let _e120 = orig;
    o_color = vec4<f32>(_e119.x, _e119.y, _e119.z, _e120.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e29 = o_color;
    return FragmentOutput(_e29);
}
