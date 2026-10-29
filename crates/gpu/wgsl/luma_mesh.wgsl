struct U {
    modelViewMatrix: mat4x4<f32>,
    projectionMatrix: mat4x4<f32>,
    gridColor: vec3<f32>,
    time: f32,
    seed: f32,
    lumaHeight: f32,
    bandsScl: f32,
    bandsHeight: f32,
    warpScale: f32,
    warpHeight: f32,
    b_lumaInvert: f32,
    opacity: f32,
    b_wireframe: f32,
    // lines and points: premultiplied output, target size in pixels, strip layout
    b_premul: f32,
    segs: f32,
    lineCount: f32,
    lineWidth: f32,
    lineSoft: f32,
    pointSize: f32,
    viewport: vec2<f32>,
}

struct VertexOutput {
    @location(0) vUv: vec2<f32>,
    @builtin(position) gl_Position: vec4<f32>,
}

var<private> position_1: vec3<f32>;
var<private> uv_1: vec2<f32>;
@group(0) @binding(0) 
var<uniform> global: U;
@group(0) @binding(1) 
var tInput_t: texture_2d<f32>;
@group(0) @binding(2) 
var tInput_s: sampler;
var<private> vUv: vec2<f32>;
var<private> gl_Position: vec4<f32>;

fn mod289_(x: vec3<f32>) -> vec3<f32> {
    var x_1: vec3<f32>;

    x_1 = x;
    let _e33 = x_1;
    let _e34 = x_1;
    return (_e33 - (floor((_e34 * 0.0034602077f)) * 289f));
}

fn mod289_1(x_2: vec4<f32>) -> vec4<f32> {
    var x_3: vec4<f32>;

    x_3 = x_2;
    let _e33 = x_3;
    let _e34 = x_3;
    return (_e33 - (floor((_e34 * 0.0034602077f)) * 289f));
}

fn permute(x_4: vec4<f32>) -> vec4<f32> {
    var x_5: vec4<f32>;

    x_5 = x_4;
    let _e33 = x_5;
    let _e39 = x_5;
    let _e41 = mod289_1((((_e33 * 34f) + vec4(1f)) * _e39));
    return _e41;
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    var r_1: vec4<f32>;

    r_1 = r;
    let _e35 = r_1;
    return (vec4(1.7928429f) - (0.85373473f * _e35));
}

fn noise3d(v: vec3<f32>) -> f32 {
    var v_1: vec3<f32>;
    var C: vec2<f32> = vec2<f32>(0.16666667f, 0.33333334f);
    var D: vec4<f32> = vec4<f32>(0f, 0.5f, 1f, 2f);
    var i: vec3<f32>;
    var x0_: vec3<f32>;
    var g: vec3<f32>;
    var l: vec3<f32>;
    var i1_: vec3<f32>;
    var i2_: vec3<f32>;
    var x1_: vec3<f32>;
    var x2_: vec3<f32>;
    var x3_: vec3<f32>;
    var p: vec4<f32>;
    var n: f32 = 0.14285715f;
    var ns: vec3<f32>;
    var j: vec4<f32>;
    var x_6: vec4<f32>;
    var y: vec4<f32>;
    var x_7: vec4<f32>;
    var y_1: vec4<f32>;
    var h: vec4<f32>;
    var b0_: vec4<f32>;
    var b1_: vec4<f32>;
    var s0_: vec4<f32>;
    var s1_: vec4<f32>;
    var sh: vec4<f32>;
    var a0_: vec4<f32>;
    var a1_: vec4<f32>;
    var p0_: vec3<f32>;
    var p1_: vec3<f32>;
    var p2_: vec3<f32>;
    var p3_: vec3<f32>;
    var norm: vec4<f32>;
    var m: vec4<f32>;

    v_1 = v;
    let _e47 = v_1;
    let _e48 = v_1;
    let _e49 = C;
    i = floor((_e47 + vec3(dot(_e48, _e49.yyy))));
    let _e56 = v_1;
    let _e57 = i;
    let _e59 = i;
    let _e60 = C;
    x0_ = ((_e56 - _e57) + vec3(dot(_e59, _e60.xxx)));
    let _e66 = x0_;
    let _e68 = x0_;
    g = step(_e66.yzx, _e68.xyz);
    let _e73 = g;
    l = (vec3(1f) - _e73);
    let _e77 = g;
    let _e79 = l;
    i1_ = min(_e77.xyz, _e79.zxy);
    let _e83 = g;
    let _e85 = l;
    i2_ = max(_e83.xyz, _e85.zxy);
    let _e89 = x0_;
    let _e90 = i1_;
    let _e92 = C;
    x1_ = ((_e89 - _e90) + _e92.xxx);
    let _e96 = x0_;
    let _e97 = i2_;
    let _e99 = C;
    x2_ = ((_e96 - _e97) + _e99.yyy);
    let _e103 = x0_;
    let _e104 = D;
    x3_ = (_e103 - _e104.yyy);
    let _e108 = i;
    let _e109 = mod289_(_e108);
    i = _e109;
    let _e110 = i;
    let _e113 = i1_;
    let _e115 = i2_;
    let _e121 = permute((vec4(_e110.z) + vec4<f32>(0f, _e113.z, _e115.z, 1f)));
    let _e122 = i;
    let _e127 = i1_;
    let _e129 = i2_;
    let _e134 = permute(((_e121 + vec4(_e122.y)) + vec4<f32>(0f, _e127.y, _e129.y, 1f)));
    let _e135 = i;
    let _e140 = i1_;
    let _e142 = i2_;
    let _e147 = permute(((_e134 + vec4(_e135.x)) + vec4<f32>(0f, _e140.x, _e142.x, 1f)));
    p = _e147;
    let _e151 = n;
    let _e152 = D;
    let _e155 = D;
    ns = ((_e151 * _e152.wyz) - _e155.xzx);
    let _e159 = p;
    let _e161 = p;
    let _e162 = ns;
    let _e165 = ns;
    j = (_e159 - (49f * floor(((_e161 * _e162.z) * _e165.z))));
    let _e172 = j;
    let _e173 = ns;
    x_6 = floor((_e172 * _e173.z));
    let _e178 = j;
    let _e180 = x_6;
    y = floor((_e178 - (7f * _e180)));
    let _e185 = x_6;
    let _e186 = ns;
    let _e189 = ns;
    x_7 = ((_e185 * _e186.x) + _e189.yyyy);
    let _e193 = y;
    let _e194 = ns;
    let _e197 = ns;
    y_1 = ((_e193 * _e194.x) + _e197.yyyy);
    let _e202 = x_7;
    let _e206 = y_1;
    h = ((vec4(1f) - abs(_e202)) - abs(_e206));
    let _e210 = x_7;
    let _e211 = _e210.xy;
    let _e212 = y_1;
    let _e213 = _e212.xy;
    b0_ = vec4<f32>(_e211.x, _e211.y, _e213.x, _e213.y);
    let _e220 = x_7;
    let _e221 = _e220.zw;
    let _e222 = y_1;
    let _e223 = _e222.zw;
    b1_ = vec4<f32>(_e221.x, _e221.y, _e223.x, _e223.y);
    let _e230 = b0_;
    s0_ = ((floor(_e230) * 2f) + vec4(1f));
    let _e238 = b1_;
    s1_ = ((floor(_e238) * 2f) + vec4(1f));
    let _e246 = h;
    sh = -(step(_e246, vec4(0f)));
    let _e252 = b0_;
    let _e254 = s0_;
    let _e256 = sh;
    a0_ = (_e252.xzyw + (_e254.xzyw * _e256.xxyy));
    let _e261 = b1_;
    let _e263 = s1_;
    let _e265 = sh;
    a1_ = (_e261.xzyw + (_e263.xzyw * _e265.zzww));
    let _e270 = a0_;
    let _e271 = _e270.xy;
    let _e272 = h;
    p0_ = vec3<f32>(_e271.x, _e271.y, _e272.x);
    let _e278 = a0_;
    let _e279 = _e278.zw;
    let _e280 = h;
    p1_ = vec3<f32>(_e279.x, _e279.y, _e280.y);
    let _e286 = a1_;
    let _e287 = _e286.xy;
    let _e288 = h;
    p2_ = vec3<f32>(_e287.x, _e287.y, _e288.z);
    let _e294 = a1_;
    let _e295 = _e294.zw;
    let _e296 = h;
    p3_ = vec3<f32>(_e295.x, _e295.y, _e296.w);
    let _e302 = p0_;
    let _e303 = p0_;
    let _e305 = p1_;
    let _e306 = p1_;
    let _e308 = p2_;
    let _e309 = p2_;
    let _e311 = p3_;
    let _e312 = p3_;
    let _e315 = taylorInvSqrt(vec4<f32>(dot(_e302, _e303), dot(_e305, _e306), dot(_e308, _e309), dot(_e311, _e312)));
    norm = _e315;
    let _e317 = p0_;
    let _e318 = norm;
    p0_ = (_e317 * _e318.x);
    let _e321 = p1_;
    let _e322 = norm;
    p1_ = (_e321 * _e322.y);
    let _e325 = p2_;
    let _e326 = norm;
    p2_ = (_e325 * _e326.z);
    let _e329 = p3_;
    let _e330 = norm;
    p3_ = (_e329 * _e330.w);
    let _e334 = x0_;
    let _e335 = x0_;
    let _e337 = x1_;
    let _e338 = x1_;
    let _e340 = x2_;
    let _e341 = x2_;
    let _e343 = x3_;
    let _e344 = x3_;
    m = max((vec4(0.6f) - vec4<f32>(dot(_e334, _e335), dot(_e337, _e338), dot(_e340, _e341), dot(_e343, _e344))), vec4(0f));
    let _e353 = m;
    let _e354 = m;
    m = (_e353 * _e354);
    let _e357 = m;
    let _e358 = m;
    let _e360 = p0_;
    let _e361 = x0_;
    let _e363 = p1_;
    let _e364 = x1_;
    let _e366 = p2_;
    let _e367 = x2_;
    let _e369 = p3_;
    let _e370 = x3_;
    return (42f * dot((_e357 * _e358), vec4<f32>(dot(_e360, _e361), dot(_e363, _e364), dot(_e366, _e367), dot(_e369, _e370))));
}

fn random1d(n_1: f32) -> f32 {
    var n_2: f32;

    n_2 = n_1;
    let _e33 = n_2;
    return fract((sin(_e33) * 10000f));
}

fn noise1d(p_1: f32) -> f32 {
    var p_2: f32;
    var i_1: f32;
    var f: f32;
    var u: f32;

    p_2 = p_1;
    let _e33 = p_2;
    i_1 = floor(_e33);
    let _e36 = p_2;
    f = fract(_e36);
    let _e39 = f;
    let _e40 = f;
    let _e44 = f;
    u = ((_e39 * _e40) * (3f - (2f * _e44)));
    let _e49 = i_1;
    let _e50 = random1d(_e49);
    let _e51 = i_1;
    let _e54 = random1d((_e51 + 1f));
    let _e55 = u;
    return mix(_e50, _e54, _e55);
}

fn luma(color: vec3<f32>) -> f32 {
    var color_1: vec3<f32>;

    color_1 = color;
    let _e33 = color_1;
    return dot(_e33, vec3<f32>(0.299f, 0.587f, 0.114f));
}

fn loopNoise3d(val: vec2<f32>, time: f32, loopDuration: f32, speed: f32) -> f32 {
    var val_1: vec2<f32>;
    var time_1: f32;
    var loopDuration_1: f32;
    var speed_1: f32;
    var loopTime: f32;
    var v1_: f32;
    var v2_: f32;

    val_1 = val;
    time_1 = time;
    loopDuration_1 = loopDuration;
    speed_1 = speed;
    let _e39 = time_1;
    let _e40 = loopDuration_1;
    loopTime = (_e39 - (floor((_e39 / _e40)) * _e40));
    let _e46 = val_1;
    let _e47 = loopTime;
    let _e48 = speed_1;
    let _e53 = noise3d(vec3<f32>(_e46.x, _e46.y, (_e47 * _e48)));
    v1_ = _e53;
    let _e55 = val_1;
    let _e56 = loopTime;
    let _e57 = speed_1;
    let _e59 = loopDuration_1;
    let _e60 = speed_1;
    let _e66 = noise3d(vec3<f32>(_e55.x, _e55.y, ((_e56 * _e57) - (_e59 * _e60))));
    v2_ = _e66;
    let _e68 = v1_;
    let _e69 = v2_;
    let _e70 = loopTime;
    let _e71 = loopDuration_1;
    return mix(_e68, _e69, (_e70 / _e71));
}

// Height of the mesh at a texture coordinate (luma, warp noise and bands).
fn displace_at(uv: vec2<f32>) -> f32 {
    let MAX_DISPLACE = 400f;
    let warpSpeed = 1.5f;
    var disp = 0f;
    let col = textureSampleLevel(tInput_t, tInput_s, uv, 0f).xyz;
    let b = luma(col);
    disp = disp + ((b * global.lumaHeight) * MAX_DISPLACE);
    if (global.b_lumaInvert > 0.5f) {
        disp = -(disp);
    }
    let nDisp = loopNoise3d((uv * global.warpScale), (global.time + global.seed), 1f, warpSpeed);
    disp = disp + (((nDisp * global.warpHeight) * MAX_DISPLACE) * 0.5f);
    let yt = (((uv.y * 10f) + (global.time * 10f)) + (global.seed * 10f));
    var offset = noise1d(yt);
    offset = pow(offset, 4f);
    offset = step(0.5f, offset);
    disp = disp + (((offset * MAX_DISPLACE) * global.bandsHeight) * 0.25f);
    return disp;
}

fn main_1() {
    var dispPosition: vec3<f32>;
    var mvPosition: vec4<f32>;

    vUv = uv_1;
    let disp = displace_at(uv_1);
    dispPosition = position_1;
    dispPosition.z = disp;
    mvPosition = (global.modelViewMatrix * vec4<f32>(dispPosition.x, dispPosition.y, dispPosition.z, 1f));
    gl_Position = (global.projectionMatrix * mvPosition);
    return;
}

@vertex 
fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    position_1 = position;
    uv_1 = uv;
    main_1();
    let _e41 = vUv;
    let _e43 = gl_Position;
    return VertexOutput(_e41, _e43);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    var col = textureSample(tInput_t, tInput_s, uv);
    if (global.b_wireframe > 0.5) {
        let l = luma(col.xyz);
        col = vec4<f32>(global.gridColor * l, l);
    }
    return vec4<f32>(col.xyz, col.w * global.opacity);
}

// ---- lines and points: generated from the grid position, no vertex buffer

const PLANE_SIZE = 1024f;

// Clip position of the displaced grid point at texture coordinate uv.
fn project(uv: vec2<f32>) -> vec4<f32> {
    let p = vec4<f32>((uv.x - 0.5) * PLANE_SIZE, (uv.y - 0.5) * PLANE_SIZE, displace_at(uv), 1f);
    return global.projectionMatrix * (global.modelViewMatrix * p);
}

// Pixel offset (from the centre) converted to a clip space offset.
fn to_clip(c: vec4<f32>, px: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(c.xy + px / (global.viewport * 0.5) * c.w, c.z, c.w);
}

fn output_color(c: vec3<f32>, a: f32) -> vec4<f32> {
    if (global.b_premul > 0.5) {
        return vec4<f32>(c * a, a);
    }
    return vec4<f32>(c, a);
}

struct LineOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) col: vec3<f32>,
    @location(1) across: f32,
}

// One instance per line, a triangle strip of (segs + 1) points with two vertices each.
@vertex
fn vs_line(@builtin(vertex_index) vi: u32, @builtin(instance_index) row: u32) -> LineOut {
    let n = global.segs;
    let u = f32(vi / 2u) / n;
    let v = f32(row) / max(global.lineCount - 1f, 1f);
    let side = select(-1f, 1f, (vi & 1u) == 1u);
    let du = 1f / n;
    let c = project(vec2<f32>(u, v));
    let a = project(vec2<f32>(max(u - du, 0f), v));
    let b = project(vec2<f32>(min(u + du, 1f), v));
    let hv = global.viewport * 0.5;
    var t = b.xy / b.w * hv - a.xy / a.w * hv;
    if (dot(t, t) < 1e-8) {
        t = vec2<f32>(1f, 0f);
    }
    t = normalize(t);
    let half_w = (global.lineWidth + global.lineSoft) * 0.5;
    var o: LineOut;
    o.pos = to_clip(c, vec2<f32>(-t.y, t.x) * (side * half_w));
    o.col = textureSampleLevel(tInput_t, tInput_s, vec2<f32>(u, v), 0f).xyz;
    o.across = side * half_w;
    return o;
}

@fragment
fn fs_line(in: LineOut) -> @location(0) vec4<f32> {
    let hw = global.lineWidth * 0.5;
    let s = global.lineSoft * 0.5;
    let a = 1f - smoothstep(hw - s, hw + s, abs(in.across));
    return output_color(in.col, a * global.opacity);
}

struct PointOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) col: vec3<f32>,
    @location(1) corner: vec2<f32>,
}

// One instance per grid point, a four vertex strip around it.
@vertex
fn vs_point(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> PointOut {
    let per_row = u32(global.segs) + 1u;
    let uv = vec2<f32>(f32(ii % per_row), f32(ii / per_row)) / global.segs;
    let corner = vec2<f32>(select(-1f, 1f, (vi & 1u) == 1u), select(-1f, 1f, (vi & 2u) == 2u));
    let half_px = global.pointSize * 0.5 + 0.5;
    var o: PointOut;
    o.pos = to_clip(project(uv), corner * half_px);
    o.col = textureSampleLevel(tInput_t, tInput_s, uv, 0f).xyz;
    o.corner = corner * half_px;
    return o;
}

@fragment
fn fs_point(in: PointOut) -> @location(0) vec4<f32> {
    let r = global.pointSize * 0.5;
    let a = 1f - smoothstep(r - 0.5, r + 0.5, length(in.corner));
    return output_color(in.col, a);
}
