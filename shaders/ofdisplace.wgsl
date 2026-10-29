struct U {
    resolution: vec2<f32>,
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
var tMap_t: texture_2d<f32>;
@group(0) @binding(4) 
var tMap_s: sampler;

fn main_1() {
    var uv: vec2<f32>;
    var d: vec2<f32>;
    var left_coord: vec2<f32>;
    var right_coord: vec2<f32>;
    var above_coord: vec2<f32>;
    var below_coord: vec2<f32>;
    var lefta_coord: vec2<f32>;
    var righta_coord: vec2<f32>;
    var leftb_coord: vec2<f32>;
    var rightb_coord: vec2<f32>;
    var color: vec4<f32>;
    var colorL: vec4<f32>;
    var colorR: vec4<f32>;
    var colorA: vec4<f32>;
    var colorB: vec4<f32>;
    var colorLA: vec4<f32>;
    var colorRA: vec4<f32>;
    var colorLB: vec4<f32>;
    var colorRB: vec4<f32>;
    var blurVector: vec4<f32>;
    var blurAmount: vec2<f32>;
    var tmp: vec2<f32>;
    var sample0_: vec4<f32>;

    let _e10 = vUv_1;
    uv = _e10;
    let _e13 = global.resolution;
    d = (vec2(4f) / _e13);
    let _e17 = uv;
    let _e18 = d;
    left_coord = clamp(vec2<f32>((_e17 + vec2<f32>(-(_e18.x), 0f))), vec2(0f), vec2(1f));
    let _e32 = uv;
    let _e33 = d;
    right_coord = clamp(vec2<f32>((_e32 + vec2<f32>(_e33.x, 0f))), vec2(0f), vec2(1f));
    let _e46 = uv;
    let _e48 = d;
    above_coord = clamp(vec2<f32>((_e46 + vec2<f32>(0f, _e48.y))), vec2(0f), vec2(1f));
    let _e60 = uv;
    let _e62 = d;
    below_coord = clamp(vec2<f32>((_e60 + vec2<f32>(0f, -(_e62.y)))), vec2(0f), vec2(1f));
    let _e75 = uv;
    let _e76 = d;
    let _e79 = d;
    lefta_coord = clamp(vec2<f32>((_e75 + vec2<f32>(-(_e76.x), _e79.x))), vec2(0f), vec2(1f));
    let _e90 = uv;
    let _e91 = d;
    let _e93 = d;
    righta_coord = clamp(vec2<f32>((_e90 + vec2<f32>(_e91.x, _e93.x))), vec2(0f), vec2(1f));
    let _e104 = uv;
    let _e105 = d;
    let _e108 = d;
    leftb_coord = clamp(vec2<f32>((_e104 + vec2<f32>(-(_e105.x), -(_e108.x)))), vec2(0f), vec2(1f));
    let _e120 = uv;
    let _e121 = d;
    let _e123 = d;
    rightb_coord = clamp(vec2<f32>((_e120 + vec2<f32>(_e121.x, -(_e123.x)))), vec2(0f), vec2(1f));
    let _e135 = uv;
    let _e136 = textureSample(tMap_t, tMap_s, _e135);
    color = _e136;
    let _e138 = left_coord;
    let _e139 = textureSample(tMap_t, tMap_s, _e138);
    colorL = _e139;
    let _e141 = right_coord;
    let _e142 = textureSample(tMap_t, tMap_s, _e141);
    colorR = _e142;
    let _e144 = above_coord;
    let _e145 = textureSample(tMap_t, tMap_s, _e144);
    colorA = _e145;
    let _e147 = below_coord;
    let _e148 = textureSample(tMap_t, tMap_s, _e147);
    colorB = _e148;
    let _e150 = lefta_coord;
    let _e151 = textureSample(tMap_t, tMap_s, _e150);
    colorLA = _e151;
    let _e153 = righta_coord;
    let _e154 = textureSample(tMap_t, tMap_s, _e153);
    colorRA = _e154;
    let _e156 = leftb_coord;
    let _e157 = textureSample(tMap_t, tMap_s, _e156);
    colorLB = _e157;
    let _e159 = rightb_coord;
    let _e160 = textureSample(tMap_t, tMap_s, _e159);
    colorRB = _e160;
    let _e162 = color;
    let _e163 = colorL;
    let _e165 = colorR;
    let _e167 = colorA;
    let _e169 = colorB;
    let _e171 = colorLA;
    let _e173 = colorRA;
    let _e175 = colorLB;
    let _e177 = colorRB;
    blurVector = (((((((((_e162 + _e163) + _e165) + _e167) + _e169) + _e171) + _e173) + _e175) + _e177) / vec4(9f));
    let _e183 = blurVector;
    let _e185 = blurVector;
    let _e188 = blurVector;
    let _e190 = blurVector;
    blurAmount = vec2<f32>((_e183.y - _e185.x), (_e188.w - _e190.z));
    let _e195 = uv;
    let _e196 = blurAmount;
    let _e197 = global.amount;
    tmp = (_e195 + (_e196 * _e197));
    let _e201 = tmp;
    tmp = clamp(_e201, vec2(0f), vec2(1f));
    let _e207 = tmp;
    let _e208 = textureSample(tDiffuse_t, tDiffuse_s, _e207);
    sample0_ = _e208;
    let _e210 = sample0_;
    o_color = _e210;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e19 = o_color;
    return FragmentOutput(_e19);
}
