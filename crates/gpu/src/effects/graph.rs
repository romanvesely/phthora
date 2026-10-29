//! Effects made of several passes, history buffers or geometry.

use super::{base_uniform, shader_default, GpuEffect, RenderCtx};
use crate::context::{GpuContext, SamplerKind, Target, FORMAT};
use crate::shader::{pack_uniforms, reflect, run_pass, Blend, CompiledShader, ShaderLibrary, TexInput, UVal, UniformBuf, UniformLayout};
use glam::{Mat4, Vec3};
use phthora_core::{EffectDesc, OutputMode, ResolvedEffect};
use std::rc::Rc;
use wgpu::util::DeviceExt;

pub fn create_custom(ctx: &GpuContext, lib: &ShaderLibrary, desc: &EffectDesc) -> Option<Box<dyn GpuEffect>> {
    Some(match desc.id.as_str() {
        "feedback" => Box::new(History::new(ctx, lib, "feedback", "tPrev", HistoryKind::Feedback)),
        "pixel_sort" => Box::new(History::new(ctx, lib, "pixel_sort", "tPrev", HistoryKind::PixelSort { frame: 0, reset: 0 })),
        "watercolor" => Box::new(History::new(ctx, lib, "watercolor", "tPrevious", HistoryKind::Plain)),
        "data_mosh" => Box::new(DataMosh::new(ctx, lib)),
        "optical_flow" => Box::new(OpticalFlow::new(ctx, lib)),
        "decimate" => Box::new(Decimate::new(ctx, lib)),
        "glow" => Box::new(Glow::new(ctx, lib)),
        "blur" => Box::new(Blur::new(ctx, lib)),
        "transform_3d" => Box::new(Plane3d::new(ctx)),
        "luma_mesh" => Box::new(LumaMesh::new(ctx)),
        _ => return None,
    })
}

/// One use of a fullscreen shader with its own uniform buffer
/// (uniform writes land before the frame's commands, so a buffer cannot be shared between uses).
struct Node {
    shader: Rc<CompiledShader>,
    ubuf: UniformBuf,
}

impl Node {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary, name: &str, blend: Blend) -> Self {
        let shader = lib.get_with(name, blend, FORMAT);
        let size = shader.uniform.as_ref().map(|u| u.size).unwrap_or(16);
        Self { ubuf: UniformBuf::new(ctx, size), shader }
    }

    fn pack(&self, lookup: &dyn Fn(&str) -> Option<UVal>) -> Vec<u8> {
        match &self.shader.uniform {
            Some(l) => pack_uniforms(l, &|k| lookup(k).or_else(|| shader_default(&self.shader.name, k))).0,
            None => Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn run<'a>(
        &self,
        ctx: &GpuContext,
        enc: &mut wgpu::CommandEncoder,
        data: &[u8],
        tex: &dyn Fn(&str) -> Option<TexInput<'a>>,
        fallback: &wgpu::TextureView,
        target: &Target,
        clear: Option<[f64; 4]>,
    ) {
        if !data.is_empty() {
            self.ubuf.write(ctx, data);
        }
        run_pass(ctx, enc, &self.shader, Some(&self.ubuf.buf), tex, fallback, &target.view, clear);
    }
}

fn texs<'a>(list: &'a [(&'a str, &'a Target)]) -> impl Fn(&str) -> Option<TexInput<'a>> + 'a {
    move |n: &str| list.iter().find(|(k, _)| *k == n).map(|(_, t)| TexInput { view: &t.view, sampler: SamplerKind::LinearClamp })
}

/// (Re)create a target when the size changed. Returns true if it was (re)created.
fn ensure(t: &mut Option<Target>, ctx: &GpuContext, w: u32, h: u32, label: &str) -> bool {
    if t.as_ref().map(|t| t.size()) != Some((w, h)) {
        *t = Some(Target::new(ctx, w, h, label));
        true
    } else {
        false
    }
}

const TRANSPARENT: [f64; 4] = [0.0; 4];

// ---------------------------------------------------------------- history (feedback, pixel sort, watercolor)

enum HistoryKind {
    Feedback,
    PixelSort { frame: u64, reset: u64 },
    Plain,
}

/// Shader that reads its own previous output.
struct History {
    node: Node,
    copy: Node,
    prev_name: &'static str,
    prev: Option<Target>,
    kind: HistoryKind,
}

impl History {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary, shader: &str, prev_name: &'static str, kind: HistoryKind) -> Self {
        Self {
            node: Node::new(ctx, lib, shader, Blend::Premultiplied),
            copy: Node::new(ctx, lib, "copy", Blend::Premultiplied),
            prev_name,
            prev: None,
            kind,
        }
    }
}

impl GpuEffect for History {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        if ensure(&mut self.prev, rc.ctx, rc.width, rc.height, "history")
            && let HistoryKind::PixelSort { frame, .. } = &mut self.kind {
                *frame = 0;
            }
        let frame_val = match &mut self.kind {
            HistoryKind::PixelSort { frame, reset } => {
                let r = e.triggers.get("reset").copied().unwrap_or(0);
                if r != *reset {
                    *reset = r;
                    *frame = 0;
                }
                let v = *frame;
                *frame += 1;
                v as f32
            }
            _ => 0.0,
        };
        let feedback = matches!(self.kind, HistoryKind::Feedback);
        let d = self.node.pack(&|k| {
            match k {
                "seed" if feedback => return Some(UVal::F((e.seed * 100.0) as f32)),
                "frame" => return Some(UVal::F(frame_val)),
                // declared by the shader but never set by the reference
                "blend" | "fade" | "scaleUp" => return Some(UVal::F(0.0)),
                _ => {}
            }
            base_uniform(e, rc, k)
        });
        let prev = self.prev.as_ref().unwrap();
        let list = [("tDiffuse", input), (self.prev_name, prev)];
        self.node.run(rc.ctx, rc.enc, &d, &texs(&list), &rc.blank.view, out, Some(rc.clear));
        let list = [("tDiffuse", out)];
        self.copy.run(rc.ctx, rc.enc, &[], &texs(&list), &rc.blank.view, prev, Some(rc.clear));
    }

    fn reset(&mut self, ctx: &GpuContext) {
        if let Some(p) = &self.prev {
            p.clear(ctx, TRANSPARENT);
        }
        if let HistoryKind::PixelSort { frame, .. } = &mut self.kind {
            *frame = 0;
        }
    }
}

// ---------------------------------------------------------------- Data Mosh

struct DataMosh {
    mask: Node,
    acc: Node,
    disp: Node,
    copy: [Node; 3],
    t_mask: Option<Target>,
    t_acc: Option<Target>,
    hist: [Option<Target>; 3],
}

impl DataMosh {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary) -> Self {
        let c = || Node::new(ctx, lib, "copy", Blend::Premultiplied);
        Self {
            mask: Node::new(ctx, lib, "data_mosh_mask", Blend::Premultiplied),
            acc: Node::new(ctx, lib, "accumulator", Blend::Premultiplied),
            disp: Node::new(ctx, lib, "data_mosh_displace", Blend::Premultiplied),
            copy: [c(), c(), c()],
            t_mask: None,
            t_acc: None,
            hist: [None, None, None],
        }
    }
}

impl GpuEffect for DataMosh {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        let (w, h) = (rc.width, rc.height);
        ensure(&mut self.t_mask, rc.ctx, w, h, "dm-mask");
        ensure(&mut self.t_acc, rc.ctx, w, h, "dm-acc");
        for t in &mut self.hist {
            ensure(t, rc.ctx, w, h, "dm-hist");
        }
        let (tm, ta) = (self.t_mask.as_ref().unwrap(), self.t_acc.as_ref().unwrap());
        let [h1, h2, h3] = [self.hist[0].as_ref().unwrap(), self.hist[1].as_ref().unwrap(), self.hist[2].as_ref().unwrap()];
        let (ctx, fb, clear) = (rc.ctx, &rc.blank.view, rc.clear);
        // the helper passes only see their own defaults; the effect params drive the displace pass
        let none = |_: &str| None;
        let dm = self.mask.pack(&none);
        let da = self.acc.pack(&none);
        let dd = self.disp.pack(&|k| base_uniform(e, rc, k));
        self.mask.run(ctx, rc.enc, &dm, &texs(&[("tDiffuse", input), ("tPrevious", h1)]), fb, tm, Some(clear));
        self.copy[0].run(ctx, rc.enc, &[], &texs(&[("tDiffuse", input)]), fb, h1, Some(clear));
        self.acc.run(ctx, rc.enc, &da, &texs(&[("tCurrent", tm), ("tPrev", h2)]), fb, ta, Some(clear));
        self.copy[1].run(ctx, rc.enc, &[], &texs(&[("tDiffuse", ta)]), fb, h2, Some(clear));
        self.disp.run(ctx, rc.enc, &dd, &texs(&[("tDiffuse", input), ("tPrev", h3), ("tOFMap", ta)]), fb, out, Some(clear));
        self.copy[2].run(ctx, rc.enc, &[], &texs(&[("tDiffuse", out)]), fb, h3, Some(clear));
    }

    fn reset(&mut self, ctx: &GpuContext) {
        for t in self.hist.iter().flatten() {
            t.clear(ctx, TRANSPARENT);
        }
    }
}

// ---------------------------------------------------------------- Optical Flow

struct OpticalFlow {
    map: Node,
    disp: Node,
    copy_map: Node,
    copy_in: Node,
    t_map: Option<Target>,
    map_delay: Option<Target>,
    delay: Option<Target>,
}

impl OpticalFlow {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary) -> Self {
        Self {
            map: Node::new(ctx, lib, "ofmap", Blend::Premultiplied),
            disp: Node::new(ctx, lib, "ofdisplace", Blend::Premultiplied),
            copy_map: Node::new(ctx, lib, "copy", Blend::Premultiplied),
            copy_in: Node::new(ctx, lib, "copy", Blend::Premultiplied),
            t_map: None,
            map_delay: None,
            delay: None,
        }
    }
}

impl GpuEffect for OpticalFlow {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        let (w, h) = (rc.width, rc.height);
        ensure(&mut self.t_map, rc.ctx, w, h, "of-map");
        ensure(&mut self.map_delay, rc.ctx, w, h, "of-map-delay");
        ensure(&mut self.delay, rc.ctx, w, h, "of-delay");
        let (tm, md, dl) = (self.t_map.as_ref().unwrap(), self.map_delay.as_ref().unwrap(), self.delay.as_ref().unwrap());
        let (ctx, fb, clear) = (rc.ctx, &rc.blank.view, rc.clear);
        // `distortion` and `speed` belong to the map pass, the rest to the displace pass
        let dmap = self.map.pack(&|k| match k {
            "distortion" | "speed" | "resolution" => base_uniform(e, rc, k),
            _ => None,
        });
        let ddisp = self.disp.pack(&|k| base_uniform(e, rc, k));
        // the analysis passes clear to transparent regardless of the background
        self.map.run(ctx, rc.enc, &dmap, &texs(&[("tDiffuse", input), ("tPrevious", dl), ("tMapPrevious", md)]), fb, tm, Some(TRANSPARENT));
        self.copy_map.run(ctx, rc.enc, &[], &texs(&[("tDiffuse", tm)]), fb, md, Some(TRANSPARENT));
        self.copy_in.run(ctx, rc.enc, &[], &texs(&[("tDiffuse", input)]), fb, dl, Some(TRANSPARENT));
        self.disp.run(ctx, rc.enc, &ddisp, &texs(&[("tDiffuse", input), ("tMap", tm)]), fb, out, Some(clear));
    }

    fn reset(&mut self, ctx: &GpuContext) {
        for t in [&self.map_delay, &self.delay].into_iter().flatten() {
            t.clear(ctx, TRANSPARENT);
        }
    }
}

// ---------------------------------------------------------------- Decimate

struct Decimate {
    nodes: [Node; 3],
    tmp: [Option<Target>; 2],
}

impl Decimate {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary) -> Self {
        let n = |s: &str| Node::new(ctx, lib, s, Blend::Premultiplied);
        Self { nodes: [n("decimate1"), n("decimate2"), n("decimate3")], tmp: [None, None] }
    }
}

impl GpuEffect for Decimate {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        for t in &mut self.tmp {
            ensure(t, rc.ctx, rc.width, rc.height, "decimate");
        }
        let d: Vec<Vec<u8>> = self.nodes.iter().map(|n| n.pack(&|k| base_uniform(e, rc, k))).collect();
        let (t1, t2) = (self.tmp[0].as_ref().unwrap(), self.tmp[1].as_ref().unwrap());
        let (ctx, fb, clear) = (rc.ctx, &rc.blank.view, rc.clear);
        self.nodes[0].run(ctx, rc.enc, &d[0], &texs(&[("tDiffuse", input)]), fb, t1, Some(clear));
        self.nodes[1].run(ctx, rc.enc, &d[1], &texs(&[("tDiffuse", t1)]), fb, t2, Some(clear));
        self.nodes[2].run(ctx, rc.enc, &d[2], &texs(&[("tDiffuse", t2)]), fb, out, Some(clear));
    }
}

// ---------------------------------------------------------------- Glow / Blur (downsampled blur pyramid)

const LEVELS: usize = 4;

/// Level sizes: full, then halved (rounded) per level.
fn pyramid(w: u32, h: u32) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    let (mut tw, mut th) = (w as f64 / 2.0, h as f64 / 2.0);
    for _ in 0..LEVELS {
        let s = ((tw.round() as u32).max(1), (th.round() as u32).max(1));
        v.push(s);
        tw = s.0 as f64 / 2.0;
        th = s.1 as f64 / 2.0;
    }
    v
}

struct Pyramid {
    full: Option<Target>,
    /// (horizontal, vertical) target per level
    levels: Vec<(Target, Target)>,
    size: (u32, u32),
    blur: Vec<(Node, Node)>,
}

impl Pyramid {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary, shader: &str) -> Self {
        let blur = (0..LEVELS)
            .map(|_| (Node::new(ctx, lib, shader, Blend::Replace), Node::new(ctx, lib, shader, Blend::Replace)))
            .collect();
        Self { full: None, levels: Vec::new(), size: (0, 0), blur }
    }

    fn ensure(&mut self, ctx: &GpuContext, w: u32, h: u32) {
        if self.size == (w, h) {
            return;
        }
        self.size = (w, h);
        self.full = Some(Target::new(ctx, w, h, "pyr-0"));
        self.levels = pyramid(w, h)
            .into_iter()
            .map(|(lw, lh)| (Target::new(ctx, lw, lh, "pyr-h"), Target::new(ctx, lw, lh, "pyr-v")))
            .collect();
    }

    /// Blur `full` down the pyramid with step `v` (texels).
    fn run(&self, rc: &mut RenderCtx, v: f32) {
        let mut src = self.full.as_ref().unwrap();
        for (i, (th, tv)) in self.levels.iter().enumerate() {
            let res = UVal::V2([th.width as f32, th.height as f32]);
            let (nh, nv) = &self.blur[i];
            let dh = nh.pack(&|k| match k {
                "delta" => Some(UVal::V2([v, 0.0])),
                "resolution" => Some(res),
                _ => None,
            });
            let dv = nv.pack(&|k| match k {
                "delta" => Some(UVal::V2([0.0, v])),
                "resolution" => Some(res),
                _ => None,
            });
            nh.run(rc.ctx, rc.enc, &dh, &texs(&[("source", src)]), &rc.blank.view, th, Some(rc.clear));
            nv.run(rc.ctx, rc.enc, &dv, &texs(&[("source", th)]), &rc.blank.view, tv, Some(rc.clear));
            src = tv;
        }
    }
}

struct Glow {
    highlight: Node,
    bloom: Node,
    pyr: Pyramid,
}

impl Glow {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary) -> Self {
        Self {
            highlight: Node::new(ctx, lib, "highlight", Blend::Replace),
            bloom: Node::new(ctx, lib, "bloom", Blend::Replace),
            pyr: Pyramid::new(ctx, lib, "bloom_blur"),
        }
    }
}

impl GpuEffect for Glow {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        self.pyr.ensure(rc.ctx, rc.width, rc.height);
        let dh = self.highlight.pack(&|k| base_uniform(e, rc, k));
        let db = self.bloom.pack(&|k| base_uniform(e, rc, k));
        let full = self.pyr.full.as_ref().unwrap();
        self.highlight.run(rc.ctx, rc.enc, &dh, &texs(&[("tDiffuse", input)]), &rc.blank.view, full, Some(rc.clear));
        self.pyr.run(rc, 1.0);
        let l = &self.pyr.levels;
        let list = [("base", input), ("level0", &l[0].1), ("level1", &l[1].1), ("level2", &l[2].1), ("level3", &l[3].1)];
        self.bloom.run(rc.ctx, rc.enc, &db, &texs(&list), &rc.blank.view, out, Some(rc.clear));
    }
}

struct Blur {
    copy: Node,
    mix: Node,
    pyr: Pyramid,
}

impl Blur {
    fn new(ctx: &GpuContext, lib: &ShaderLibrary) -> Self {
        Self {
            copy: Node::new(ctx, lib, "copy", Blend::Replace),
            mix: Node::new(ctx, lib, "mix", Blend::Replace),
            pyr: Pyramid::new(ctx, lib, "blur_blur"),
        }
    }
}

impl GpuEffect for Blur {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        self.pyr.ensure(rc.ctx, rc.width, rc.height);
        let full = self.pyr.full.as_ref().unwrap();
        self.copy.run(rc.ctx, rc.enc, &[], &texs(&[("tDiffuse", input)]), &rc.blank.view, full, Some(rc.clear));
        // resolution independent blur distance
        let v = (e.f("delta") * 1.4 + 0.3) * rc.width as f32 / 1024.0;
        self.pyr.run(rc, v);
        let dm = self.mix.pack(&|k| base_uniform(e, rc, k));
        let last = &self.pyr.levels[LEVELS - 1].1;
        self.mix.run(rc.ctx, rc.enc, &dm, &texs(&[("tDiffuse", input), ("tBlend", last)]), &rc.blank.view, out, Some(rc.clear));
    }
}

// ---------------------------------------------------------------- geometry

const PLANE_SIZE: f32 = 1024.0;
const CAMERA_Z: f32 = 886.0;

/// Perspective projection of the reference camera, converted to wgpu clip space
/// and flipped vertically to match the bottom-up texture convention.
fn projection(aspect: f32) -> Mat4 {
    // OpenGL-style perspective (fov 60, near 1, far 5000)
    let (near, far) = (1.0f32, 5000.0f32);
    let f = 1.0 / (30f32.to_radians()).tan();
    let gl = Mat4::from_cols_array(&[
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, -(far + near) / (far - near), -1.0,
        0.0, 0.0, -2.0 * far * near / (far - near), 0.0,
    ]);
    let fix = Mat4::from_cols_array(&[1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0]);
    fix * gl
}

fn view() -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 0.0, -CAMERA_Z))
}

fn euler_xyz(x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_rotation_x(x) * Mat4::from_rotation_y(y) * Mat4::from_rotation_z(z)
}

/// Plane geometry like the reference: vertices (x, y, z, u, v) and triangle indices.
fn plane(size: f32, segs: u32) -> (Vec<[f32; 5]>, Vec<u32>) {
    let n = segs + 1;
    let step = size / segs as f32;
    let half = size / 2.0;
    let mut v = Vec::with_capacity((n * n) as usize);
    for iy in 0..n {
        let y = iy as f32 * step - half;
        for ix in 0..n {
            let x = ix as f32 * step - half;
            v.push([x, -y, 0.0, ix as f32 / segs as f32, 1.0 - iy as f32 / segs as f32]);
        }
    }
    let mut idx = Vec::with_capacity((segs * segs * 6) as usize);
    for iy in 0..segs {
        for ix in 0..segs {
            let a = ix + n * iy;
            let b = ix + n * (iy + 1);
            let c = ix + 1 + n * (iy + 1);
            let d = ix + 1 + n * iy;
            idx.extend_from_slice(&[a, b, d, b, c, d]);
        }
    }
    (v, idx)
}

/// Wireframe lines from triangles: every triangle edge, shared edges included twice.
fn wire_indices(tris: &[u32]) -> Vec<u32> {
    tris.as_chunks::<3>().0.iter().flat_map(|t| [t[0], t[1], t[1], t[2], t[2], t[0]]).collect()
}

struct GeoPipeline {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    layout: UniformLayout,
    tex_binding: u32,
    samp_binding: u32,
}

fn geo_pipeline(ctx: &GpuContext, src: &str, label: &str, topology: wgpu::PrimitiveTopology, blend: wgpu::BlendState) -> GeoPipeline {
    let (uniform, textures) = reflect(src).expect("geometry shader");
    let layout = uniform.expect("uniform block");
    let t = &textures[0];
    let dev = &ctx.device;
    let vis = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
    let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: layout.binding,
                visibility: vis,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: t.tex,
                visibility: vis,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: t.samp.unwrap(),
                visibility: vis,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some(label), bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
    let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(label), source: wgpu::ShaderSource::Wgsl(src.into()) });
    let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
    let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attrs })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { topology, cull_mode: None, ..Default::default() },
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    GeoPipeline { pipeline, bgl, layout, tex_binding: t.tex, samp_binding: t.samp.unwrap() }
}

/// Non-premultiplied "normal" blending of the reference mesh materials.
const NORMAL_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

const ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
    alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
};

fn write_mat(buf: &mut [u8], layout: &UniformLayout, key: &str, m: &Mat4) {
    if let Some(f) = layout.fields.iter().find(|f| f.key == key) {
        let o = f.offset as usize;
        buf[o..o + 64].copy_from_slice(bytemuck::cast_slice(&m.to_cols_array()));
    }
}

fn geo_bind_group(ctx: &GpuContext, p: &GeoPipeline, ubuf: &wgpu::Buffer, view: &wgpu::TextureView) -> wgpu::BindGroup {
    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("geo"),
        layout: &p.bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: p.layout.binding, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: p.tex_binding, resource: wgpu::BindingResource::TextureView(view) },
            wgpu::BindGroupEntry { binding: p.samp_binding, resource: wgpu::BindingResource::Sampler(ctx.samplers.get(SamplerKind::LinearClamp)) },
        ],
    })
}

fn begin<'e>(enc: &'e mut wgpu::CommandEncoder, out: &'e Target, clear: [f64; 4]) -> wgpu::RenderPass<'e> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("geometry"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &out.view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0], g: clear[1], b: clear[2], a: clear[3] }),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    })
}

struct Mesh {
    vbuf: wgpu::Buffer,
    tris: wgpu::Buffer,
    tri_count: u32,
    lines: Option<(wgpu::Buffer, u32)>,
}

impl Mesh {
    fn new(ctx: &GpuContext, segs: u32, with_lines: bool) -> Self {
        let (v, idx) = plane(PLANE_SIZE, segs);
        let dev = &ctx.device;
        let vbuf = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("plane"), contents: bytemuck::cast_slice(&v), usage: wgpu::BufferUsages::VERTEX });
        let tris = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("tris"), contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX });
        let lines = with_lines.then(|| {
            let w = wire_indices(&idx);
            let b = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("lines"), contents: bytemuck::cast_slice(&w), usage: wgpu::BufferUsages::INDEX });
            (b, w.len() as u32)
        });
        Self { vbuf, tris, tri_count: idx.len() as u32, lines }
    }
}

/// Transform 3D: the input as a textured plane in front of a perspective camera.
struct Plane3d {
    pipe: GeoPipeline,
    mesh: Mesh,
    ubuf: UniformBuf,
}

impl Plane3d {
    fn new(ctx: &GpuContext) -> Self {
        let pipe = geo_pipeline(ctx, include_str!("../../wgsl/plane3d.wgsl"), "plane3d", wgpu::PrimitiveTopology::TriangleList, NORMAL_BLEND);
        let ubuf = UniformBuf::new(ctx, pipe.layout.size);
        Self { pipe, mesh: Mesh::new(ctx, 1, false), ubuf }
    }
}

impl GpuEffect for Plane3d {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        let aspect = rc.width as f32 / rc.height.max(1) as f32;
        let tau = std::f32::consts::TAU;
        let group = Mat4::from_scale(Vec3::new(aspect, 1.0, 1.0));
        let pos = Vec3::new(e.f("xPos") - 0.5, e.f("yPos") - 0.5, 0.0) * 2048.0;
        let model = Mat4::from_translation(pos)
            * euler_xyz((e.f("xRot") - 0.5) * tau, (e.f("yRot") - 0.5) * tau, (e.f("zRot") - 0.5) * tau)
            * Mat4::from_scale(Vec3::new(e.f("xScale"), e.f("yScale"), 1.0));
        let mvp = projection(aspect) * view() * group * model;
        let mut data = vec![0u8; self.pipe.layout.size as usize];
        write_mat(&mut data, &self.pipe.layout, "mvp", &mvp);
        self.ubuf.write(rc.ctx, &data);
        let bg = geo_bind_group(rc.ctx, &self.pipe, &self.ubuf.buf, &input.view);
        let mut rp = begin(rc.enc, out, rc.clear);
        rp.set_pipeline(&self.pipe.pipeline);
        rp.set_bind_group(0, &bg, &[]);
        rp.set_vertex_buffer(0, self.mesh.vbuf.slice(..));
        rp.set_index_buffer(self.mesh.tris.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.mesh.tri_count, 0, 0..1);
    }
}

/// Luma Mesh: a displaced 120x120 grid, drawn filled and as additive wireframe.
struct LumaMesh {
    mesh_pipe: GeoPipeline,
    grid_pipe: GeoPipeline,
    mesh: Mesh,
    ubuf_mesh: UniformBuf,
    ubuf_grid: UniformBuf,
}

impl LumaMesh {
    fn new(ctx: &GpuContext) -> Self {
        let src = include_str!("../../wgsl/luma_mesh.wgsl");
        let mesh_pipe = geo_pipeline(ctx, src, "luma-mesh", wgpu::PrimitiveTopology::TriangleList, NORMAL_BLEND);
        let grid_pipe = geo_pipeline(ctx, src, "luma-grid", wgpu::PrimitiveTopology::LineList, ADDITIVE_BLEND);
        let size = mesh_pipe.layout.size;
        Self { mesh_pipe, grid_pipe, mesh: Mesh::new(ctx, 120, true), ubuf_mesh: UniformBuf::new(ctx, size), ubuf_grid: UniformBuf::new(ctx, size) }
    }
}

impl GpuEffect for LumaMesh {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, _orig: &Target, out: &Target) {
        let aspect = rc.width as f32 / rc.height.max(1) as f32;
        let rot_rng = std::f64::consts::FRAC_PI_2 * e.f("tiltAmount") as f64;
        let progress = if rc.ft.mode == OutputMode::Video { rc.ft.export_progress } else { e.f("seed") as f64 };
        let scale = e.f("tiltSpeed") as f64 * rc.ft.export_duration / 8.0;
        let seeds = [0u32, 1, 2].map(|k| super::hooks::sub_seed(e.seed, 10 + k) * 100.0);
        let n = |i: usize| phthora_core::noise::loop_noise1d(progress, scale, seeds[i]);
        let (rx, ry, rz) = ((n(0) * rot_rng / 4.0) as f32, (n(1) * rot_rng) as f32, (n(2) * rot_rng / 4.0) as f32);
        let model = euler_xyz(rx, ry, rz) * Mat4::from_scale(Vec3::new(aspect, 1.0, 1.0));
        let mv = view() * model;
        let proj = projection(aspect);
        let layout = &self.mesh_pipe.layout;
        let pack = |opacity: f32, wire: bool| {
            let (mut d, _) = pack_uniforms(layout, &|k| match k {
                "opacity" => Some(UVal::F(opacity)),
                "b_wireframe" => Some(UVal::F(wire as i32 as f32)),
                "time" => Some(UVal::F(e.time)),
                "seed" => Some(UVal::F(e.f("seed"))),
                "bandsScl" => Some(UVal::F(0.4)),
                "warpScale" => Some(UVal::F(2.0)),
                _ => base_uniform(e, rc, k),
            });
            write_mat(&mut d, layout, "modelViewMatrix", &mv);
            write_mat(&mut d, layout, "projectionMatrix", &proj);
            d
        };
        self.ubuf_mesh.write(rc.ctx, &pack(e.f("meshOpacity"), false));
        self.ubuf_grid.write(rc.ctx, &pack(e.f("gridOpacity"), true));
        let bg_mesh = geo_bind_group(rc.ctx, &self.mesh_pipe, &self.ubuf_mesh.buf, &input.view);
        let bg_grid = geo_bind_group(rc.ctx, &self.grid_pipe, &self.ubuf_grid.buf, &input.view);
        let mut rp = begin(rc.enc, out, rc.clear);
        rp.set_vertex_buffer(0, self.mesh.vbuf.slice(..));
        rp.set_pipeline(&self.mesh_pipe.pipeline);
        rp.set_bind_group(0, &bg_mesh, &[]);
        rp.set_index_buffer(self.mesh.tris.slice(..), wgpu::IndexFormat::Uint32);
        rp.draw_indexed(0..self.mesh.tri_count, 0, 0..1);
        if let Some((lines, count)) = &self.mesh.lines {
            rp.set_pipeline(&self.grid_pipe.pipeline);
            rp.set_bind_group(0, &bg_grid, &[]);
            rp.set_index_buffer(lines.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..*count, 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyramid_sizes() {
        assert_eq!(pyramid(1920, 1080), vec![(960, 540), (480, 270), (240, 135), (120, 68)]);
    }

    #[test]
    fn plane_layout() {
        let (v, i) = plane(1024.0, 2);
        assert_eq!(v.len(), 9);
        assert_eq!(i.len(), 24);
        // first vertex: top-left, uv (0, 1)
        assert_eq!(v[0], [-512.0, 512.0, 0.0, 0.0, 1.0]);
        assert_eq!(wire_indices(&i).len(), 48);
    }

    #[test]
    fn camera_fills_view() {
        // a plane of PLANE_SIZE at z=0 spans the full vertical clip range
        let p = projection(1.0) * view();
        let top = p * glam::Vec4::new(0.0, PLANE_SIZE / 2.0, 0.0, 1.0);
        assert!(((top.y / top.w).abs() - 1.0).abs() < 0.01, "{}", top.y / top.w);
    }
}
