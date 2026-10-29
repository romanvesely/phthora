struct U {
    amount: f32,
    count: f32,
    size: f32,
    fontCharCount: f32,
    colorize: f32,
    resolution: vec2<f32>,
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
var tFont_t: texture_2d<f32>;
@group(0) @binding(4) 
var tFont_s: sampler;

fn luma(color: vec3<f32>) -> f32 {
    var color_1: vec3<f32>;

    color_1 = color;
    let _e20 = color_1;
    return dot(_e20, vec3<f32>(0.299f, 0.587f, 0.114f));
}

fn pixelateUV(coord: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    var coord_1: vec2<f32>;
    var size_1: vec2<f32>;

    coord_1 = coord;
    size_1 = size;
    let _e22 = coord_1;
    let _e23 = size_1;
    let _e29 = size_1;
    return ((floor((_e22 / _e23)) + vec2(0.5f)) * _e29);
}

fn getInnerUV(coord_2: vec2<f32>, size_2: vec2<f32>) -> vec2<f32> {
    var coord_3: vec2<f32>;
    var size_3: vec2<f32>;

    coord_3 = coord_2;
    size_3 = size_2;
    let _e22 = coord_3;
    let _e23 = size_3;
    let _e28 = size_3;
    return ((_e22 - (floor((_e22 / _e23)) * _e23)) / _e28);
}

fn main_1() {
    var fontCharPixelSize: f32 = 8f;
    var pixDims: vec2<f32>;
    var pixelUV: vec2<f32>;
    var pixelInnerUV: vec2<f32>;
    var orig: vec4<f32>;
    var pixCol: vec4<f32>;
    var gray: f32;
    var charIndex: f32;
    var fontUV: vec2<f32>;
    var fontInnerUV: vec2<f32>;
    var fontColor: vec4<f32>;
    var base: vec4<f32>;
    var color_2: vec4<f32>;

    let _e20 = fontCharPixelSize;
    let _e21 = global.size;
    fontCharPixelSize = (_e20 * _e21);
    let _e23 = fontCharPixelSize;
    let _e24 = global.resolution;
    let _e27 = fontCharPixelSize;
    let _e28 = global.resolution;
    pixDims = vec2<f32>((_e23 / _e24.x), (_e27 / _e28.y));
    let _e33 = vUv_1;
    let _e34 = pixDims;
    let _e35 = pixelateUV(_e33, _e34);
    pixelUV = _e35;
    let _e37 = vUv_1;
    let _e38 = pixDims;
    let _e39 = getInnerUV(_e37, _e38);
    pixelInnerUV = _e39;
    let _e41 = vUv_1;
    let _e42 = textureSample(tDiffuse_t, tDiffuse_s, _e41);
    orig = _e42;
    let _e44 = pixelUV;
    let _e45 = textureSample(tDiffuse_t, tDiffuse_s, _e44);
    pixCol = _e45;
    let _e47 = pixCol;
    let _e49 = luma(_e47.xyz);
    gray = clamp(_e49, 0f, 1f);
    let _e54 = gray;
    let _e55 = global.fontCharCount;
    charIndex = floor(((_e54 * _e55) - 0.001f));
    let _e61 = charIndex;
    let _e62 = global.fontCharCount;
    fontUV = vec2<f32>((_e61 / _e62), 0f);
    let _e67 = pixelInnerUV;
    fontInnerUV = _e67;
    let _e70 = fontInnerUV;
    let _e72 = global.fontCharCount;
    fontInnerUV.x = (_e70.x / _e72);
    let _e74 = fontUV;
    let _e75 = fontInnerUV;
    let _e77 = textureSample(tFont_t, tFont_s, (_e74 + _e75));
    fontColor = _e77;
    let _e81 = pixCol;
    let _e82 = global.colorize;
    base = mix(vec4(1f), _e81, vec4(_e82));
    let _e86 = base;
    let _e87 = fontColor;
    color_2 = (_e86 * _e87);
    let _e90 = orig;
    let _e91 = color_2;
    let _e92 = global.amount;
    color_2 = mix(_e90, _e91, vec4(_e92));
    let _e95 = color_2;
    o_color = _e95;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e27 = o_color;
    return FragmentOutput(_e27);
}
