struct U {
    amount: f32,
    angle: f32,
    mode: f32,
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

fn modeOff(id: f32) -> f32 {
    var id_1: f32;

    id_1 = id;
    let _e12 = id_1;
    return ((_e12 - (floor((_e12 / 3f)) * 3f)) - 1f);
}

fn main_1() {
    var a: f32;
    var offset: vec2<f32>;
    var cr: vec4<f32>;
    var cga: vec4<f32>;
    var cb: vec4<f32>;
    var alpha: f32;

    let _e10 = global.angle;
    a = (_e10 * 6.2831855f);
    let _e14 = global.amount;
    let _e17 = a;
    let _e19 = a;
    offset = ((_e14 * 0.06f) * vec2<f32>(cos(_e17), sin(_e19)));
    let _e24 = vUv_1;
    let _e25 = offset;
    let _e26 = global.mode;
    let _e27 = modeOff(_e26);
    let _e30 = textureSample(tDiffuse_t, tDiffuse_s, (_e24 + (_e25 * _e27)));
    cr = _e30;
    let _e32 = vUv_1;
    let _e33 = offset;
    let _e34 = global.mode;
    let _e37 = modeOff((_e34 + 1f));
    let _e40 = textureSample(tDiffuse_t, tDiffuse_s, (_e32 + (_e33 * _e37)));
    cga = _e40;
    let _e42 = vUv_1;
    let _e43 = offset;
    let _e44 = global.mode;
    let _e47 = modeOff((_e44 + 2f));
    let _e50 = textureSample(tDiffuse_t, tDiffuse_s, (_e42 + (_e43 * _e47)));
    cb = _e50;
    let _e52 = cr;
    let _e54 = cga;
    let _e56 = cb;
    alpha = max(_e52.w, max(_e54.w, _e56.w));
    let _e61 = cr;
    let _e63 = cga;
    let _e65 = cb;
    let _e67 = alpha;
    o_color = vec4<f32>(_e61.x, _e63.y, _e65.z, _e67);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e17 = o_color;
    return FragmentOutput(_e17);
}
