struct U {
    time: f32,
    speed: f32,
    random: f32,
    amount: f32,
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
    return fract((sin(_e16) * 10000f));
}

fn noise1d(p: f32) -> f32 {
    var p_1: f32;
    var i: f32;
    var f: f32;
    var u: f32;

    p_1 = p;
    let _e16 = p_1;
    i = floor(_e16);
    let _e19 = p_1;
    f = fract(_e19);
    let _e22 = f;
    let _e23 = f;
    let _e27 = f;
    u = ((_e22 * _e23) * (3f - (2f * _e27)));
    let _e32 = i;
    let _e33 = random1d(_e32);
    let _e34 = i;
    let _e37 = random1d((_e34 + 1f));
    let _e38 = u;
    return mix(_e33, _e37, _e38);
}

fn main_1() {
    var orig: vec4<f32>;
    var noiseDark: f32;
    var cosDark: f32;
    var darkness: f32;
    var col: vec3<f32>;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    orig = _e15;
    let _e17 = global.time;
    let _e18 = global.speed;
    let _e22 = global.seed;
    let _e26 = noise1d((((_e17 * _e18) * 10f) + (_e22 * 8.783f)));
    noiseDark = _e26;
    let _e28 = global.time;
    let _e29 = global.speed;
    let _e33 = global.seed;
    cosDark = cos((((_e28 * _e29) * 10f) + (_e33 * 8.783f)));
    let _e39 = cosDark;
    let _e40 = noiseDark;
    let _e41 = global.random;
    darkness = mix(_e39, _e40, _e41);
    let _e44 = orig;
    let _e46 = orig;
    let _e48 = darkness;
    let _e50 = global.amount;
    col = mix(_e44.xyz, (_e46.xyz * _e48), vec3(_e50));
    let _e54 = col;
    let _e55 = orig;
    o_color = vec4<f32>(_e54.x, _e54.y, _e54.z, _e55.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
