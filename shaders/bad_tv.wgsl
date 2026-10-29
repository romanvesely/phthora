struct U {
    time: f32,
    distortion: f32,
    distortion2_: f32,
    rollSpeed: f32,
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

fn mod289_2D(x: vec3<f32>) -> vec3<f32> {
    var x_1: vec3<f32>;

    x_1 = x;
    let _e16 = x_1;
    let _e17 = x_1;
    return (_e16 - (floor((_e17 * 0.0034602077f)) * 289f));
}

fn mod289_2D_1(x_2: vec2<f32>) -> vec2<f32> {
    var x_3: vec2<f32>;

    x_3 = x_2;
    let _e16 = x_3;
    let _e17 = x_3;
    return (_e16 - (floor((_e17 * 0.0034602077f)) * 289f));
}

fn permute(x_4: vec3<f32>) -> vec3<f32> {
    var x_5: vec3<f32>;

    x_5 = x_4;
    let _e16 = x_5;
    let _e22 = x_5;
    let _e24 = mod289_2D((((_e16 * 34f) + vec3(1f)) * _e22));
    return _e24;
}

fn noise2d(v: vec2<f32>) -> f32 {
    var v_1: vec2<f32>;
    var C: vec4<f32> = vec4<f32>(0.21132487f, 0.36602542f, -0.57735026f, 0.024390243f);
    var i: vec2<f32>;
    var x0_: vec2<f32>;
    var i1_: vec2<f32>;
    var local: vec2<f32>;
    var x12_: vec4<f32>;
    var p: vec3<f32>;
    var m: vec3<f32>;
    var x_6: vec3<f32>;
    var h: vec3<f32>;
    var ox: vec3<f32>;
    var a0_: vec3<f32>;
    var g: vec3<f32>;

    v_1 = v;
    let _e23 = v_1;
    let _e24 = v_1;
    let _e25 = C;
    i = floor((_e23 + vec2(dot(_e24, _e25.yy))));
    let _e32 = v_1;
    let _e33 = i;
    let _e35 = i;
    let _e36 = C;
    x0_ = ((_e32 - _e33) + vec2(dot(_e35, _e36.xx)));
    let _e43 = x0_;
    let _e45 = x0_;
    if (_e43.x > _e45.y) {
        local = vec2<f32>(1f, 0f);
    } else {
        local = vec2<f32>(0f, 1f);
    }
    let _e55 = local;
    i1_ = _e55;
    let _e56 = x0_;
    let _e58 = C;
    x12_ = (_e56.xyxy + _e58.xxzz);
    let _e62 = x12_;
    let _e64 = x12_;
    let _e66 = i1_;
    let _e67 = (_e64.xy - _e66);
    x12_.x = _e67.x;
    x12_.y = _e67.y;
    let _e72 = i;
    let _e73 = mod289_2D_1(_e72);
    i = _e73;
    let _e74 = i;
    let _e77 = i1_;
    let _e83 = permute((vec3(_e74.y) + vec3<f32>(0f, _e77.y, 1f)));
    let _e84 = i;
    let _e89 = i1_;
    let _e94 = permute(((_e83 + vec3(_e84.x)) + vec3<f32>(0f, _e89.x, 1f)));
    p = _e94;
    let _e97 = x0_;
    let _e98 = x0_;
    let _e100 = x12_;
    let _e102 = x12_;
    let _e105 = x12_;
    let _e107 = x12_;
    m = max((vec3(0.5f) - vec3<f32>(dot(_e97, _e98), dot(_e100.xy, _e102.xy), dot(_e105.zw, _e107.zw))), vec3(0f));
    let _e117 = m;
    let _e118 = m;
    m = (_e117 * _e118);
    let _e120 = m;
    let _e121 = m;
    m = (_e120 * _e121);
    let _e124 = p;
    let _e125 = C;
    x_6 = ((2f * fract((_e124 * _e125.www))) - vec3(1f));
    let _e134 = x_6;
    h = (abs(_e134) - vec3(0.5f));
    let _e140 = x_6;
    ox = floor((_e140 + vec3(0.5f)));
    let _e146 = x_6;
    let _e147 = ox;
    a0_ = (_e146 - _e147);
    let _e150 = m;
    let _e153 = a0_;
    let _e154 = a0_;
    let _e156 = h;
    let _e157 = h;
    m = (_e150 * (vec3(1.7928429f) - (0.85373473f * ((_e153 * _e154) + (_e156 * _e157)))));
    let _e166 = a0_;
    let _e168 = x0_;
    let _e171 = h;
    let _e173 = x0_;
    g.x = ((_e166.x * _e168.x) + (_e171.x * _e173.y));
    let _e177 = g;
    let _e179 = a0_;
    let _e181 = x12_;
    let _e184 = h;
    let _e186 = x12_;
    let _e189 = ((_e179.yz * _e181.xz) + (_e184.yz * _e186.yw));
    g.y = _e189.x;
    g.z = _e189.y;
    let _e195 = m;
    let _e196 = g;
    return (130f * dot(_e195, _e196));
}

fn main_1() {
    var p_1: vec2<f32>;
    var time2_: f32;
    var ty: f32;
    var yt: f32;
    var offset: f32;

    let _e14 = vUv_1;
    p_1 = _e14;
    let _e16 = global.time;
    let _e17 = global.seed;
    time2_ = (_e16 + _e17);
    let _e20 = time2_;
    ty = (_e20 * 2f);
    let _e24 = p_1;
    let _e26 = ty;
    yt = (_e24.y + _e26);
    let _e29 = yt;
    let _e34 = noise2d(vec2<f32>((_e29 * 3f), 0f));
    offset = (_e34 * 0.2f);
    let _e38 = offset;
    let _e39 = global.distortion;
    let _e41 = offset;
    let _e43 = global.distortion;
    let _e45 = offset;
    offset = (((((_e38 * _e39) * _e41) * _e43) * _e45) * 36f);
    let _e49 = offset;
    let _e50 = yt;
    let _e55 = noise2d(vec2<f32>((_e50 * 50f), 0f));
    let _e56 = global.distortion2_;
    offset = (_e49 + ((_e55 * _e56) * 0.012f));
    let _e61 = p_1;
    let _e63 = offset;
    let _e66 = p_1;
    let _e68 = time2_;
    let _e69 = global.rollSpeed;
    let _e74 = textureSample(tDiffuse_t, tDiffuse_s, vec2<f32>(fract((_e61.x + _e63)), fract((_e66.y - (_e68 * _e69)))));
    o_color = _e74;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
