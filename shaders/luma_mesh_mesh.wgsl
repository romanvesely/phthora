struct U {
    opacity: f32,
    seed: f32,
    b_wireframe: f32,
    gridColor: vec3<f32>,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tInput_t: texture_2d<f32>;
@group(0) @binding(2) 
var tInput_s: sampler;

fn luma(color: vec3<f32>) -> f32 {
    var color_1: vec3<f32>;

    color_1 = color;
    let _e14 = color_1;
    return dot(_e14, vec3<f32>(0.299f, 0.587f, 0.114f));
}

fn main_1() {
    var col: vec4<f32>;
    var luma_1: f32;

    let _e12 = vUv_1;
    let _e13 = textureSample(tInput_t, tInput_s, _e12);
    col = _e13;
    let _e15 = global.b_wireframe;
    if (_e15 > 0.5f) {
        {
            let _e18 = col;
            let _e20 = luma(_e18.xyz);
            luma_1 = _e20;
            let _e22 = global.gridColor;
            let _e23 = luma_1;
            let _e24 = (_e22 * _e23);
            let _e25 = luma_1;
            col = vec4<f32>(_e24.x, _e24.y, _e24.z, _e25);
        }
    }
    let _e30 = col;
    let _e31 = _e30.xyz;
    let _e32 = col;
    let _e34 = global.opacity;
    o_color = vec4<f32>(_e31.x, _e31.y, _e31.z, (_e32.w * _e34));
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
