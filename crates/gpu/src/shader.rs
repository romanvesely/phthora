//! Shader library: compiles effect fragment shaders, reflects their uniforms and
//! texture bindings, and runs fullscreen passes.

use crate::context::{GpuContext, SamplerKind, Target, FORMAT};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/shaders.rs"));
}

pub const FULLSCREEN_WGSL: &str = include_str!("../wgsl/fullscreen.wgsl");
pub const INPUT_WGSL: &str = include_str!("../wgsl/input.wgsl");

pub fn shader_source(name: &str) -> Option<&'static str> {
    match name {
        "input" => Some(INPUT_WGSL),
        _ => embedded::SHADERS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s),
    }
}

pub fn shader_names() -> impl Iterator<Item = &'static str> {
    embedded::SHADERS.iter().map(|(n, _)| *n)
}

/// A uniform value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UVal {
    F(f32),
    I(i32),
    V2([f32; 2]),
    V3([f32; 3]),
    V4([f32; 4]),
}

impl UVal {
    fn as_f32s(&self) -> [f32; 4] {
        match *self {
            UVal::F(v) => [v, 0., 0., 0.],
            UVal::I(v) => [v as f32, 0., 0., 0.],
            UVal::V2(v) => [v[0], v[1], 0., 0.],
            UVal::V3(v) => [v[0], v[1], v[2], 0.],
            UVal::V4(v) => v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldTy {
    F32,
    I32,
    U32,
    Vec(u8),
    Other,
}

#[derive(Clone, Debug)]
pub struct Field {
    /// member name as written by the shader
    pub name: String,
    /// normalized lookup key (trailing `_` removed)
    pub key: String,
    pub offset: u32,
    pub ty: FieldTy,
}

#[derive(Clone, Debug, Default)]
pub struct UniformLayout {
    pub binding: u32,
    pub size: u32,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub struct TexBinding {
    /// logical name, e.g. `tDiffuse`
    pub name: String,
    pub tex: u32,
    pub samp: Option<u32>,
}

pub struct CompiledShader {
    pub name: String,
    pub pipeline: wgpu::RenderPipeline,
    pub bgl: wgpu::BindGroupLayout,
    pub uniform: Option<UniformLayout>,
    pub textures: Vec<TexBinding>,
}

/// Reflect a fragment shader written in WGSL.
pub fn reflect(src: &str) -> Result<(Option<UniformLayout>, Vec<TexBinding>), String> {
    let module = naga::front::wgsl::parse_str(src).map_err(|e| e.emit_to_string(src))?;
    let mut uniform = None;
    let mut images: Vec<(String, u32)> = Vec::new();
    let mut samplers: Vec<(String, u32)> = Vec::new();
    for (_, gv) in module.global_variables.iter() {
        let Some(rb) = &gv.binding else { continue };
        let name = gv.name.clone().unwrap_or_default();
        match (&gv.space, &module.types[gv.ty].inner) {
            (naga::AddressSpace::Uniform, naga::TypeInner::Struct { members, span }) => {
                let fields = members
                    .iter()
                    .map(|m| {
                        let n = m.name.clone().unwrap_or_default();
                        let ty = match &module.types[m.ty].inner {
                            naga::TypeInner::Scalar(s) => match s.kind {
                                naga::ScalarKind::Float => FieldTy::F32,
                                naga::ScalarKind::Sint => FieldTy::I32,
                                naga::ScalarKind::Uint => FieldTy::U32,
                                _ => FieldTy::Other,
                            },
                            naga::TypeInner::Vector { size, scalar } if scalar.kind == naga::ScalarKind::Float => {
                                FieldTy::Vec(*size as u8)
                            }
                            _ => FieldTy::Other,
                        };
                        Field { key: n.trim_end_matches('_').to_string(), name: n, offset: m.offset, ty }
                    })
                    .collect();
                uniform = Some(UniformLayout { binding: rb.binding, size: (*span).max(16).div_ceil(16) * 16, fields });
            }
            (_, naga::TypeInner::Image { .. }) => images.push((name, rb.binding)),
            (_, naga::TypeInner::Sampler { .. }) => samplers.push((name, rb.binding)),
            _ => {}
        }
    }
    let textures = images
        .into_iter()
        .map(|(n, b)| {
            let base = n.strip_suffix("_t").unwrap_or(&n).to_string();
            let samp = samplers.iter().find(|(s, _)| s.strip_suffix("_s") == Some(base.as_str())).map(|(_, b)| *b);
            TexBinding { name: base, tex: b, samp }
        })
        .collect();
    Ok((uniform, textures))
}

/// Blend mode of a pass's color target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    /// ONE, ONE_MINUS_SRC_ALPHA (reference default for every pass)
    Premultiplied,
    /// write as is
    Replace,
    Additive,
}

pub struct ShaderLibrary {
    ctx: GpuContext,
    vs: wgpu::ShaderModule,
    cache: RefCell<HashMap<(String, Blend, wgpu::TextureFormat), Rc<CompiledShader>>>,
}

impl ShaderLibrary {
    pub fn new(ctx: &GpuContext) -> Self {
        let vs = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fullscreen"),
            source: wgpu::ShaderSource::Wgsl(FULLSCREEN_WGSL.into()),
        });
        Self { ctx: ctx.clone(), vs, cache: RefCell::new(HashMap::new()) }
    }

    pub fn get(&self, name: &str) -> Rc<CompiledShader> {
        self.get_with(name, Blend::Premultiplied, FORMAT)
    }

    pub fn get_with(&self, name: &str, blend: Blend, format: wgpu::TextureFormat) -> Rc<CompiledShader> {
        let key = (name.to_string(), blend, format);
        if let Some(s) = self.cache.borrow().get(&key) {
            return s.clone();
        }
        let src = shader_source(name).unwrap_or_else(|| panic!("unknown shader '{name}'"));
        let s = Rc::new(self.compile(name, src, blend, format).unwrap_or_else(|e| panic!("shader {name}: {e}")));
        self.cache.borrow_mut().insert(key, s.clone());
        s
    }

    pub fn compile(&self, name: &str, src: &str, blend: Blend, format: wgpu::TextureFormat) -> Result<CompiledShader, String> {
        let (uniform, textures) = reflect(src)?;
        let dev = &self.ctx.device;
        let fs = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(name), source: wgpu::ShaderSource::Wgsl(src.into()) });
        let mut entries = Vec::new();
        if let Some(u) = &uniform {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: u.binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            });
        }
        for t in &textures {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: t.tex,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            if let Some(s) = t.samp {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: s,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                });
            }
        }
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(name), entries: &entries });
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(name),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let blend_state = match blend {
            Blend::Premultiplied => Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            Blend::Replace => None,
            Blend::Additive => Some(wgpu::BlendState {
                color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
                alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            }),
        };
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&layout),
            vertex: wgpu::VertexState { module: &self.vs, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &fs,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: blend_state, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(CompiledShader { name: name.to_string(), pipeline, bgl, uniform, textures })
    }
}

/// Pack uniform values into a std140 buffer according to the reflected layout.
/// Returns the packed bytes and the list of fields that got no value.
pub fn pack_uniforms(layout: &UniformLayout, lookup: &dyn Fn(&str) -> Option<UVal>) -> (Vec<u8>, Vec<String>) {
    let mut buf = vec![0u8; layout.size as usize];
    let mut missing = Vec::new();
    for f in &layout.fields {
        let v = lookup(&f.key).or_else(|| if f.key != f.name { lookup(&f.name) } else { None });
        let Some(v) = v else {
            missing.push(f.name.clone());
            continue;
        };
        let o = f.offset as usize;
        match f.ty {
            FieldTy::F32 => buf[o..o + 4].copy_from_slice(&v.as_f32s()[0].to_le_bytes()),
            FieldTy::I32 => {
                let i = match v { UVal::I(i) => i, other => other.as_f32s()[0].round() as i32 };
                buf[o..o + 4].copy_from_slice(&i.to_le_bytes());
            }
            FieldTy::U32 => {
                let i = match v { UVal::I(i) => i as u32, other => other.as_f32s()[0].round().max(0.0) as u32 };
                buf[o..o + 4].copy_from_slice(&i.to_le_bytes());
            }
            FieldTy::Vec(n) => {
                let a = v.as_f32s();
                for k in 0..n as usize {
                    buf[o + k * 4..o + k * 4 + 4].copy_from_slice(&a[k].to_le_bytes());
                }
            }
            FieldTy::Other => missing.push(f.name.clone()),
        }
    }
    (buf, missing)
}

/// Texture input for a pass.
pub struct TexInput<'a> {
    pub view: &'a wgpu::TextureView,
    pub sampler: SamplerKind,
}

/// Run a fullscreen pass. `clear` is the (premultiplied) clear color; None keeps the target contents.
pub fn run_pass<'a>(
    ctx: &GpuContext,
    enc: &mut wgpu::CommandEncoder,
    shader: &CompiledShader,
    uniform_buf: Option<&wgpu::Buffer>,
    textures: &dyn Fn(&str) -> Option<TexInput<'a>>,
    fallback: &wgpu::TextureView,
    target: &wgpu::TextureView,
    clear: Option<[f64; 4]>,
) {
    let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();
    if let (Some(u), Some(b)) = (&shader.uniform, uniform_buf) {
        entries.push(wgpu::BindGroupEntry { binding: u.binding, resource: b.as_entire_binding() });
    }
    let inputs: Vec<(u32, Option<u32>, Option<TexInput>)> = shader.textures.iter().map(|t| (t.tex, t.samp, textures(&t.name))).collect();
    for (tex, samp, inp) in &inputs {
        let (view, sk) = match inp {
            Some(i) => (i.view, i.sampler),
            None => (fallback, SamplerKind::LinearClamp),
        };
        entries.push(wgpu::BindGroupEntry { binding: *tex, resource: wgpu::BindingResource::TextureView(view) });
        if let Some(s) = samp {
            entries.push(wgpu::BindGroupEntry { binding: *s, resource: wgpu::BindingResource::Sampler(ctx.samplers.get(sk)) });
        }
    }
    let bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(&shader.name), layout: &shader.bgl, entries: &entries });
    let load = match clear {
        Some(c) => wgpu::LoadOp::Clear(wgpu::Color { r: c[0], g: c[1], b: c[2], a: c[3] }),
        None => wgpu::LoadOp::Load,
    };
    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(&shader.name),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
        })],
        ..Default::default()
    });
    rp.set_pipeline(&shader.pipeline);
    rp.set_bind_group(0, &bg, &[]);
    rp.draw(0..3, 0..1);
}

/// A uniform buffer sized for a layout, reused every frame.
pub struct UniformBuf {
    pub buf: wgpu::Buffer,
}

impl UniformBuf {
    pub fn new(ctx: &GpuContext, size: u32) -> Self {
        let buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: size.max(16) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buf }
    }
    pub fn write(&self, ctx: &GpuContext, data: &[u8]) {
        ctx.queue.write_buffer(&self.buf, 0, data);
    }
}

/// Helper owning a target for convenience in tests.
pub fn blank_target(ctx: &GpuContext) -> Target {
    let t = Target::new(ctx, 1, 1, "blank");
    t.upload_rgba8(ctx, &[0, 0, 0, 0]);
    t
}
