struct U {
    b_showMask: f32,
    b_invert: f32,
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
var tOrig_t: texture_2d<f32>;
@group(0) @binding(2) 
var tOrig_s: sampler;
@group(0) @binding(3) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(4) 
var tDiffuse_s: sampler;
@group(0) @binding(5) 
var tMask_t: texture_2d<f32>;
@group(0) @binding(6) 
var tMask_s: sampler;

fn main_1() {
    var colUnder: vec4<f32>;
    var colOver: vec4<f32>;
    var mask: f32;
    var col: vec4<f32> = vec4(1f);

    let _e14 = vUv_1;
    let _e15 = textureSample(tOrig_t, tOrig_s, _e14);
    colUnder = _e15;
    let _e17 = vUv_1;
    let _e18 = textureSample(tDiffuse_t, tDiffuse_s, _e17);
    colOver = _e18;
    let _e20 = vUv_1;
    let _e21 = textureSample(tMask_t, tMask_s, _e20);
    mask = _e21.x;
    let _e27 = global.b_showMask;
    if (_e27 > 0.5f) {
        {
            let _e30 = colUnder;
            let _e36 = mask;
            col = mix(_e30, vec4<f32>(1f, 0f, 0f, 1f), vec4(_e36));
        }
    } else {
        {
            let _e39 = global.b_invert;
            if (_e39 > 0.5f) {
                let _e43 = mask;
                mask = (1f - _e43);
            }
            let _e45 = colUnder;
            let _e46 = colOver;
            let _e47 = mask;
            col = mix(_e45, _e46, vec4(_e47));
        }
    }
    let _e50 = colOver;
    let _e51 = col;
    let _e52 = global.amount;
    col = mix(_e50, _e51, vec4(_e52));
    let _e55 = col;
    o_color = _e55;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
