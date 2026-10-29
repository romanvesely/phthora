struct U {
    centerBrightness: f32,
    powerCurve: f32,
    colorize: f32,
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

fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    var c_1: vec3<f32>;
    var K: vec4<f32> = vec4<f32>(0f, -0.33333334f, 0.6666667f, -1f);
    var local: vec4<f32>;
    var p: vec4<f32>;
    var local_1: vec4<f32>;
    var q: vec4<f32>;
    var d: f32;
    var e: f32 = 0.0000000001f;

    c_1 = c;
    let _e26 = c_1;
    let _e28 = c_1;
    if (_e26.y < _e28.z) {
        let _e31 = c_1;
        let _e32 = _e31.zy;
        let _e33 = K;
        let _e34 = _e33.wz;
        local = vec4<f32>(_e32.x, _e32.y, _e34.x, _e34.y);
    } else {
        let _e40 = c_1;
        let _e41 = _e40.yz;
        let _e42 = K;
        let _e43 = _e42.xy;
        local = vec4<f32>(_e41.x, _e41.y, _e43.x, _e43.y);
    }
    let _e50 = local;
    p = _e50;
    let _e52 = c_1;
    let _e54 = p;
    if (_e52.x < _e54.x) {
        let _e57 = p;
        let _e58 = _e57.xyw;
        let _e59 = c_1;
        local_1 = vec4<f32>(_e58.x, _e58.y, _e58.z, _e59.x);
    } else {
        let _e65 = c_1;
        let _e67 = p;
        let _e68 = _e67.yzx;
        local_1 = vec4<f32>(_e65.x, _e68.x, _e68.y, _e68.z);
    }
    let _e74 = local_1;
    q = _e74;
    let _e76 = q;
    let _e78 = q;
    let _e80 = q;
    d = (_e76.x - min(_e78.w, _e80.y));
    let _e87 = q;
    let _e89 = q;
    let _e91 = q;
    let _e95 = d;
    let _e97 = e;
    let _e102 = d;
    let _e103 = q;
    let _e105 = e;
    let _e108 = q;
    return vec3<f32>(abs((_e87.z + ((_e89.w - _e91.y) / ((6f * _e95) + _e97)))), (_e102 / (_e103.x + _e105)), _e108.x);
}

fn hsv2rgb(c_2: vec3<f32>) -> vec3<f32> {
    var c_3: vec3<f32>;
    var K_1: vec4<f32> = vec4<f32>(1f, 0.6666667f, 0.33333334f, 3f);
    var p_1: vec3<f32>;

    c_3 = c_2;
    let _e24 = c_3;
    let _e26 = K_1;
    let _e32 = K_1;
    p_1 = abs(((fract((_e24.xxx + _e26.xyz)) * 6f) - _e32.www));
    let _e37 = c_3;
    let _e39 = K_1;
    let _e41 = p_1;
    let _e42 = K_1;
    let _e50 = c_3;
    return (_e37.z * mix(_e39.xxx, clamp((_e41 - _e42.xxx), vec3(0f), vec3(1f)), vec3(_e50.y)));
}

fn main_1() {
    var orig: vec4<f32>;
    var hslColor: vec3<f32>;
    var outColor: vec3<f32>;
    var local_2: f32;
    var color: vec4<f32>;

    let _e12 = vUv_1;
    let _e13 = textureSample(tDiffuse_t, tDiffuse_s, _e12);
    orig = _e13;
    let _e15 = orig;
    let _e17 = rgb2hsv(_e15.xyz);
    hslColor = _e17;
    let _e19 = hslColor;
    outColor = _e19;
    let _e22 = outColor;
    let _e24 = global.powerCurve;
    outColor.z = pow(_e22.z, _e24);
    let _e27 = outColor;
    let _e29 = global.centerBrightness;
    if (_e27.z < _e29) {
        let _e32 = outColor;
        let _e34 = global.centerBrightness;
        local_2 = (1f - (_e32.z / _e34));
    } else {
        let _e37 = outColor;
        let _e39 = global.centerBrightness;
        let _e41 = global.centerBrightness;
        local_2 = ((_e37.z - _e39) / _e41);
    }
    let _e44 = local_2;
    outColor.z = _e44;
    let _e46 = outColor;
    let _e48 = hslColor;
    let _e51 = global.colorize;
    outColor.y = ((_e46.y * _e48.z) * _e51);
    let _e53 = outColor;
    let _e54 = hsv2rgb(_e53);
    outColor = _e54;
    let _e55 = outColor;
    let _e56 = orig;
    color = vec4<f32>(_e55.x, _e55.y, _e55.z, _e56.w);
    let _e63 = orig;
    let _e64 = color;
    let _e65 = global.amount;
    color = mix(_e63, _e64, vec4(_e65));
    let _e68 = color;
    o_color = _e68;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
