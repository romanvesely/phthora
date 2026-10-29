struct U {
    pixelsX: f32,
    pixelsY: f32,
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
    var normCoord: vec2<f32>;
    var r: f32;
    var phi: f32;
    var textureCoordinateToUse: vec2<f32>;
    var color: vec4<f32>;

    let _e10 = vUv_1;
    let _e11 = textureSample(tDiffuse_t, tDiffuse_s, _e10);
    orig = _e11;
    let _e14 = vUv_1;
    normCoord = ((2f * _e14) - vec2(1f));
    let _e20 = normCoord;
    r = length(_e20);
    let _e23 = normCoord;
    let _e25 = normCoord;
    phi = atan2(_e23.y, _e25.x);
    let _e29 = r;
    let _e30 = r;
    let _e31 = global.pixelsX;
    r = ((_e29 - (_e30 - (floor((_e30 / _e31)) * _e31))) + 0.03f);
    let _e39 = phi;
    let _e40 = phi;
    let _e41 = global.pixelsY;
    phi = (_e39 - (_e40 - (floor((_e40 / _e41)) * _e41)));
    let _e48 = r;
    let _e49 = phi;
    normCoord.x = (_e48 * cos(_e49));
    let _e53 = r;
    let _e54 = phi;
    normCoord.y = (_e53 * sin(_e54));
    let _e57 = normCoord;
    textureCoordinateToUse = ((_e57 / vec2(2f)) + vec2(0.5f));
    let _e65 = textureCoordinateToUse;
    let _e66 = textureSample(tDiffuse_t, tDiffuse_s, _e65);
    color = _e66;
    let _e68 = orig;
    let _e69 = color;
    let _e70 = global.amount;
    color = mix(_e68, _e69, vec4(_e70));
    let _e73 = color;
    o_color = _e73;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
