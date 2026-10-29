struct U {
    resolution: vec2<f32>,
    count: f32,
    size: f32,
    blur: f32,
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

fn main_1() {
    var orig: vec4<f32>;
    var cells: vec2<f32>;
    var uvCell: vec2<f32>;
    var uvInner: vec2<f32>;
    var cellSize: f32;
    var col: vec4<f32>;
    var d: f32;

    let _e14 = vUv_1;
    let _e15 = textureSample(tDiffuse_t, tDiffuse_s, _e14);
    orig = _e15;
    let _e17 = global.count;
    let _e18 = global.count;
    let _e19 = global.resolution;
    let _e22 = global.resolution;
    cells = vec2<f32>(_e17, ((_e18 * _e19.y) / _e22.x));
    let _e27 = vUv_1;
    let _e28 = cells;
    let _e34 = cells;
    uvCell = ((floor((_e27 * _e28)) + vec2(0.5f)) / _e34);
    let _e37 = vUv_1;
    let _e38 = uvCell;
    let _e40 = cells;
    uvInner = ((_e37 - _e38) * _e40);
    let _e44 = global.count;
    cellSize = (1f / _e44);
    let _e47 = uvCell;
    let _e48 = textureSample(tDiffuse_t, tDiffuse_s, _e47);
    col = _e48;
    let _e50 = uvInner;
    d = distance(_e50, vec2(0f));
    let _e55 = global.size;
    let _e56 = global.size;
    let _e57 = global.blur;
    let _e59 = d;
    d = smoothstep(_e55, (_e56 + _e57), _e59);
    let _e61 = col;
    let _e64 = d;
    col = mix(_e61, vec4(0f), vec4(_e64));
    let _e67 = orig;
    let _e68 = col;
    let _e69 = global.amount;
    col = mix(_e67, _e68, vec4(_e69));
    let _e72 = col;
    o_color = _e72;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e21 = o_color;
    return FragmentOutput(_e21);
}
