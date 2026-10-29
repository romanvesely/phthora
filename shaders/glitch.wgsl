struct U {
    amount: f32,
    colorSplit: f32,
    time: f32,
    scale: f32,
    speed: f32,
    seed: f32,
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
var tDisp_t: texture_2d<f32>;
@group(0) @binding(4) 
var tDisp_s: sampler;

fn random1d(n: f32) -> f32 {
    var n_1: f32;

    n_1 = n;
    let _e20 = n_1;
    return fract((sin(_e20) * 43758.547f));
}

fn random2d(st: vec2<f32>) -> f32 {
    var st_1: vec2<f32>;

    st_1 = st;
    let _e20 = st_1;
    return fract((sin(dot(_e20.xy, vec2<f32>(12.9898f, 78.233f))) * 43758.547f));
}

fn main_1() {
    var scale2_: f32;
    var amount2_: f32;
    var fps: f32;
    var stepTime: f32;
    var seed: f32;
    var seed_x: f32;
    var seed_y: f32;
    var distortion_x: f32;
    var distortion_y: f32;
    var p: vec2<f32>;
    var numCells: f32;
    var ij: vec2<f32>;
    var cellId: f32;
    var normal: vec4<f32>;
    var angle: f32;
    var offset: vec2<f32>;
    var cr: vec4<f32>;
    var cga: vec4<f32>;
    var cb: vec4<f32>;

    let _e20 = global.scale;
    scale2_ = mix(0.1f, 4f, _e20);
    let _e25 = global.amount;
    amount2_ = mix(0f, 1f, _e25);
    let _e30 = global.speed;
    fps = (mix(0f, 60f, _e30) * 4f);
    let _e35 = global.time;
    let _e36 = fps;
    let _e38 = global.seed;
    stepTime = floor(((_e35 * _e36) + (_e38 * 391.48f)));
    let _e44 = stepTime;
    let _e45 = random1d(_e44);
    seed = _e45;
    let _e47 = stepTime;
    let _e50 = random1d((_e47 + 62.97f));
    seed_x = ((_e50 * 2f) - 1f);
    let _e56 = stepTime;
    let _e59 = random1d((_e56 + 93.97f));
    seed_y = ((_e59 * 2f) - 1f);
    let _e65 = stepTime;
    let _e68 = random1d((_e65 + 3.7f));
    distortion_x = _e68;
    let _e70 = stepTime;
    let _e73 = random1d((_e70 + 43.67f));
    distortion_y = _e73;
    let _e75 = vUv_1;
    p = _e75;
    let _e77 = p;
    p = (_e77 - vec2(0.5f));
    let _e81 = p;
    let _e82 = scale2_;
    p = (_e81 * _e82);
    let _e84 = p;
    p = (_e84 + vec2(0.5f));
    let _e88 = p;
    p = fract(_e88);
    let _e91 = scale2_;
    numCells = (6f * _e91);
    let _e94 = vUv_1;
    let _e95 = numCells;
    let _e98 = numCells;
    ij = (floor((_e94 * _e95)) / vec2(_e98));
    let _e102 = ij;
    let _e104 = numCells;
    let _e106 = ij;
    cellId = ((_e102.x * _e104) + _e106.y);
    let _e110 = stepTime;
    let _e111 = cellId;
    let _e113 = random1d((_e110 + _e111));
    seed = _e113;
    let _e114 = p;
    let _e115 = seed;
    let _e117 = seed;
    let _e119 = textureSample(tDisp_t, tDisp_s, ((_e114 * _e115) * _e117));
    normal = (_e119 - vec4(0.5f));
    let _e124 = vUv_1;
    p = _e124;
    let _e126 = p;
    let _e128 = normal;
    let _e130 = seed_x;
    let _e132 = seed;
    let _e136 = amount2_;
    p.x = (_e126.x + (((_e128.x * _e130) * (_e132 / 5f)) * _e136));
    let _e140 = p;
    let _e142 = normal;
    let _e144 = seed_y;
    let _e146 = seed;
    let _e150 = amount2_;
    p.y = (_e140.y + (((_e142.y * _e144) * (_e146 / 5f)) * _e150));
    let _e153 = normal;
    let _e155 = stepTime;
    let _e158 = random2d((_e153.xy + vec2(_e155)));
    angle = ((_e158 * 2f) * 3.1415927f);
    let _e164 = amount2_;
    let _e165 = global.colorSplit;
    let _e169 = angle;
    let _e171 = angle;
    offset = (((_e164 * _e165) * 0.1f) * vec2<f32>(cos(_e169), sin(_e171)));
    let _e176 = p;
    let _e177 = offset;
    let _e179 = textureSample(tDiffuse_t, tDiffuse_s, (_e176 + _e177));
    cr = _e179;
    let _e181 = p;
    let _e182 = textureSample(tDiffuse_t, tDiffuse_s, _e181);
    cga = _e182;
    let _e184 = p;
    let _e185 = offset;
    let _e187 = textureSample(tDiffuse_t, tDiffuse_s, (_e184 - _e185));
    cb = _e187;
    let _e189 = cr;
    let _e191 = cga;
    let _e193 = cb;
    let _e195 = cga;
    o_color = vec4<f32>(_e189.x, _e191.y, _e193.z, _e195.w);
    return;
}

@fragment 
fn main(@location(0) vUv: vec2<f32>) -> FragmentOutput {
    vUv_1 = vUv;
    main_1();
    let _e27 = o_color;
    return FragmentOutput(_e27);
}
