struct U {
    feather: f32,
    roundness: f32,
    color: vec3<f32>,
    amount: f32,
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

fn sdRoundBox(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    var p_1: vec2<f32>;
    var b_1: vec2<f32>;
    var r_1: f32;
    var q: vec2<f32>;

    p_1 = p;
    b_1 = b;
    r_1 = r;
    let _e18 = p_1;
    let _e20 = b_1;
    let _e22 = r_1;
    q = ((abs(_e18) - _e20) + vec2(_e22));
    let _e26 = q;
    let _e28 = q;
    let _e33 = q;
    let _e39 = r_1;
    return ((min(max(_e26.x, _e28.y), 0f) + length(max(_e33, vec2(0f)))) - _e39);
}

fn main_1() {
    var texel: vec4<f32>;
    var feather2_: f32;
    var amount2_: f32;
    var size: f32;
    var d: f32;
    var vig: f32;

    let _e12 = vUv_1;
    let _e13 = textureSample(tDiffuse_t, tDiffuse_s, _e12);
    texel = _e13;
    let _e15 = global.feather;
    feather2_ = (_e15 / 3f);
    let _e19 = global.amount;
    amount2_ = ((_e19 - 0.5f) * 0.25f);
    let _e26 = feather2_;
    let _e30 = amount2_;
    size = ((0.5f - (_e26 * 0.5f)) - _e30);
    let _e33 = vUv_1;
    let _e37 = size;
    let _e39 = size;
    let _e40 = global.roundness;
    let _e42 = sdRoundBox((_e33 - vec2(0.5f)), vec2(_e37), (_e39 * _e40));
    d = _e42;
    let _e45 = feather2_;
    let _e46 = d;
    vig = smoothstep(0f, _e45, _e46);
    let _e49 = texel;
    let _e51 = global.color;
    let _e52 = vig;
    let _e54 = mix(_e49.xyz, _e51, vec3(_e52));
    let _e55 = texel;
    o_color = vec4<f32>(_e54.x, _e54.y, _e54.z, _e55.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
