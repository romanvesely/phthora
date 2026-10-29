struct U {
    amount: f32,
    resolution: vec2<f32>,
    b_invert: f32,
    b_concave: f32,
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
var<private> amountZoom: f32 = 0.5f;

fn remap(t: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    var t_1: vec2<f32>;
    var a_1: vec2<f32>;
    var b_1: vec2<f32>;

    t_1 = t;
    a_1 = a;
    b_1 = b;
    let _e19 = t_1;
    let _e20 = a_1;
    let _e22 = b_1;
    let _e23 = a_1;
    return clamp(((_e19 - _e20) / (_e22 - _e23)), vec2(0f), vec2(1f));
}

fn spectrum_offset_rgb(t_2: f32) -> vec3<f32> {
    var t_3: f32;
    var t0_: f32;
    var ret: vec3<f32>;

    t_3 = t_2;
    let _e16 = t_3;
    t0_ = ((3f * _e16) - 1.5f);
    let _e21 = t0_;
    let _e24 = t0_;
    let _e27 = t0_;
    ret = clamp(vec3<f32>(-(_e21), (1f - abs(_e24)), _e27), vec3(0f), vec3(1f));
    let _e35 = ret;
    return _e35;
}

fn barrelDistortion(p: vec2<f32>, amt: vec2<f32>) -> vec2<f32> {
    var p_1: vec2<f32>;
    var amt_1: vec2<f32>;
    var maxBarrelPower: f32 = 5f;
    var theta: f32;
    var radius: vec2<f32>;

    p_1 = p;
    amt_1 = amt;
    let _e18 = p_1;
    p_1 = ((2f * _e18) - vec2(1f));
    let _e25 = p_1;
    let _e27 = p_1;
    theta = atan2(_e25.y, _e27.x);
    let _e31 = p_1;
    radius = vec2(length(_e31));
    let _e35 = radius;
    let _e37 = maxBarrelPower;
    let _e38 = amt_1;
    radius = pow(_e35, (vec2(1f) + (_e37 * _e38)));
    let _e44 = radius;
    let _e46 = theta;
    p_1.x = (_e44.x * cos(_e46));
    let _e50 = radius;
    let _e52 = theta;
    p_1.y = (_e50.y * sin(_e52));
    let _e55 = p_1;
    return ((_e55 * 0.5f) + vec2(0.5f));
}

fn brownConradyDistortion(uv: vec2<f32>, dist: f32) -> vec2<f32> {
    var uv_1: vec2<f32>;
    var dist_1: f32;
    var barrelDistortion1_: f32;
    var barrelDistortion2_: f32;
    var r2_: f32;

    uv_1 = uv;
    dist_1 = dist;
    let _e17 = uv_1;
    uv_1 = ((_e17 * 2f) - vec2(1f));
    let _e24 = dist_1;
    barrelDistortion1_ = (0.1f * _e24);
    let _e29 = dist_1;
    barrelDistortion2_ = (-0.025f * _e29);
    let _e32 = uv_1;
    let _e33 = uv_1;
    r2_ = dot(_e32, _e33);
    let _e36 = uv_1;
    let _e38 = barrelDistortion1_;
    let _e39 = r2_;
    let _e42 = barrelDistortion2_;
    let _e43 = r2_;
    let _e45 = r2_;
    uv_1 = (_e36 * ((1f + (_e38 * _e39)) + ((_e42 * _e43) * _e45)));
    let _e49 = uv_1;
    return ((_e49 * 0.5f) + vec2(0.5f));
}

fn distort(uv_2: vec2<f32>, t_4: f32, min_distort: vec2<f32>, max_distort: vec2<f32>) -> vec2<f32> {
    var uv_3: vec2<f32>;
    var t_5: f32;
    var min_distort_1: vec2<f32>;
    var max_distort_1: vec2<f32>;
    var dist_2: vec2<f32>;

    uv_3 = uv_2;
    t_5 = t_4;
    min_distort_1 = min_distort;
    max_distort_1 = max_distort;
    let _e21 = min_distort_1;
    let _e22 = max_distort_1;
    let _e23 = t_5;
    dist_2 = mix(_e21, _e22, vec2(_e23));
    let _e27 = global.b_invert;
    if (_e27 > 0.5f) {
        {
            let _e30 = uv_3;
            let _e32 = dist_2;
            let _e34 = barrelDistortion(_e30, (1f * _e32));
            return _e34;
        }
    } else {
        {
            let _e35 = uv_3;
            let _e37 = dist_2;
            let _e40 = brownConradyDistortion(_e35, (75f * _e37.x));
            return _e40;
        }
    }
}

fn nrand(n: vec2<f32>) -> f32 {
    var n_1: vec2<f32>;

    n_1 = n;
    let _e15 = n_1;
    return fract((sin(dot(_e15.xy, vec2<f32>(12.9898f, 78.233f))) * 43758.547f));
}

fn render(uv_4: vec2<f32>) -> vec4<f32> {
    var uv_5: vec2<f32>;

    uv_5 = uv_4;
    let _e15 = uv_5;
    let _e16 = textureSample(tDiffuse_t, tDiffuse_s, _e15);
    return _e16;
}

fn main_1() {
    var uv_6: vec2<f32>;
    var amount2_: f32;
    var MAX_DIST_PX: f32 = 50f;
    var max_distort_px: f32;
    var max_distort_2: vec2<f32>;
    var min_distort_2: vec2<f32>;
    var oversiz: vec2<f32>;
    var num_iter: i32 = 12i;
    var stepsiz: f32;
    var rnd: f32;
    var t_6: f32;
    var sumcol: vec4<f32> = vec4(0f);
    var sumw: vec3<f32> = vec3(0f);
    var i: i32 = 0i;
    var w: vec3<f32>;
    var uvd: vec2<f32>;
    var col: vec4<f32>;

    let _e13 = vUv_1;
    uv_6 = _e13;
    let _e15 = global.amount;
    amount2_ = _e15;
    let _e17 = global.b_concave;
    if (_e17 > 0.5f) {
        {
            let _e20 = amount2_;
            amount2_ = -(_e20);
        }
    } else {
        {
            let _e22 = amount2_;
            amount2_ = (_e22 * 3f);
        }
    }
    let _e27 = MAX_DIST_PX;
    let _e28 = amount2_;
    max_distort_px = (_e27 * _e28);
    let _e31 = max_distort_px;
    let _e33 = global.resolution;
    max_distort_2 = (vec2(_e31) / _e33);
    let _e37 = max_distort_2;
    min_distort_2 = (0.5f * _e37);
    let _e43 = min_distort_2;
    let _e44 = max_distort_2;
    let _e45 = distort(vec2(1f), 1f, _e43, _e44);
    oversiz = _e45;
    let _e47 = uv_6;
    let _e48 = uv_6;
    let _e50 = oversiz;
    let _e53 = oversiz;
    let _e54 = remap(_e48, (vec2(1f) - _e50), _e53);
    let _e55 = amountZoom;
    uv_6 = mix(_e47, _e54, vec2(_e55));
    let _e61 = num_iter;
    stepsiz = (1f / (f32(_e61) - 1f));
    let _e67 = uv_6;
    let _e68 = nrand(_e67);
    rnd = _e68;
    let _e70 = rnd;
    let _e71 = stepsiz;
    t_6 = (_e70 * _e71);
    loop {
        let _e82 = i;
        let _e83 = num_iter;
        if !((_e82 < _e83)) {
            break;
        }
        {
            let _e89 = t_6;
            let _e90 = spectrum_offset_rgb(_e89);
            w = _e90;
            let _e92 = sumw;
            let _e93 = w;
            sumw = (_e92 + _e93);
            let _e95 = uv_6;
            let _e96 = t_6;
            let _e97 = min_distort_2;
            let _e98 = max_distort_2;
            let _e99 = distort(_e95, _e96, _e97, _e98);
            uvd = _e99;
            let _e101 = uvd;
            let _e102 = render(_e101);
            col = _e102;
            let _e104 = col;
            let _e106 = col;
            let _e108 = w;
            let _e109 = (_e106.xyz * _e108);
            col.x = _e109.x;
            col.y = _e109.y;
            col.z = _e109.z;
            let _e116 = sumcol;
            let _e117 = col;
            sumcol = (_e116 + _e117);
            let _e119 = t_6;
            let _e120 = stepsiz;
            t_6 = (_e119 + _e120);
        }
        continuing {
            let _e86 = i;
            i = (_e86 + 1i);
        }
    }
    let _e122 = sumcol;
    let _e124 = sumcol;
    let _e126 = sumw;
    let _e127 = (_e124.xyz / _e126);
    sumcol.x = _e127.x;
    sumcol.y = _e127.y;
    sumcol.z = _e127.z;
    let _e134 = sumcol;
    let _e135 = rnd;
    sumcol = (_e134 + vec4((_e135 / 255f)));
    let _e140 = sumcol;
    o_color = _e140;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e22 = o_color;
    return FragmentOutput(_e22);
}
