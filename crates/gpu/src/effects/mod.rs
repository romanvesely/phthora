//! GPU implementations of effects.

pub mod graph;
pub mod hooks;
mod person;

use crate::assets::Assets;
use crate::context::{GpuContext, SamplerKind, Target};
use crate::shader::{pack_uniforms, run_pass, CompiledShader, ShaderLibrary, TexInput, UVal, UniformBuf};
use phthora_core::clock::FrameTime;
use phthora_core::params::{ParamKind, Value};
use phthora_core::{EffectKind, ResolvedEffect};
use std::rc::Rc;

/// Diagnostics: (effect id, uniform names) that got no value.
pub static MISSING_UNIFORMS: std::sync::Mutex<Vec<(String, Vec<String>)>> = std::sync::Mutex::new(Vec::new());

pub struct RenderCtx<'a> {
    pub ctx: &'a GpuContext,
    pub lib: &'a ShaderLibrary,
    pub enc: &'a mut wgpu::CommandEncoder,
    pub ft: &'a FrameTime,
    pub width: u32,
    pub height: u32,
    /// premultiplied background used to clear every pass target
    pub clear: [f64; 4],
    pub blank: &'a Target,
    /// the chain's source after the input stage (what the first effect receives)
    pub source: &'a Target,
    pub assets: &'a mut Assets,
    /// log-frequency audio spectrum of this frame, 0..1 (empty without audio)
    pub audio: &'a [f32],
    /// live drawing canvases by effect uid (the editor); without one the effect's data is used
    pub paint: &'a std::collections::HashMap<u64, crate::paint::PaintCanvas>,
    /// effects that need the image on the CPU (person segmentation): true waits for the
    /// result every frame (export, same mask for the same frame), false never waits and
    /// uses the newest mask that is done (live preview)
    pub blocking: bool,
    /// person mask supplied by the caller instead of the network
    pub person_mask: Option<&'a phthora_seg::Mask>,
}

pub trait GpuEffect {
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, orig: &Target, out: &Target);
    /// called when the render size changes
    fn resize(&mut self, _ctx: &GpuContext, _w: u32, _h: u32) {}
    /// clear history buffers
    fn reset(&mut self, _ctx: &GpuContext) {}
    /// why the effect cannot do its job (a file that cannot be read), shown in the editor
    fn problem(&self) -> Option<String> {
        None
    }
}

/// Uniform values every effect shader can use, derived from the effect params.
pub fn base_uniform(e: &ResolvedEffect, rc: &RenderCtx, key: &str) -> Option<UVal> {
    match key {
        "time" => return Some(UVal::F(e.time)),
        "resolution" => return Some(UVal::V2([rc.width as f32, rc.height as f32])),
        "exportProgress" => return Some(UVal::F(rc.ft.export_progress as f32)),
        "aspect" => return Some(UVal::F(rc.width as f32 / rc.height.max(1) as f32)),
        _ => {}
    }
    if let Some(v) = e.values.get(key) {
        return value_to_uval(v);
    }
    if let Some(b) = key.strip_prefix("b_")
        && let Some(v) = e.values.get(b) {
            return Some(UVal::F(v.as_bool() as i32 as f32));
        }
    // combo index: `blendModeIndex` for combo `blendMode`
    if let Some(base) = key.strip_suffix("Index")
        && let (Some(p), Some(v)) = (e.desc.param(base), e.values.get(base))
            && let ParamKind::Combo { values, .. } = &p.kind {
                return Some(UVal::F(values.iter().position(|x| x == v.as_str()).map(|i| i as f32).unwrap_or(-1.0)));
            }
    // color stops -> color1..6, stop1..6, numStops
    if let Some(stops) = e.values.values().find_map(|v| if let Value::Stops(s) = v { Some(s) } else { None }) {
        if key == "numStops" {
            return Some(UVal::F(stops.len() as f32));
        }
        for (prefix, is_color) in [("color", true), ("stop", false)] {
            if let Some(n) = key.strip_prefix(prefix).and_then(|n| n.parse::<usize>().ok()) {
                return Some(match stops.get(n.wrapping_sub(1)) {
                    Some(s) if is_color => UVal::V3(phthora_core::params::parse_hex(&s.color)),
                    Some(s) => UVal::F(s.stop),
                    None if is_color => UVal::V3([0.0; 3]),
                    None => UVal::F(0.0),
                });
            }
        }
    }
    None
}

/// Default value a shader declares for a non-GUI uniform.
pub fn shader_default(shader: &str, key: &str) -> Option<UVal> {
    use std::sync::OnceLock;
    static D: OnceLock<serde_json::Value> = OnceLock::new();
    let d = D.get_or_init(|| serde_json::from_str(include_str!("../../data/shader_defaults.json")).expect("valid json"));
    let v = d.get(shader)?.get(key)?;
    Some(match v {
        serde_json::Value::Number(n) => UVal::F(n.as_f64()? as f32),
        serde_json::Value::String(s) => UVal::V3(phthora_core::params::parse_hex(s)),
        serde_json::Value::Array(a) => {
            let f: Vec<f32> = a.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
            match f.len() {
                2 => UVal::V2([f[0], f[1]]),
                3 => UVal::V3([f[0], f[1], f[2]]),
                4 => UVal::V4([f[0], f[1], f[2], f[3]]),
                _ => return None,
            }
        }
        _ => return None,
    })
}

pub fn value_to_uval(v: &Value) -> Option<UVal> {
    Some(match v {
        Value::Float(f) => UVal::F(*f),
        Value::Bool(b) => UVal::F(*b as i32 as f32),
        Value::Color(c) => UVal::V3(*c),
        _ => return None,
    })
}

/// Extra uniforms/textures a hook provides for a shader effect.
pub trait ShaderHook {
    fn prepare(&mut self, _rc: &mut RenderCtx, _e: &ResolvedEffect) {}
    /// Like `prepare`, for hooks that also need the effect's input image.
    fn prepare_input(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, _input: &Target) {
        self.prepare(rc, e);
    }
    fn uniform(&self, _key: &str) -> Option<UVal> {
        None
    }
    fn texture(&self, _name: &str) -> Option<TexInput<'_>> {
        None
    }
    /// see `GpuEffect::problem`
    fn problem(&self) -> Option<String> {
        None
    }
    fn sampler_for(&self, _name: &str) -> Option<SamplerKind> {
        None
    }
}

/// A single fullscreen fragment shader.
pub struct ShaderEffect {
    pub shader: Rc<CompiledShader>,
    ubuf: UniformBuf,
    pub hook: Option<Box<dyn ShaderHook>>,
    warned: bool,
}

impl ShaderEffect {
    pub fn new(ctx: &GpuContext, lib: &ShaderLibrary, shader: &str, hook: Option<Box<dyn ShaderHook>>) -> Self {
        let shader = lib.get(shader);
        let size = shader.uniform.as_ref().map(|u| u.size).unwrap_or(16);
        Self { ubuf: UniformBuf::new(ctx, size), shader, hook, warned: false }
    }

    /// Uniforms the shader declares but nobody provides (for tests).
    pub fn missing_uniforms(&self, rc: &RenderCtx, e: &ResolvedEffect) -> Vec<String> {
        let Some(layout) = &self.shader.uniform else { return vec![] };
        let name = &self.shader.name;
        let lookup = |k: &str| {
            self.hook.as_ref().and_then(|h| h.uniform(k)).or_else(|| base_uniform(e, rc, k)).or_else(|| shader_default(name, k))
        };
        pack_uniforms(layout, &lookup).1
    }
}

impl GpuEffect for ShaderEffect {
    fn problem(&self) -> Option<String> {
        self.hook.as_ref().and_then(|h| h.problem())
    }
    fn render(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target, orig: &Target, out: &Target) {
        if let Some(h) = self.hook.as_mut() {
            h.prepare_input(rc, e, input);
        }
        if let Some(layout) = &self.shader.uniform {
            let hook = &self.hook;
            let name = &self.shader.name;
            let lookup = |k: &str| {
                hook.as_ref().and_then(|h| h.uniform(k)).or_else(|| base_uniform(e, rc, k)).or_else(|| shader_default(name, k))
            };
            let (data, missing) = pack_uniforms(layout, &lookup);
            if !missing.is_empty() && !self.warned {
                eprintln!("[{}] uniforms without value: {:?}", e.desc.id, missing);
                MISSING_UNIFORMS.lock().unwrap().push((e.desc.id.clone(), missing));
                self.warned = true;
            }
            self.ubuf.write(rc.ctx, &data);
        }
        let hook = &self.hook;
        let source = rc.source;
        let textures = |name: &str| -> Option<TexInput> {
            if let Some(t) = hook.as_ref().and_then(|h| h.texture(name)) {
                return Some(t);
            }
            let sampler = hook.as_ref().and_then(|h| h.sampler_for(name)).unwrap_or_default();
            match name {
                "tDiffuse" => Some(TexInput { view: &input.view, sampler }),
                "tOrig" => Some(TexInput { view: &orig.view, sampler }),
                "tSource" => Some(TexInput { view: &source.view, sampler }),
                _ => None,
            }
        };
        run_pass(rc.ctx, rc.enc, &self.shader, Some(&self.ubuf.buf), &textures, &rc.blank.view, &out.view, Some(rc.clear));
    }
}

/// Create the GPU implementation for an effect.
pub fn create(ctx: &GpuContext, lib: &ShaderLibrary, e: &ResolvedEffect) -> Box<dyn GpuEffect> {
    if let Some(custom) = graph::create_custom(ctx, lib, e.desc) {
        return custom;
    }
    let shader = e.desc.shader.as_deref().unwrap_or("copy");
    if e.desc.kind == EffectKind::Custom {
        eprintln!("effect '{}' has no custom implementation yet, using '{shader}'", e.desc.id);
    }
    Box::new(ShaderEffect::new(ctx, lib, shader, hooks::create_hook(ctx, e.desc)))
}
