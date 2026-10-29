struct U {
    amount: f32,
    time: f32,
    speed: f32,
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

fn sat(t: f32) -> f32 {
    var t_1: f32;

    t_1 = t;
    let _e14 = t_1;
    return clamp(_e14, 0f, 1f);
}

fn sat_1(t_2: vec2<f32>) -> vec2<f32> {
    var t_3: vec2<f32>;

    t_3 = t_2;
    let _e14 = t_3;
    return clamp(_e14, vec2(0f), vec2(1f));
}

fn remap(t_4: f32, a: f32, b: f32) -> f32 {
    var t_5: f32;
    var a_1: f32;
    var b_1: f32;

    t_5 = t_4;
    a_1 = a;
    b_1 = b;
    let _e18 = t_5;
    let _e19 = a_1;
    let _e21 = b_1;
    let _e22 = a_1;
    let _e25 = sat(((_e18 - _e19) / (_e21 - _e22)));
    return _e25;
}

fn linterp(t_6: f32) -> f32 {
    var t_7: f32;

    t_7 = t_6;
    let _e16 = t_7;
    let _e22 = sat((1f - abs(((2f * _e16) - 1f))));
    return _e22;
}

fn spectrum_offset(t_8: f32) -> vec3<f32> {
    var t_9: f32;
    var t0_: f32;

    t_9 = t_8;
    let _e15 = t_9;
    t0_ = ((3f * _e15) - 1.5f);
    let _e20 = t0_;
    let _e23 = t0_;
    let _e26 = t0_;
    return clamp(vec3<f32>(-(_e20), (1f - abs(_e23)), _e26), vec3(0f), vec3(1f));
}

fn rand(n: vec2<f32>) -> f32 {
    var n_1: vec2<f32>;

    n_1 = n;
    let _e14 = n_1;
    return fract((sin(dot(_e14.xy, vec2<f32>(12.9898f, 78.233f))) * 43758.547f));
}

fn srand(n_2: vec2<f32>) -> f32 {
    var n_3: vec2<f32>;

    n_3 = n_2;
    let _e14 = n_3;
    let _e15 = rand(_e14);
    return ((_e15 * 2f) - 1f);
}

fn mytrunc(x: f32, num_levels: f32) -> f32 {
    var x_1: f32;
    var num_levels_1: f32;

    x_1 = x;
    num_levels_1 = num_levels;
    let _e16 = x_1;
    let _e17 = num_levels_1;
    let _e20 = num_levels_1;
    return (floor((_e16 * _e17)) / _e20);
}

fn mytrunc_1(x_2: vec2<f32>, num_levels_2: f32) -> vec2<f32> {
    var x_3: vec2<f32>;
    var num_levels_3: f32;

    x_3 = x_2;
    num_levels_3 = num_levels_2;
    let _e16 = x_3;
    let _e17 = num_levels_3;
    let _e20 = num_levels_3;
    return (floor((_e16 * _e17)) / vec2(_e20));
}

fn main_1() {
    var uv: vec2<f32>;
    var fps: f32;
    var mTime: f32;
    var GLITCH: f32;
    var gnm: f32;
    var rnd0_: f32;
    var r0_: f32;
    var rnd1_: f32;
    var r1_: f32;
    var local: f32;
    var rnd2_: f32;
    var r2_: f32;
    var rnd3_: f32;
    var r3_: f32;
    var pxrnd: f32;
    var local_1: f32;
    var ofs: f32;
    var NUM_SAMPLES: i32 = 20i;
    var RCP_NUM_SAMPLES_F: f32;
    var sum: vec4<f32> = vec4(0f);
    var wsum: vec3<f32> = vec3(0f);
    var i: i32 = 0i;
    var t_10: f32;
    var samplecol: vec4<f32>;
    var s: vec3<f32>;

    let _e12 = vUv_1;
    uv = _e12;
    let _e16 = global.speed;
    fps = (mix(0f, 60f, _e16) * 4f);
    let _e21 = global.time;
    let _e22 = fps;
    let _e24 = global.seed;
    mTime = floor(((_e21 * _e22) + (_e24 * 694.48f)));
    let _e30 = global.amount;
    GLITCH = (_e30 / 4f);
    let _e34 = GLITCH;
    let _e35 = sat(_e34);
    gnm = _e35;
    let _e37 = mTime;
    let _e38 = mTime;
    let _e41 = mytrunc_1(vec2<f32>(_e37, _e38), 6f);
    let _e42 = rand(_e41);
    rnd0_ = _e42;
    let _e45 = gnm;
    let _e49 = rnd0_;
    let _e51 = sat((((1f - _e45) * 0.7f) + _e49));
    r0_ = _e51;
    let _e53 = uv;
    let _e56 = r0_;
    let _e58 = mytrunc(_e53.x, (10f * _e56));
    let _e59 = mTime;
    let _e61 = rand(vec2<f32>(_e58, _e59));
    rnd1_ = _e61;
    let _e65 = gnm;
    let _e68 = rnd1_;
    r1_ = ((0.5f - (0.5f * _e65)) + _e68);
    let _e73 = r1_;
    if (_e73 < 1f) {
        let _e76 = r1_;
        local = _e76;
    } else {
        local = 0.9999999f;
    }
    let _e79 = local;
    r1_ = (1f - max(0f, _e79));
    let _e82 = uv;
    let _e85 = r1_;
    let _e87 = mytrunc(_e82.y, (40f * _e85));
    let _e88 = mTime;
    let _e90 = rand(vec2<f32>(_e87, _e88));
    rnd2_ = _e90;
    let _e92 = rnd2_;
    let _e93 = sat(_e92);
    r2_ = _e93;
    let _e95 = uv;
    let _e98 = r0_;
    let _e100 = mytrunc(_e95.y, (10f * _e98));
    let _e101 = mTime;
    let _e103 = rand(vec2<f32>(_e100, _e101));
    rnd3_ = _e103;
    let _e106 = rnd3_;
    let _e109 = sat((_e106 + 0.8f));
    r3_ = ((1f - _e109) - 0.1f);
    let _e114 = uv;
    let _e115 = mTime;
    let _e118 = rand((_e114 + vec2(_e115)));
    pxrnd = _e118;
    let _e121 = r2_;
    let _e123 = GLITCH;
    let _e125 = rnd0_;
    if (_e125 > 0.5f) {
        local_1 = 1f;
    } else {
        local_1 = -1f;
    }
    let _e132 = local_1;
    ofs = (((0.05f * _e121) * _e123) * _e132);
    let _e135 = ofs;
    let _e137 = pxrnd;
    let _e139 = ofs;
    ofs = (_e135 + ((0.5f * _e137) * _e139));
    let _e143 = uv;
    let _e146 = r3_;
    let _e148 = GLITCH;
    uv.y = (_e143.y + ((0.1f * _e146) * _e148));
    let _e154 = NUM_SAMPLES;
    RCP_NUM_SAMPLES_F = (1f / f32(_e154));
    loop {
        let _e166 = i;
        let _e167 = NUM_SAMPLES;
        if !((_e166 < _e167)) {
            break;
        }
        {
            let _e173 = i;
            let _e175 = RCP_NUM_SAMPLES_F;
            t_10 = (f32(_e173) * _e175);
            let _e179 = uv;
            let _e181 = ofs;
            let _e182 = t_10;
            let _e185 = sat((_e179.x + (_e181 * _e182)));
            uv.x = _e185;
            let _e186 = uv;
            let _e189 = textureSampleBias(tDiffuse_t, tDiffuse_s, _e186, -10f);
            samplecol = _e189;
            let _e191 = t_10;
            let _e192 = spectrum_offset(_e191);
            s = _e192;
            let _e194 = samplecol;
            let _e196 = samplecol;
            let _e198 = s;
            let _e199 = (_e196.xyz * _e198);
            samplecol.x = _e199.x;
            samplecol.y = _e199.y;
            samplecol.z = _e199.z;
            let _e206 = sum;
            let _e207 = samplecol;
            sum = (_e206 + _e207);
            let _e209 = wsum;
            let _e210 = s;
            wsum = (_e209 + _e210);
        }
        continuing {
            let _e170 = i;
            i = (_e170 + 1i);
        }
    }
    let _e212 = sum;
    let _e214 = sum;
    let _e216 = wsum;
    let _e217 = (_e214.xyz / _e216);
    sum.x = _e217.x;
    sum.y = _e217.y;
    sum.z = _e217.z;
    let _e225 = sum;
    let _e227 = RCP_NUM_SAMPLES_F;
    sum.w = (_e225.w * _e227);
    let _e230 = sum;
    o_color.w = _e230.w;
    let _e232 = o_color;
    let _e234 = sum;
    let _e235 = _e234.xyz;
    o_color.x = _e235.x;
    o_color.y = _e235.y;
    o_color.z = _e235.z;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
