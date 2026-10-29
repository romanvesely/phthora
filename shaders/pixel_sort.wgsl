struct U {
    resolution: vec2<f32>,
    threshold: f32,
    amount: f32,
    b_verticalSort: f32,
    frame: f32,
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

fn gscale(c: vec3<f32>) -> f32 {
    var c_1: vec3<f32>;

    c_1 = c;
    let _e18 = c_1;
    let _e20 = c_1;
    let _e23 = c_1;
    return (((_e18.x + _e20.y) + _e23.z) / 3f);
}

fn main_1() {
    var uv: vec2<f32>;
    var orig: vec4<f32>;
    var threshold2_: f32;
    var result: vec4<f32>;
    var vp: f32;
    var dir: vec2<f32>;
    var fParity: f32;
    var curr: vec4<f32>;
    var comp: vec4<f32>;
    var gCurr: f32;
    var gComp: f32;
    var amount2_: f32;
    var cutoff: f32;

    let _e16 = vUv_1;
    uv = _e16;
    let _e18 = uv;
    let _e19 = textureSample(tDiffuse_t, tDiffuse_s, _e18);
    orig = _e19;
    let _e21 = global.frame;
    if (_e21 < 2f) {
        {
            let _e24 = orig;
            o_color = _e24;
            return;
        }
    }
    let _e25 = global.threshold;
    threshold2_ = (_e25 * 0.71f);
    let _e32 = global.frame;
    let _e33 = f32(_e32);
    fParity = (((_e33 - (floor((_e33 / 2f)) * 2f)) * 2f) - 1f);
    let _e44 = global.b_verticalSort;
    if (_e44 > 0.5f) {
        {
            let _e47 = uv;
            let _e49 = global.resolution;
            let _e52 = floor((_e47.y * _e49.y));
            vp = (((_e52 - (floor((_e52 / 2f)) * 2f)) * 2f) - 1f);
            dir = vec2<f32>(0f, 1f);
        }
    } else {
        {
            let _e67 = uv;
            let _e69 = global.resolution;
            let _e72 = floor((_e67.x * _e69.x));
            vp = (((_e72 - (floor((_e72 / 2f)) * 2f)) * 2f) - 1f);
            dir = vec2<f32>(1f, 0f);
        }
    }
    let _e87 = dir;
    let _e88 = fParity;
    let _e89 = vp;
    dir = (_e87 * (_e88 * _e89));
    let _e92 = dir;
    let _e93 = global.resolution;
    dir = (_e92 / _e93.xy);
    let _e96 = uv;
    let _e97 = textureSample(tPrev_t, tPrev_s, _e96);
    curr = _e97;
    let _e99 = uv;
    let _e100 = dir;
    let _e102 = textureSample(tPrev_t, tPrev_s, (_e99 + _e100));
    comp = _e102;
    let _e104 = curr;
    let _e106 = gscale(_e104.xyz);
    gCurr = _e106;
    let _e108 = comp;
    let _e110 = gscale(_e108.xyz);
    gComp = _e110;
    let _e112 = dir;
    let _e116 = dir;
    if ((_e112.y < 0f) || (_e116.x < 0f)) {
        {
            let _e121 = gCurr;
            let _e122 = threshold2_;
            let _e124 = gComp;
            let _e125 = gCurr;
            if ((_e121 > _e122) && (_e124 > _e125)) {
                {
                    let _e128 = comp;
                    result = _e128;
                }
            } else {
                {
                    let _e129 = curr;
                    result = _e129;
                }
            }
        }
    } else {
        {
            let _e130 = gComp;
            let _e131 = threshold2_;
            let _e133 = gCurr;
            let _e134 = gComp;
            if ((_e130 > _e131) && (_e133 >= _e134)) {
                {
                    let _e137 = comp;
                    result = _e137;
                }
            } else {
                {
                    let _e138 = curr;
                    result = _e138;
                }
            }
        }
    }
    let _e141 = global.amount;
    amount2_ = mix(0.85f, 1f, _e141);
    let _e146 = global.amount;
    cutoff = smoothstep(0f, 0.1f, _e146);
    let _e149 = amount2_;
    let _e150 = cutoff;
    amount2_ = (_e149 * _e150);
    let _e152 = orig;
    let _e153 = result;
    let _e154 = amount2_;
    result = mix(_e152, _e153, vec4(_e154));
    let _e157 = result;
    o_color = _e157;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
