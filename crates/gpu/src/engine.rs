//! Renders a resolved effect chain for one frame.

use crate::assets::Assets;
use crate::context::{GpuContext, SamplerKind, Target, FORMAT};
use crate::effects::{self, GpuEffect, RenderCtx};
use crate::shader::{pack_uniforms, run_pass, Blend, CompiledShader, ShaderLibrary, TexInput, UVal, UniformBuf};
use phthora_core::chain::{mask_source, GroupRef, MaskSource};
use phthora_core::{FrameTime, GroupBlend, ResolvedEffect};
use std::collections::HashMap;
use std::rc::Rc;

struct Slot {
    effect: Box<dyn GpuEffect>,
    out: Target,
    id: String,
}

/// Output of a group's blend pass.
struct GroupSlot {
    out: Target,
    ubuf: UniformBuf,
}

fn pick<'a>(t: Tex, base: &'a Target, clear: &'a Target, slots: &'a HashMap<u64, Slot>, groups: &'a HashMap<u64, GroupSlot>) -> &'a Target {
    match t {
        Tex::Base => base,
        Tex::Clear => clear,
        Tex::Effect(uid) => &slots[&uid].out,
        Tex::Group(uid) => &groups[&uid].out,
    }
}

/// A texture of the current frame.
#[derive(Clone, Copy)]
enum Tex {
    Base,
    /// full-size transparent image, the start of an isolated group
    Clear,
    Effect(u64),
    Group(u64),
}

pub struct Engine {
    pub ctx: GpuContext,
    pub lib: ShaderLibrary,
    width: u32,
    height: u32,
    source: Option<Target>,
    base: Target,
    blank: Target,
    clear_target: Target,
    slots: HashMap<u64, Slot>,
    groups: HashMap<u64, GroupSlot>,
    group_shader: Rc<CompiledShader>,
    input_shader: Rc<CompiledShader>,
    input_ubuf: UniformBuf,
    /// straight-alpha background color (rgb) and opacity
    pub background: [f32; 4],
    pub flip_x: bool,
    pub assets: Assets,
    /// log-frequency audio spectrum for the next render (audio visualizer)
    pub audio_spectrum: Vec<f32>,
    /// live drawing canvases by effect uid
    pub paint: HashMap<u64, crate::paint::PaintCanvas>,
    last_output: Tex,
}

impl Engine {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let lib = ShaderLibrary::new(ctx);
        let input_shader = lib.get("input");
        let group_shader = lib.get_with("group_blend", Blend::Replace, FORMAT);
        Self {
            ctx: ctx.clone(),
            width,
            height,
            source: None,
            base: Target::new(ctx, width, height, "base"),
            blank: crate::shader::blank_target(ctx),
            clear_target: Target::new(ctx, width, height, "clear"),
            slots: HashMap::new(),
            groups: HashMap::new(),
            group_shader,
            input_ubuf: UniformBuf::new(ctx, input_shader.uniform.as_ref().map(|u| u.size).unwrap_or(16)),
            input_shader,
            lib,
            background: [0.0, 0.0, 0.0, 1.0],
            flip_x: false,
            assets: Assets::new(),
            audio_spectrum: Vec::new(),
            paint: HashMap::new(),
            last_output: Tex::Base,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        self.base = Target::new(&self.ctx, width, height, "base");
        self.clear_target = Target::new(&self.ctx, width, height, "clear");
        self.groups.clear();
        for s in self.slots.values_mut() {
            s.out = Target::new(&self.ctx, width, height, &s.id);
            s.effect.resize(&self.ctx, width, height);
        }
    }

    /// Upload a source frame (top-down rows, straight alpha RGBA8).
    pub fn set_source_rgba(&mut self, width: u32, height: u32, data: &[u8]) {
        if self.source.as_ref().map(|s| s.size()) != Some((width, height)) {
            self.source = Some(Target::new(&self.ctx, width, height, "source"));
        }
        self.source.as_ref().unwrap().upload_rgba8(&self.ctx, data);
    }

    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Clear all effect history (feedback trails etc.).
    pub fn reset_history(&mut self) {
        for s in self.slots.values_mut() {
            s.effect.reset(&self.ctx);
            s.out.clear(&self.ctx, [0.0; 4]);
        }
    }

    fn clear_color(&self) -> [f64; 4] {
        let [r, g, b, a] = self.background;
        [(r * a) as f64, (g * a) as f64, (b * a) as f64, a as f64]
    }

    /// Render one frame. Returns the final target (bottom-up rows, premultiplied).
    pub fn render(&mut self, effects: &[ResolvedEffect], ft: &FrameTime) -> &Target {
        for e in effects {
            if !self.slots.contains_key(&e.uid) {
                let effect = effects::create(&self.ctx, &self.lib, e);
                let out = Target::new(&self.ctx, self.width, self.height, &e.desc.id);
                self.slots.insert(e.uid, Slot { effect, out, id: e.desc.id.clone() });
            }
        }
        let clear = self.clear_color();
        let mut enc = self.ctx.device.create_command_encoder(&Default::default());

        // input stage
        if let Some(src) = &self.source {
            if let Some(layout) = &self.input_shader.uniform {
                let fx = self.flip_x as i32 as f32;
                let (data, _) = pack_uniforms(layout, &|k: &str| match k {
                    "flip_x" => Some(UVal::F(fx)),
                    "force_opaque" => Some(UVal::F(0.0)),
                    _ => None,
                });
                self.input_ubuf.write(&self.ctx, &data);
            }
            let tex = |n: &str| (n == "src").then_some(TexInput { view: &src.view, sampler: SamplerKind::LinearClamp });
            run_pass(&self.ctx, &mut enc, &self.input_shader, Some(&self.input_ubuf.buf), &tex, &self.blank.view, &self.base.view, Some(clear));
        } else {
            self.base.clear_in(&mut enc, clear);
        }

        self.groups.retain(|g, _| effects.iter().any(|e| e.group.is_some_and(|r| r.uid == *g)));
        // what each effect got as input, and the input of the group it is in
        let mut input_of: Vec<Tex> = Vec::with_capacity(effects.len());
        let mut group_input_of: Vec<Tex> = Vec::with_capacity(effects.len());
        let mut prev = Tex::Base;
        let mut open: Option<(GroupRef, Tex)> = None;
        for (i, e) in effects.iter().enumerate() {
            if let Some((g, outside)) = open
                && e.group.map(|r| r.uid) != Some(g.uid) {
                    prev = self.blend_group(&mut enc, g, outside, prev);
                    open = None;
                }
            if let (Some(g), None) = (e.group, open) {
                open = Some((g, prev));
                if g.isolate {
                    prev = Tex::Clear;
                }
            }
            input_of.push(prev);
            group_input_of.push(open.map(|(_, o)| o).unwrap_or(prev));
            // temporarily take the slot out so we can borrow other slots' outputs
            let mut slot = self.slots.remove(&e.uid).unwrap();
            {
                let tex = |t| pick(t, &self.base, &self.clear_target, &self.slots, &self.groups);
                let input = tex(prev);
                let orig = match mask_source(effects, i) {
                    MaskSource::Blank => &self.blank,
                    MaskSource::InputOf(j) => tex(input_of[j]),
                    MaskSource::GroupInput(j) => tex(group_input_of[j]),
                };
                // an isolated group composes on transparent, not on the background
                let clear = if open.is_some_and(|(g, _)| g.isolate) { [0.0; 4] } else { clear };
                let mut rc = RenderCtx {
                    ctx: &self.ctx,
                    lib: &self.lib,
                    enc: &mut enc,
                    ft,
                    width: self.width,
                    height: self.height,
                    clear,
                    blank: &self.blank,
                    assets: &mut self.assets,
                    audio: &self.audio_spectrum,
                    paint: &self.paint,
                };
                slot.effect.render(&mut rc, e, input, orig, &slot.out);
            }
            self.slots.insert(e.uid, slot);
            prev = Tex::Effect(e.uid);
        }
        if let Some((g, outside)) = open {
            prev = self.blend_group(&mut enc, g, outside, prev);
        }
        self.ctx.queue.submit([enc.finish()]);
        self.last_output = prev;
        self.output()
    }

    fn tex(&self, t: Tex) -> &Target {
        pick(t, &self.base, &self.clear_target, &self.slots, &self.groups)
    }

    /// Combine a finished group with the image that went into it.
    fn blend_group(&mut self, enc: &mut wgpu::CommandEncoder, g: GroupRef, outside: Tex, result: Tex) -> Tex {
        let mode = match (g.blend, g.isolate) {
            // without isolation the members already started from the input
            (GroupBlend::Normal, false) => return result,
            (GroupBlend::Normal, true) => 0.0,
            (GroupBlend::Mask, _) => 1.0,
            (GroupBlend::Behind, _) => 2.0,
        };
        if !self.groups.contains_key(&g.uid) {
            let size = self.group_shader.uniform.as_ref().map(|u| u.size).unwrap_or(16);
            let slot = GroupSlot { out: Target::new(&self.ctx, self.width, self.height, "group"), ubuf: UniformBuf::new(&self.ctx, size) };
            self.groups.insert(g.uid, slot);
        }
        let slot = &self.groups[&g.uid];
        if let Some(layout) = &self.group_shader.uniform {
            let (data, _) = pack_uniforms(layout, &|k: &str| (k == "mode").then_some(UVal::F(mode)));
            slot.ubuf.write(&self.ctx, &data);
        }
        let (inp, res) = (self.tex(outside), self.tex(result));
        let tex = |n: &str| match n {
            "tInput" => Some(TexInput { view: &inp.view, sampler: SamplerKind::LinearClamp }),
            "tGroup" => Some(TexInput { view: &res.view, sampler: SamplerKind::LinearClamp }),
            _ => None,
        };
        run_pass(&self.ctx, enc, &self.group_shader, Some(&slot.ubuf.buf), &tex, &self.blank.view, &slot.out.view, Some([0.0; 4]));
        Tex::Group(g.uid)
    }

    /// Drop the GPU state of effects that are gone. Disabled effects keep theirs, so
    /// switching an effect off and on again does not rebuild it.
    pub fn retain(&mut self, alive: impl Fn(u64) -> bool) {
        self.slots.retain(|uid, _| alive(*uid));
    }

    /// 1x1 transparent texture (fallback binding).
    pub fn blank(&self) -> &Target {
        &self.blank
    }

    /// Result of the last render.
    pub fn output(&self) -> &Target {
        match self.last_output {
            Tex::Effect(u) if self.slots.contains_key(&u) => &self.slots[&u].out,
            Tex::Group(u) if self.groups.contains_key(&u) => &self.groups[&u].out,
            Tex::Base | Tex::Clear | Tex::Effect(_) | Tex::Group(_) => &self.base,
        }
    }

    /// Read the last output as top-down RGBA8 (premultiplied).
    pub fn read_output(&self) -> Vec<u8> {
        let t = self.output();
        let mut px = crate::context::read_rgba8(&self.ctx, t);
        crate::context::flip_rows(&mut px, t.width, t.height);
        px
    }
}
