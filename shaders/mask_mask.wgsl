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
    colUnder = _e15.xyzw;
    let _e18 = vUv_1;
    let _e19 = textureSample(tDiffuse_t, tDiffuse_s, _e18);
    colOver = _e19.xyzw;
    let _e22 = vUv_1;
    let _e23 = textureSample(tMask_t, tMask_s, _e22);
    mask = _e23.x;
    let _e29 = global.b_showMask;
    if (_e29 > 0.5f) {
        {
            let _e32 = colUnder;
            let _e38 = mask;
            col = mix(_e32, vec4<f32>(1f, 0f, 0f, 1f), vec4(_e38));
        }
    } else {
        {
            let _e41 = global.b_invert;
            if (_e41 > 0.5f) {
                let _e45 = mask;
                mask = (1f - _e45);
            }
            let _e47 = colUnder;
            let _e48 = colOver;
            let _e49 = mask;
            col = mix(_e47, _e48, vec4(_e49));
        }
    }
    let _e52 = colOver;
    let _e53 = col;
    let _e54 = global.amount;
    col = mix(_e52, _e53, vec4(_e54));
    let _e57 = col;
    o_color = _e57;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e25 = o_color;
    return FragmentOutput(_e25);
}
