struct U {
    resolution: vec2<f32>,
    delta: vec2<f32>,
    blurFac: f32,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var source_t: texture_2d<f32>;
@group(0) @binding(2) 
var source_s: sampler;

fn blur5_(uv: vec2<f32>, resolution: vec2<f32>, direction: vec2<f32>) -> vec4<f32> {
    var uv_1: vec2<f32>;
    var resolution_1: vec2<f32>;
    var direction_1: vec2<f32>;
    var color: vec4<f32> = vec4(0f);
    var off1_: vec2<f32>;

    uv_1 = uv;
    resolution_1 = resolution;
    direction_1 = direction;
    let _e21 = direction_1;
    off1_ = (vec2(1.3333334f) * _e21);
    let _e24 = color;
    let _e25 = uv_1;
    let _e26 = textureSample(source_t, source_s, _e25);
    color = (_e24 + (_e26 * 0.29411766f));
    let _e30 = color;
    let _e31 = uv_1;
    let _e32 = off1_;
    let _e33 = resolution_1;
    let _e36 = textureSample(source_t, source_s, (_e31 + (_e32 / _e33)));
    color = (_e30 + (_e36 * 0.3529412f));
    let _e40 = color;
    let _e41 = uv_1;
    let _e42 = off1_;
    let _e43 = resolution_1;
    let _e46 = textureSample(source_t, source_s, (_e41 - (_e42 / _e43)));
    color = (_e40 + (_e46 * 0.3529412f));
    let _e50 = color;
    return _e50;
}

fn main_1() {
    var b: vec4<f32>;

    let _e10 = vUv_1;
    let _e11 = global.resolution;
    let _e12 = global.delta;
    let _e13 = global.blurFac;
    let _e15 = blur5_(_e10, _e11, (_e12 * _e13));
    b = _e15;
    let _e17 = b;
    o_color = _e17;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
