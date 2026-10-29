struct U {
    x: f32,
    y: f32,
    angle: f32,
    scalex: f32,
    scaley: f32,
    resolution: vec2<f32>,
    b_repeat: f32,
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

fn insideBox(v: vec2<f32>, bottomLeft: vec2<f32>, topRight: vec2<f32>) -> f32 {
    var v_1: vec2<f32>;
    var bottomLeft_1: vec2<f32>;
    var topRight_1: vec2<f32>;
    var s: vec2<f32>;

    v_1 = v;
    bottomLeft_1 = bottomLeft;
    topRight_1 = topRight;
    let _e24 = bottomLeft_1;
    let _e25 = v_1;
    let _e27 = topRight_1;
    let _e28 = v_1;
    s = (step(_e24, _e25) - step(_e27, _e28));
    let _e32 = s;
    let _e34 = s;
    return (_e32.x * _e34.y);
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e22 = theta_1;
    let _e24 = theta_1;
    let _e27 = theta_1;
    let _e29 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e22), -(sin(_e24))), vec2<f32>(sin(_e27), cos(_e29)));
    let _e35 = m;
    let _e36 = position_1;
    return (_e35 * _e36);
}

fn main_1() {
    var p: vec2<f32>;
    var outCol: vec4<f32> = vec4(0f);

    let _e18 = vUv_1;
    p = _e18;
    let _e20 = p;
    p = (_e20 - vec2(0.5f));
    let _e24 = p;
    let _e26 = global.scalex;
    let _e27 = global.scaley;
    p = (_e24 * (vec2(1f) / vec2<f32>(_e26, _e27)));
    let _e33 = p;
    let _e35 = global.resolution;
    let _e37 = global.resolution;
    p.y = (_e33.y * (_e35.y / _e37.x));
    let _e41 = p;
    let _e42 = global.angle;
    let _e45 = rotate2d(_e41, (_e42 * 6.2831855f));
    p = _e45;
    let _e47 = p;
    let _e49 = global.resolution;
    let _e51 = global.resolution;
    p.y = (_e47.y * (_e49.x / _e51.y));
    let _e55 = p;
    p = (_e55 + vec2(0.5f));
    let _e59 = p;
    let _e60 = global.x;
    let _e61 = global.y;
    p = (_e59 - vec2<f32>(_e60, _e61));
    let _e67 = global.b_repeat;
    if (_e67 > 0.5f) {
        {
            let _e70 = p;
            p = fract(_e70);
            let _e72 = p;
            let _e73 = textureSample(tDiffuse_t, tDiffuse_s, _e72);
            outCol = _e73;
        }
    } else {
        {
            let _e74 = p;
            let _e81 = insideBox(_e74, vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
            if (_e81 > 0.5f) {
                {
                    let _e84 = p;
                    let _e85 = textureSample(tDiffuse_t, tDiffuse_s, _e84);
                    outCol = _e85;
                }
            }
        }
    }
    let _e86 = outCol;
    o_color = _e86;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
