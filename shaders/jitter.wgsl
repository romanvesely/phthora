struct U {
    amount: f32,
    speed: f32,
    time: f32,
    angle: f32,
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

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e16 = n_1;
    return fract((sin(_e16) * 43758.547f));
}

fn random2d(n_2: vec2<f32>) -> f32 {
    var n_3: vec2<f32>;

    n_3 = n_2;
    let _e16 = n_3;
    return fract((sin(dot(_e16, vec2<f32>(12.9898f, 4.1414f))) * 43758.547f));
}

fn randomRange(seed: vec2<f32>, min_: f32, max_: f32) -> f32 {
    var seed_1: vec2<f32>;
    var min_1: f32;
    var max_1: f32;

    seed_1 = seed;
    min_1 = min_;
    max_1 = max_;
    let _e20 = min_1;
    let _e21 = seed_1;
    let _e22 = random2d(_e21);
    let _e23 = max_1;
    let _e24 = min_1;
    return (_e20 + (_e22 * (_e23 - _e24)));
}

fn insideRange(v: f32, bottom: f32, top: f32) -> f32 {
    var v_1: f32;
    var bottom_1: f32;
    var top_1: f32;

    v_1 = v;
    bottom_1 = bottom;
    top_1 = top;
    let _e20 = bottom_1;
    let _e21 = v_1;
    let _e23 = top_1;
    let _e24 = v_1;
    return (step(_e20, _e21) - step(_e23, _e24));
}

fn rand(co: vec2<f32>) -> f32 {
    var co_1: vec2<f32>;

    co_1 = co;
    let _e16 = co_1;
    return fract((sin(dot(_e16.xy, vec2<f32>(12.9898f, 78.233f))) * 43758.547f));
}

fn rotate2d(position: vec2<f32>, theta: f32) -> vec2<f32> {
    var position_1: vec2<f32>;
    var theta_1: f32;
    var m: mat2x2<f32>;

    position_1 = position;
    theta_1 = theta;
    let _e18 = theta_1;
    let _e20 = theta_1;
    let _e23 = theta_1;
    let _e25 = theta_1;
    m = mat2x2<f32>(vec2<f32>(cos(_e18), -(sin(_e20))), vec2<f32>(sin(_e23), cos(_e25)));
    let _e31 = m;
    let _e32 = position_1;
    return (_e31 * _e32);
}

fn main_1() {
    var uv: vec2<f32>;
    var ang: f32;
    var sTime: f32;
    var inCol: vec4<f32>;
    var outCol: vec4<f32>;
    var amount2_: f32;
    var maxOffset: f32;
    var uvR: vec2<f32>;
    var uvOff: vec2<f32>;
    var i: f32 = 0f;
    var sliceY: f32;
    var sliceH: f32;
    var hOffset: f32;
    var uvOff_1: vec2<f32>;
    var maxColOffset: f32;
    var colOffset: vec2<f32>;
    var rnd: f32;

    let _e14 = vUv_1;
    uv = _e14;
    let _e16 = global.angle;
    ang = (_e16 * 3.1415927f);
    let _e20 = global.time;
    let _e21 = global.speed;
    let _e27 = global.seed;
    sTime = floor(((((_e20 * _e21) * 6f) * 24f) + (_e27 * 9999.59f)));
    let _e33 = uv;
    let _e34 = textureSample(tDiffuse_t, tDiffuse_s, _e33);
    inCol = _e34;
    let _e36 = inCol;
    outCol = _e36;
    let _e38 = global.amount;
    amount2_ = (_e38 / 2f);
    let _e42 = amount2_;
    maxOffset = (_e42 / 2f);
    let _e46 = uv;
    uvR = _e46;
    let _e48 = uv;
    uvR = (_e48 - vec2(0.5f));
    let _e52 = uvR;
    let _e53 = ang;
    let _e54 = rotate2d(_e52, _e53);
    uvR = _e54;
    let _e55 = uvR;
    uvR = (_e55 + vec2(0.5f));
    loop {
        let _e62 = i;
        if !((_e62 < 10f)) {
            break;
        }
        {
            let _e69 = i;
            let _e71 = amount2_;
            if (_e69 > (10f * _e71)) {
                break;
            }
            let _e74 = sTime;
            let _e75 = amount2_;
            let _e78 = i;
            let _e82 = random2d(vec2<f32>((_e74 + _e75), (2345f + f32(_e78))));
            sliceY = _e82;
            let _e84 = sTime;
            let _e85 = amount2_;
            let _e88 = i;
            let _e92 = random2d(vec2<f32>((_e84 + _e85), (9035f + f32(_e88))));
            sliceH = (_e92 * 0.25f);
            let _e96 = sTime;
            let _e97 = amount2_;
            let _e100 = i;
            let _e104 = maxOffset;
            let _e106 = maxOffset;
            let _e107 = randomRange(vec2<f32>((_e96 + _e97), (9625f + f32(_e100))), -(_e104), _e106);
            hOffset = _e107;
            let _e109 = uv;
            uvOff = _e109;
            let _e110 = uvOff;
            let _e111 = hOffset;
            let _e115 = ang;
            let _e117 = rotate2d(vec2<f32>(_e111, 0f), (3.1415927f - _e115));
            uvOff = (_e110 + _e117);
            let _e119 = uvOff;
            uvOff_1 = fract(_e119);
            let _e122 = uvR;
            let _e124 = sliceY;
            let _e125 = sliceY;
            let _e126 = sliceH;
            let _e129 = insideRange(_e122.y, _e124, fract((_e125 + _e126)));
            if (_e129 == 1f) {
                {
                    let _e132 = uvOff_1;
                    let _e133 = textureSample(tDiffuse_t, tDiffuse_s, _e132);
                    outCol = _e133;
                }
            }
        }
        continuing {
            let _e66 = i;
            i = (_e66 + 1f);
        }
    }
    let _e134 = amount2_;
    maxColOffset = (_e134 / 6f);
    let _e138 = sTime;
    let _e139 = amount2_;
    let _e143 = maxColOffset;
    let _e145 = maxColOffset;
    let _e146 = randomRange(vec2<f32>((_e138 + _e139), 3545f), -(_e143), _e145);
    let _e147 = sTime;
    let _e150 = maxColOffset;
    let _e152 = maxColOffset;
    let _e153 = randomRange(vec2<f32>(_e147, 7205f), -(_e150), _e152);
    colOffset = vec2<f32>(_e146, _e153);
    let _e156 = uv;
    let _e157 = colOffset;
    uvOff = clamp((_e156 + _e157), vec2(0f), vec2(1f));
    let _e164 = sTime;
    let _e165 = amount2_;
    let _e169 = random2d(vec2<f32>((_e164 + _e165), 9545f));
    rnd = _e169;
    let _e171 = rnd;
    if (_e171 < 0.33f) {
        {
            let _e175 = uvOff;
            let _e176 = textureSample(tDiffuse_t, tDiffuse_s, _e175);
            outCol.x = _e176.x;
        }
    } else {
        let _e178 = rnd;
        if (_e178 < 0.66f) {
            {
                let _e182 = uvOff;
                let _e183 = textureSample(tDiffuse_t, tDiffuse_s, _e182);
                outCol.y = _e183.y;
            }
        } else {
            {
                let _e186 = uvOff;
                let _e187 = textureSample(tDiffuse_t, tDiffuse_s, _e186);
                outCol.z = _e187.z;
            }
        }
    }
    let _e189 = outCol;
    o_color = vec4<f32>(_e189);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
