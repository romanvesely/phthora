struct U {
    amount: f32,
    colorize: f32,
    scale: f32,
    modeIndex: f32,
    b_animate: f32,
    animFrame: f32,
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
var tNoise_t: texture_2d<f32>;
var<private> gl_FragCoord_1: vec4<f32>;

const PI: f32 = 3.1415927f;
const NOISE_SIZE: i32 = 64i;

fn luma(color: vec3<f32>) -> f32 {
    var color_1: vec3<f32>;

    color_1 = color;
    let _e12 = color_1;
    return dot(_e12, vec3<f32>(0.299f, 0.587f, 0.114f));
}

fn dither(position: vec2<f32>, brightness: f32) -> f32 {
    var position_1: vec2<f32>;
    var brightness_1: f32;
    var x: i32;
    var y: i32;
    var index: i32;
    var limit: f32 = 0f;
    var local: f32;

    position_1 = position;
    brightness_1 = brightness;
    let _e14 = position_1;
    x = i32((_e14.x - (floor((_e14.x / 4f)) * 4f)));
    let _e23 = position_1;
    y = i32((_e23.y - (floor((_e23.y / 4f)) * 4f)));
    let _e32 = x;
    let _e33 = y;
    index = (_e32 + (_e33 * 4i));
    let _e40 = x;
    if (_e40 < 8i) {
        {
            let _e43 = index;
            if (_e43 == 0i) {
                limit = 0.0625f;
            }
            let _e47 = index;
            if (_e47 == 1i) {
                limit = 0.5625f;
            }
            let _e51 = index;
            if (_e51 == 2i) {
                limit = 0.1875f;
            }
            let _e55 = index;
            if (_e55 == 3i) {
                limit = 0.6875f;
            }
            let _e59 = index;
            if (_e59 == 4i) {
                limit = 0.8125f;
            }
            let _e63 = index;
            if (_e63 == 5i) {
                limit = 0.3125f;
            }
            let _e67 = index;
            if (_e67 == 6i) {
                limit = 0.9375f;
            }
            let _e71 = index;
            if (_e71 == 7i) {
                limit = 0.4375f;
            }
            let _e75 = index;
            if (_e75 == 8i) {
                limit = 0.25f;
            }
            let _e79 = index;
            if (_e79 == 9i) {
                limit = 0.75f;
            }
            let _e83 = index;
            if (_e83 == 10i) {
                limit = 0.125f;
            }
            let _e87 = index;
            if (_e87 == 11i) {
                limit = 0.625f;
            }
            let _e91 = index;
            if (_e91 == 12i) {
                limit = 1f;
            }
            let _e95 = index;
            if (_e95 == 13i) {
                limit = 0.5f;
            }
            let _e99 = index;
            if (_e99 == 14i) {
                limit = 0.875f;
            }
            let _e103 = index;
            if (_e103 == 15i) {
                limit = 0.375f;
            }
        }
    }
    let _e107 = brightness_1;
    let _e108 = limit;
    if (_e107 < _e108) {
        local = 0f;
    } else {
        local = 1f;
    }
    let _e113 = local;
    return _e113;
}

fn pcg(v: u32) -> u32 {
    let s = (v * 747796405u) + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

// 0..1 hash of a cell and a frame
fn hash_cell(c: vec2<i32>, f: u32) -> f32 {
    let h = pcg(u32(c.x) ^ pcg(u32(c.y) + pcg(f)));
    return (f32(h >> 8u) + 0.5f) / 16777216f;
}

// Bayer matrix of 2^levels per side: threshold in (0, 1]
fn bayer(c: vec2<i32>, levels: i32) -> f32 {
    var v = 0u;
    for (var i = 0i; i < levels; i++) {
        let bit = u32((levels - 1i) - i);
        let xb = u32((c.x >> bit) & 1i);
        let yb = u32((c.y >> bit) & 1i);
        v += (((xb ^ yb) << 1u) | yb) << u32(2i * i);
    }
    return (f32(v) + 1f) / f32(1u << u32(2i * levels));
}

// 45 degree clustered-dot screen: lowest threshold at the dot centres
fn cluster_dot(p: vec2<f32>) -> f32 {
    let u = (p.x + p.y) / 8f;
    let v = (p.x - p.y) / 8f;
    let t = ((cos(2f * PI * u) + cos(2f * PI * v)) * 0.25f) + 0.5f;
    return ((1f - t) * 0.98f) + 0.02f;
}

fn dither_1(position_2: vec2<f32>, color_2: vec3<f32>, scale: f32) -> vec3<f32> {
    var position_3: vec2<f32>;
    var color_3: vec3<f32>;
    var scale_1: f32;
    var frame = 0u;
    var shift = vec2<i32>(0i, 0i);
    var limit: f32;

    position_3 = position_2;
    color_3 = color_2;
    scale_1 = scale;
    let lum = luma(color_3);
    let p = position_3 / vec2(scale_1);
    // animation: the pattern jumps by a hashed offset every frame
    if (global.b_animate > 0.5f) {
        frame = u32(global.animFrame);
        shift = vec2<i32>(i32(pcg((frame * 2u) + 1u) & 63u), i32(pcg((frame * 2u) + 2u) & 63u));
    }
    let mode = i32(global.modeIndex + 0.5f);
    if (mode == 0i) {
        // the original 4x4 table
        return vec3(dither(p + vec2<f32>(shift), lum));
    }
    let cell = vec2<i32>(floor(p));
    if (mode == 1i) {
        limit = bayer(cell + shift, 1i);
    } else if (mode == 2i) {
        limit = bayer(cell + shift, 3i);
    } else if (mode == 3i) {
        // blue noise: 12-bit rank split over red (high) and green (low nibble)
        let t = textureLoad(tNoise_t, (cell + shift) & vec2<i32>(NOISE_SIZE - 1i), 0);
        limit = (((round(t.x * 255f) * 16f) + (round(t.y * 255f) / 16f)) + 1f) / 4096f;
    } else if (mode == 4i) {
        limit = hash_cell(cell, frame);
    } else {
        limit = cluster_dot(p + vec2<f32>(shift));
    }
    return vec3(select(1f, 0f, lum < limit));
}

fn main_1() {
    var orig: vec4<f32>;
    var ditherCol: vec3<f32>;
    var outCol: vec3<f32>;

    let _e10 = vUv_1;
    let _e11 = textureSample(tDiffuse_t, tDiffuse_s, _e10);
    orig = _e11;
    let _e14 = gl_FragCoord_1;
    let _e16 = orig;
    let _e18 = global.scale;
    let _e19 = dither_1(_e14.xy, _e16.xyz, _e18);
    ditherCol = _e19;
    let _e21 = ditherCol;
    let _e22 = ditherCol;
    let _e23 = orig;
    let _e26 = global.colorize;
    outCol = mix(_e21, (_e22 * _e23.xyz), vec3(_e26));
    let _e30 = orig;
    let _e32 = outCol;
    let _e33 = global.amount;
    outCol = mix(_e30.xyz, _e32, vec3(_e33));
    let _e36 = outCol;
    let _e37 = orig;
    o_color = vec4<f32>(_e36.x, _e36.y, _e36.z, _e37.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>, @builtin(position) gl_FragCoord: vec4<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    gl_FragCoord_1 = gl_FragCoord;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
