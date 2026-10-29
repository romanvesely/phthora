struct U {
    side: i32,
    position2_: f32,
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
    var p: vec2<f32>;
    var orig: vec4<f32>;
    var color: vec4<f32>;

    let _e10 = vUv_1;
    p = _e10;
    let _e12 = p;
    let _e13 = textureSample(tDiffuse_t, tDiffuse_s, _e12);
    orig = _e13;
    let _e15 = global.side;
    if (_e15 == 0i) {
        {
            let _e18 = p;
            let _e20 = global.position2_;
            if (_e18.x > _e20) {
                let _e23 = global.position2_;
                let _e26 = p;
                p.x = fract(((_e23 * 2f) - _e26.x));
            }
        }
    } else {
        let _e30 = global.side;
        if (_e30 == 1i) {
            {
                let _e33 = p;
                let _e35 = global.position2_;
                if (_e33.x < _e35) {
                    let _e38 = global.position2_;
                    let _e41 = p;
                    p.x = fract(((_e38 * 2f) - _e41.x));
                }
            }
        } else {
            let _e45 = global.side;
            if (_e45 == 2i) {
                {
                    let _e48 = p;
                    let _e50 = global.position2_;
                    if (_e48.y < _e50) {
                        let _e53 = global.position2_;
                        let _e56 = p;
                        p.y = fract(((_e53 * 2f) - _e56.y));
                    }
                }
            } else {
                let _e60 = global.side;
                if (_e60 == 3i) {
                    {
                        let _e63 = p;
                        let _e65 = global.position2_;
                        if (_e63.y > _e65) {
                            let _e68 = global.position2_;
                            let _e71 = p;
                            p.y = fract(((_e68 * 2f) - _e71.y));
                        }
                    }
                }
            }
        }
    }
    let _e75 = p;
    let _e76 = textureSample(tDiffuse_t, tDiffuse_s, _e75);
    color = _e76;
    let _e78 = orig;
    let _e79 = color;
    let _e80 = global.amount;
    color = mix(_e78, _e79, vec4(_e80));
    let _e83 = color;
    o_color = _e83;
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
