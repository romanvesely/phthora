struct U {
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
@group(0) @binding(3) 
var tLUT_t: texture_2d<f32>;
@group(0) @binding(4) 
var tLUT_s: sampler;

fn main_1() {
    var col: vec4<f32>;
    var blueColor: f32;
    var quad1_: vec2<f32>;
    var quad2_: vec2<f32>;
    var texPos1_: vec2<f32>;
    var texPos2_: vec2<f32>;
    var newColor1_: vec4<f32>;
    var newColor2_: vec4<f32>;
    var newColor: vec4<f32>;

    let _e8 = vUv_1;
    let _e9 = textureSample(tDiffuse_t, tDiffuse_s, _e8);
    col = _e9;
    let _e11 = col;
    blueColor = (_e11.z * 63f);
    let _e18 = blueColor;
    quad1_.y = floor((floor(_e18) / 8f));
    let _e24 = blueColor;
    let _e26 = quad1_;
    quad1_.x = (floor(_e24) - (_e26.y * 8f));
    let _e33 = blueColor;
    quad2_.y = floor((ceil(_e33) / 8f));
    let _e39 = blueColor;
    let _e41 = quad2_;
    quad2_.x = (ceil(_e39) - (_e41.y * 8f));
    let _e48 = quad1_;
    let _e61 = col;
    texPos1_.x = (((_e48.x * 0.125f) + 0.0009765625f) + (0.123046875f * _e61.x));
    let _e66 = quad1_;
    let _e79 = col;
    texPos1_.y = (((_e66.y * 0.125f) + 0.0009765625f) + (0.123046875f * _e79.y));
    let _e85 = texPos1_;
    texPos1_.y = (1f - _e85.y);
    let _e90 = quad2_;
    let _e103 = col;
    texPos2_.x = (((_e90.x * 0.125f) + 0.0009765625f) + (0.123046875f * _e103.x));
    let _e108 = quad2_;
    let _e121 = col;
    texPos2_.y = (((_e108.y * 0.125f) + 0.0009765625f) + (0.123046875f * _e121.y));
    let _e127 = texPos2_;
    texPos2_.y = (1f - _e127.y);
    let _e130 = texPos1_;
    let _e131 = textureSample(tLUT_t, tLUT_s, _e130);
    newColor1_ = _e131;
    let _e133 = texPos2_;
    let _e134 = textureSample(tLUT_t, tLUT_s, _e133);
    newColor2_ = _e134;
    let _e136 = newColor1_;
    let _e137 = newColor2_;
    let _e138 = blueColor;
    newColor = mix(_e136, _e137, vec4(fract(_e138)));
    let _e143 = col;
    let _e144 = newColor;
    let _e145 = _e144.xyz;
    let _e146 = col;
    let _e152 = global.amount;
    o_color = mix(_e143, vec4<f32>(_e145.x, _e145.y, _e145.z, _e146.w), vec4(_e152));
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
