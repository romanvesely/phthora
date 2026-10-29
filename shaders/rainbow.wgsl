struct U {
    amount: f32,
    offset: f32,
    time: f32,
    speed: f32,
    angle: f32,
    palette: f32,
    seed: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const TWO_PI: f32 = 6.2831855f;

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;

fn pal(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    var t_1: f32;
    var a_1: vec3<f32>;
    var b_1: vec3<f32>;
    var c_1: vec3<f32>;
    var d_1: vec3<f32>;

    t_1 = t;
    a_1 = a;
    b_1 = b;
    c_1 = c;
    d_1 = d;
    let _e29 = a_1;
    let _e30 = b_1;
    let _e32 = c_1;
    let _e33 = t_1;
    let _e35 = d_1;
    return (_e29 + (_e30 * cos((6.28318f * ((_e32 * _e33) + _e35)))));
}

fn rainbow(t_2: f32) -> vec3<f32> {
    var t_3: f32;
    var d_2: vec3<f32> = vec3<f32>(0f, 0.33f, 0.67f);

    t_3 = t_2;
    let _e29 = t_3;
    let _e30 = d_2;
    return (vec3(0.5f) + (0.5f * cos((6.28318f * (vec3(_e29) + _e30)))));
}

fn rotate2D(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e23 = theta_1;
    let _e25 = theta_1;
    let _e28 = theta_1;
    let _e30 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e23), -(sin(_e25))), vec2<f32>(sin(_e28), cos(_e30)));
    let _e36 = m;
    let _e37 = position_1;
    return (_e36 * _e37);
}

fn main_1() {
    var amount2_: f32;
    var offset2_: f32;
    var p: vec2<f32>;
    var orig: vec4<f32>;
    var off: vec2<f32>;
    var colVal: f32;
    var grad: vec3<f32>;
    var col: vec3<f32>;

    let _e19 = global.amount;
    amount2_ = (_e19 * 0.8f);
    let _e23 = global.offset;
    offset2_ = (_e23 * 2f);
    let _e27 = vUv_1;
    p = _e27;
    let _e29 = p;
    let _e30 = textureSample(tDiffuse_t, tDiffuse_s, _e29);
    orig = _e30;
    let _e32 = orig;
    off = (_e32.xy - vec2(0.5f));
    let _e38 = p;
    let _e39 = off;
    let _e40 = offset2_;
    p = (_e38 + (_e39 * _e40));
    let _e43 = p;
    p = (_e43 - vec2(0.5f));
    let _e47 = p;
    let _e48 = global.angle;
    let _e50 = rotate2D(_e47, (_e48 * TWO_PI));
    p = _e50;
    let _e51 = p;
    p = (_e51 + vec2(0.5f));
    let _e55 = p;
    let _e57 = p;
    let _e60 = global.time;
    let _e61 = global.speed;
    let _e66 = global.seed;
    colVal = ((((_e55.x + _e57.y) + ((_e60 * _e61) * 2f)) + (_e66 * 2f)) * 0.5f);
    let _e73 = colVal;
    let _e90 = pal(_e73, vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(1f, 1f, 1f), vec3<f32>(0f, 0.33f, 0.67f));
    grad = _e90;
    let _e92 = global.palette;
    if (_e92 == 2f) {
        let _e95 = colVal;
        let _e112 = pal(_e95, vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(1f, 1f, 1f), vec3<f32>(0f, 0.1f, 0.2f));
        grad = _e112;
    }
    let _e113 = global.palette;
    if (_e113 == 3f) {
        let _e116 = colVal;
        let _e133 = pal(_e116, vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(2f, 1f, 0f), vec3<f32>(0.5f, 0.2f, 0.25f));
        grad = _e133;
    }
    let _e134 = global.palette;
    if (_e134 == 4f) {
        let _e137 = colVal;
        let _e154 = pal(_e137, vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(0.5f, 0.5f, 0.5f), vec3<f32>(1f, 1f, 0.5f), vec3<f32>(0.8f, 0.9f, 0.3f));
        grad = _e154;
    }
    let _e155 = orig;
    let _e157 = grad;
    let _e158 = amount2_;
    let _e159 = orig;
    col = mix(_e155.xyz, _e157, vec3((_e158 * _e159.w)));
    let _e165 = col;
    let _e166 = orig;
    o_color = vec4<f32>(_e165.x, _e165.y, _e165.z, _e166.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e27 = o_color;
    return FragmentOutput(_e27);
}
