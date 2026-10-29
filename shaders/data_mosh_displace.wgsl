struct U {
    time: f32,
    amount: f32,
    blockiness: f32,
    flow: f32,
    resolution: vec2<f32>,
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
var tPrev_t: texture_2d<f32>;
@group(0) @binding(4) 
var tPrev_s: sampler;
@group(0) @binding(5) 
var tOFMap_t: texture_2d<f32>;
@group(0) @binding(6) 
var tOFMap_s: sampler;

fn mod289_2D(x: vec3<f32>) -> vec3<f32> {
    var x_1: vec3<f32>;

    x_1 = x;
    let _e20 = x_1;
    let _e21 = x_1;
    return (_e20 - (floor((_e21 * 0.0034602077f)) * 289f));
}

fn mod289_2D_1(x_2: vec2<f32>) -> vec2<f32> {
    var x_3: vec2<f32>;

    x_3 = x_2;
    let _e20 = x_3;
    let _e21 = x_3;
    return (_e20 - (floor((_e21 * 0.0034602077f)) * 289f));
}

fn permute(x_4: vec3<f32>) -> vec3<f32> {
    var x_5: vec3<f32>;

    x_5 = x_4;
    let _e20 = x_5;
    let _e26 = x_5;
    let _e28 = mod289_2D((((_e20 * 34f) + vec3(1f)) * _e26));
    return _e28;
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
    let _e27 = v_1;
    let _e28 = v_1;
    let _e29 = C;
    i = floor((_e27 + vec2(dot(_e28, _e29.yy))));
    let _e36 = v_1;
    let _e37 = i;
    let _e39 = i;
    let _e40 = C;
    x0_ = ((_e36 - _e37) + vec2(dot(_e39, _e40.xx)));
    let _e47 = x0_;
    let _e49 = x0_;
    if (_e47.x > _e49.y) {
        local = vec2<f32>(1f, 0f);
    } else {
        local = vec2<f32>(0f, 1f);
    }
    let _e59 = local;
    i1_ = _e59;
    let _e60 = x0_;
    let _e62 = C;
    x12_ = (_e60.xyxy + _e62.xxzz);
    let _e66 = x12_;
    let _e68 = x12_;
    let _e70 = i1_;
    let _e71 = (_e68.xy - _e70);
    x12_.x = _e71.x;
    x12_.y = _e71.y;
    let _e76 = i;
    let _e77 = mod289_2D_1(_e76);
    i = _e77;
    let _e78 = i;
    let _e81 = i1_;
    let _e87 = permute((vec3(_e78.y) + vec3<f32>(0f, _e81.y, 1f)));
    let _e88 = i;
    let _e93 = i1_;
    let _e98 = permute(((_e87 + vec3(_e88.x)) + vec3<f32>(0f, _e93.x, 1f)));
    p = _e98;
    let _e101 = x0_;
    let _e102 = x0_;
    let _e104 = x12_;
    let _e106 = x12_;
    let _e109 = x12_;
    let _e111 = x12_;
    m = max((vec3(0.5f) - vec3<f32>(dot(_e101, _e102), dot(_e104.xy, _e106.xy), dot(_e109.zw, _e111.zw))), vec3(0f));
    let _e121 = m;
    let _e122 = m;
    m = (_e121 * _e122);
    let _e124 = m;
    let _e125 = m;
    m = (_e124 * _e125);
    let _e128 = p;
    let _e129 = C;
    x_6 = ((2f * fract((_e128 * _e129.www))) - vec3(1f));
    let _e138 = x_6;
    h = (abs(_e138) - vec3(0.5f));
    let _e144 = x_6;
    ox = floor((_e144 + vec3(0.5f)));
    let _e150 = x_6;
    let _e151 = ox;
    a0_ = (_e150 - _e151);
    let _e154 = m;
    let _e157 = a0_;
    let _e158 = a0_;
    let _e160 = h;
    let _e161 = h;
    m = (_e154 * (vec3(1.7928429f) - (0.85373473f * ((_e157 * _e158) + (_e160 * _e161)))));
    let _e170 = a0_;
    let _e172 = x0_;
    let _e175 = h;
    let _e177 = x0_;
    g.x = ((_e170.x * _e172.x) + (_e175.x * _e177.y));
    let _e181 = g;
    let _e183 = a0_;
    let _e185 = x12_;
    let _e188 = h;
    let _e190 = x12_;
    let _e193 = ((_e183.yz * _e185.xz) + (_e188.yz * _e190.yw));
    g.y = _e193.x;
    g.z = _e193.y;
    let _e199 = m;
    let _e200 = g;
    return (130f * dot(_e199, _e200));
}

fn main_1() {
    var current: vec4<f32>;
    var blockSize: f32;
    var uvOF: vec2<f32>;
    var ofMap: f32;
    var offset: vec2<f32>;
    var offsetUV: vec2<f32>;
    var past: vec4<f32>;
    var cutoff: f32;
    var mask: f32;
    var col: vec4<f32>;

    let _e18 = vUv_1;
    let _e19 = textureSample(tDiffuse_t, tDiffuse_s, _e18);
    current = _e19;
    let _e22 = global.resolution;
    let _e25 = global.blockiness;
    blockSize = mix(50f, _e22.x, (1f - _e25));
    let _e29 = vUv_1;
    let _e30 = blockSize;
    let _e33 = blockSize;
    uvOF = (floor((_e29 * _e30)) / vec2(_e33));
    let _e37 = uvOF;
    let _e38 = textureSample(tOFMap_t, tOFMap_s, _e37);
    ofMap = _e38.x;
    let _e41 = ofMap;
    ofMap = clamp(_e41, 0f, 1f);
    let _e45 = vUv_1;
    let _e46 = global.time;
    let _e49 = noise2d((_e45 + vec2(_e46)));
    let _e50 = vUv_1;
    let _e51 = global.time;
    let _e56 = noise2d((_e50 + vec2((_e51 * 233.23424f))));
    offset = vec2<f32>(_e49, _e56);
    let _e59 = offset;
    let _e60 = global.flow;
    offset = (_e59 * (_e60 * 0.006f));
    let _e64 = vUv_1;
    let _e65 = offset;
    offsetUV = (_e64 + _e65);
    let _e68 = offsetUV;
    let _e69 = textureSample(tPrev_t, tPrev_s, _e68);
    past = _e69;
    let _e73 = global.amount;
    cutoff = mix(1.01f, 0.3f, _e73);
    let _e76 = cutoff;
    let _e77 = ofMap;
    mask = step(_e76, _e77);
    let _e80 = current;
    let _e81 = past;
    let _e82 = mask;
    col = mix(_e80, _e81, vec4(_e82));
    let _e86 = col;
    o_color = _e86;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e29 = o_color;
    return FragmentOutput(_e29);
}
