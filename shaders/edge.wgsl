struct U {
    amount: f32,
    passthru: f32,
    b_invert: f32,
    thickness: f32,
    resolution: vec2<f32>,
    edgeCol: vec3<f32>,
}

struct FragmentOutput {
    @location(0) o_color: vec4<f32>,
}

const g0_: mat3x3<f32> = mat3x3<f32>(vec3<f32>(1f, 2f, 1f), vec3<f32>(0f, 0f, 0f), vec3<f32>(-1f, -2f, -1f));
const g1_: mat3x3<f32> = mat3x3<f32>(vec3<f32>(1f, 0f, -1f), vec3<f32>(2f, 0f, -2f), vec3<f32>(1f, 0f, -1f));

var<private> vUv_1: vec2<f32>;
var<private> o_color: vec4<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tDiffuse_t: texture_2d<f32>;
@group(0) @binding(2) 
var tDiffuse_s: sampler;
var<private> G: array<mat3x3<f32>, 2>;

fn blendNormal(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
    var base_1: vec3<f32>;
    var blend_1: vec3<f32>;

    base_1 = base;
    blend_1 = blend;
    let _e23 = blend_1;
    return _e23;
}

fn blendNormal_1(base_2: vec3<f32>, blend_2: vec3<f32>, opacity: f32) -> vec3<f32> {
    var base_3: vec3<f32>;
    var blend_3: vec3<f32>;
    var opacity_1: f32;

    base_3 = base_2;
    blend_3 = blend_2;
    opacity_1 = opacity;
    let _e25 = base_3;
    let _e26 = blend_3;
    let _e27 = blendNormal(_e25, _e26);
    let _e28 = opacity_1;
    let _e30 = base_3;
    let _e32 = opacity_1;
    return ((_e27 * _e28) + (_e30 * (1f - _e32)));
}

fn main_1() {
    var texel: vec2<f32>;
    var I: mat3x3<f32>;
    var cnv: array<f32, 2>;
    var sample_v: vec3<f32>;
    var i: f32 = 0f;
    var j: f32;
    var i_1: i32 = 0i;
    var dp3_: f32;
    var orig: vec4<f32>;
    var edgeAmt: f32;
    var blendOpac: f32;

    let _e20 = global.resolution;
    let _e24 = global.resolution;
    texel = vec2<f32>((1f / _e20.x), (1f / _e24.y));
    let _e29 = texel;
    let _e30 = global.thickness;
    texel = (_e29 * _e30);
    G[0i] = g0_;
    G[1i] = g1_;
    loop {
        let _e41 = i;
        if !((_e41 < 3f)) {
            break;
        }
        j = 0f;
        loop {
            let _e50 = j;
            if !((_e50 < 3f)) {
                break;
            }
            {
                let _e57 = vUv_1;
                let _e58 = texel;
                let _e59 = i;
                let _e62 = j;
                let _e68 = textureSample(tDiffuse_t, tDiffuse_s, (_e57 + (_e58 * vec2<f32>((_e59 - 1f), (_e62 - 1f)))));
                sample_v = _e68.xyz;
                let _e70 = j;
                let _e72 = i;
                let _e76 = sample_v;
                I[i32(_e72)][i32(_e70)] = length(_e76);
            }
            continuing {
                let _e54 = j;
                j = (_e54 + 1f);
            }
        }
        continuing {
            let _e45 = i;
            i = (_e45 + 1f);
        }
    }
    loop {
        let _e80 = i_1;
        if !((_e80 < 2i)) {
            break;
        }
        {
            let _e88 = i_1;
            let _e91 = G[_e88][0];
            let _e94 = I[0];
            let _e97 = i_1;
            let _e100 = G[_e97][1];
            let _e103 = I[1];
            let _e107 = i_1;
            let _e110 = G[_e107][2];
            let _e113 = I[2];
            dp3_ = ((dot(_e91, _e94) + dot(_e100, _e103)) + dot(_e110, _e113));
            let _e117 = i_1;
            let _e119 = dp3_;
            let _e120 = dp3_;
            cnv[_e117] = (_e119 * _e120);
        }
        continuing {
            let _e84 = i_1;
            i_1 = (_e84 + 1i);
        }
    }
    let _e122 = vUv_1;
    let _e123 = textureSample(tDiffuse_t, tDiffuse_s, _e122);
    orig = _e123;
    let _e125 = orig;
    let _e126 = global.passthru;
    orig = (_e125 * _e126);
    let _e131 = cnv[0];
    let _e134 = cnv[0];
    let _e138 = cnv[1];
    let _e141 = cnv[1];
    edgeAmt = (0.5f * sqrt(((_e131 * _e134) + (_e138 * _e141))));
    let _e147 = global.amount;
    let _e148 = edgeAmt;
    blendOpac = clamp((_e147 * _e148), 0f, 1f);
    let _e154 = o_color;
    let _e156 = orig;
    let _e158 = global.edgeCol;
    let _e159 = blendOpac;
    let _e160 = blendNormal_1(_e156.xyz, _e158, _e159);
    o_color.x = _e160.x;
    o_color.y = _e160.y;
    o_color.z = _e160.z;
    let _e168 = orig;
    let _e170 = blendOpac;
    o_color.w = clamp((_e168.w + _e170), 0f, 1f);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e29 = o_color;
    return FragmentOutput(_e29);
}
