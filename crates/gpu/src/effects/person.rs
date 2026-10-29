//! Remove Background: the person mask comes from a network on the CPU (crates/seg).
//! The effect's input is shrunk on the GPU, read back, segmented, and the mask is uploaded
//! as a texture for the shader. Export waits for every frame; the live preview reads
//! back asynchronously and a worker thread segments the newest frame.

use super::{RenderCtx, ShaderHook};
use crate::assets;
use crate::context::{copy_to_buffer, flip_rows, map_buffer, padded_row, GpuContext, SamplerKind, Target, FORMAT};
use crate::shader::{pack_uniforms, run_pass, Blend, TexInput, UVal, UniformBuf};
use phthora_core::ResolvedEffect;
use phthora_seg::{Mask, Segmenter, Status, Worker};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Mask texels a full `expand` / `feather` slider reaches.
const EXPAND_TEXELS: f32 = 10.0;
const FEATHER_TEXELS: f32 = 8.0;

/// Size the frame is shrunk to before readback: twice the network input, so the final
/// halving happens as an exact area average on the CPU.
fn readback_size() -> (u32, u32) {
    let (w, h) = Segmenter::input_size();
    (w * 2, h * 2)
}

enum StageState {
    Free,
    /// copy recorded into this frame's commands, submitted with them
    Recorded,
    /// map requested: 0 pending, 1 ready, 2 failed
    Mapping(Arc<AtomicU8>),
}

/// One staging buffer of the asynchronous readback.
struct Stage {
    buf: Option<wgpu::Buffer>,
    state: StageState,
}

/// The raw mask and the two post-processing results, all at mask resolution.
struct MaskTextures {
    raw: Target,
    grown: Target,
    soft: Target,
}

pub struct RemoveBackground {
    masks: Option<MaskTextures>,
    /// a mask is available (otherwise the effect passes the image through)
    ready: bool,
    /// the last mask given through `RenderCtx::person_mask`
    given: Option<Mask>,
    /// serial of the last worker mask uploaded
    serial: u64,
    white: Target,
    /// downsampling chain and the final readback target
    steps: Vec<Target>,
    small: Option<Target>,
    segmenter: Option<Segmenter>,
    worker: Option<Worker>,
    stage: [Stage; 2],
    morph: UniformBuf,
    blur: UniformBuf,
}

impl RemoveBackground {
    pub fn new(ctx: &GpuContext) -> Self {
        let free = || Stage { buf: None, state: StageState::Free };
        Self {
            masks: None,
            ready: false,
            given: None,
            serial: 0,
            white: assets::texture_from_rgba(ctx, 1, 1, &[255; 4], false, "white"),
            steps: Vec::new(),
            small: None,
            segmenter: None,
            worker: None,
            stage: [free(), free()],
            morph: UniformBuf::new(ctx, 16),
            blur: UniformBuf::new(ctx, 16),
        }
    }

    /// Upload a person mask (top-down) as the raw mask texture.
    fn set_mask(&mut self, rc: &RenderCtx, m: &Mask) {
        let size = (m.w, m.h);
        if self.masks.as_ref().map(|t| t.raw.size()) != Some(size) {
            let mk = |l: &str| Target::new(rc.ctx, m.w, m.h, l);
            self.masks = Some(MaskTextures { raw: mk("person-mask"), grown: mk("person-grown"), soft: mk("person-soft") });
        }
        let rgba: Vec<u8> = m.data.iter().flat_map(|&v| [v, v, v, 255]).collect();
        assets::upload(rc.ctx, &self.masks.as_ref().unwrap().raw, &rgba, true);
        self.ready = true;
    }

    /// Shrink `input` into `self.small` (box filter by repeated bilinear halving first).
    fn shrink(&mut self, rc: &mut RenderCtx, input: &Target) {
        let (tw, th) = readback_size();
        let copy = rc.lib.get_with("copy", Blend::Replace, FORMAT);
        let mut sizes = Vec::new();
        let (mut w, mut h) = input.size();
        while w > tw * 3 && h > th * 3 {
            (w, h) = (w.div_ceil(2), h.div_ceil(2));
            sizes.push((w, h));
        }
        if self.steps.iter().map(Target::size).ne(sizes.iter().copied()) {
            self.steps = sizes.iter().map(|&(w, h)| Target::new(rc.ctx, w, h, "person-step")).collect();
        }
        if self.small.is_none() {
            self.small = Some(Target::new(rc.ctx, tw, th, "person-small"));
        }
        let small = self.small.as_ref().unwrap();
        let mut src = &input.view;
        for dst in self.steps.iter().chain(std::iter::once(small)) {
            let tex = |n: &str| (n == "tDiffuse").then_some(TexInput { view: src, sampler: SamplerKind::LinearClamp });
            run_pass(rc.ctx, rc.enc, &copy, None, &tex, &rc.blank.view, &dst.view, Some([0.0; 4]));
            src = &dst.view;
        }
    }

    /// Export: submit what has been recorded so far, read the shrunk frame back and segment it.
    fn segment_blocking(&mut self, rc: &mut RenderCtx, input: &Target) {
        if !matches!(phthora_seg::status(), Status::Available | Status::Ready | Status::Loading) {
            return;
        }
        if self.segmenter.is_none() {
            match Segmenter::load_installed() {
                Ok(s) => self.segmenter = Some(s),
                Err(_) => return,
            }
        }
        self.shrink(rc, input);
        let small = self.small.as_ref().unwrap();
        let buf = copy_to_buffer(rc.ctx, rc.enc, small);
        // the frame so far has to run before the data exists
        let fresh = rc.ctx.device.create_command_encoder(&Default::default());
        let done = std::mem::replace(rc.enc, fresh);
        rc.ctx.queue.submit([done.finish()]);
        let (w, h) = small.size();
        let mut px = map_buffer(rc.ctx, &buf, w, h);
        flip_rows(&mut px, w, h);
        if let Ok(mask) = self.segmenter.as_ref().unwrap().run(&px, w, h) {
            self.set_mask(rc, &mask);
        }
    }

    /// Preview: never waits. Reads back with a delay of a frame or two, the worker
    /// segments the newest frame, the newest finished mask is used.
    fn segment_async(&mut self, rc: &mut RenderCtx, input: &Target) {
        if !matches!(phthora_seg::status(), Status::Available | Status::Ready | Status::Loading) {
            // missing, downloading or failed: a worker started later loads the new file
            self.worker = None;
            return;
        }
        let worker = self.worker.get_or_insert_with(Worker::spawn);
        pump(&mut self.stage, worker, rc.ctx, readback_size());
        if let Some((serial, mask)) = worker.latest()
            && serial != self.serial
        {
            self.serial = serial;
            self.set_mask(rc, &mask);
        }
        // record the next copy unless one is still waiting to be mapped
        if self.stage.iter().any(|s| matches!(s.state, StageState::Recorded)) {
            return;
        }
        let Some(i) = self.stage.iter().position(|s| matches!(s.state, StageState::Free)) else { return };
        self.shrink(rc, input);
        let small = self.small.as_ref().unwrap();
        let (w, h) = small.size();
        let buf = self.stage[i].buf.get_or_insert_with(|| {
            rc.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("person-readback"),
                size: (padded_row(w) * h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        });
        rc.enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &small.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
            wgpu::TexelCopyBufferInfo {
                buffer: buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded_row(w)), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.stage[i].state = StageState::Recorded;
    }

    /// Grow/shrink and soften the raw mask into `soft`.
    fn process(&self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        let Some(m) = &self.masks else { return };
        let shader = rc.lib.get_with("remove_background_mask", Blend::Replace, FORMAT);
        let layout = shader.uniform.as_ref().expect("mask shader has uniforms");
        let steps = [
            (&self.morph, 0.0, e.f("expand") * EXPAND_TEXELS, &m.raw, &m.grown),
            (&self.blur, 1.0, e.f("feather") * FEATHER_TEXELS, &m.grown, &m.soft),
        ];
        for (ubuf, mode, radius, src, dst) in steps {
            let (data, _) = pack_uniforms(layout, &|k: &str| match k {
                "radius" => Some(UVal::F(radius)),
                "mode" => Some(UVal::F(mode)),
                _ => None,
            });
            ubuf.write(rc.ctx, &data);
            let tex = |n: &str| (n == "tDiffuse").then_some(TexInput { view: &src.view, sampler: SamplerKind::LinearClamp });
            run_pass(rc.ctx, rc.enc, &shader, Some(&ubuf.buf), &tex, &rc.blank.view, &dst.view, Some([0.0; 4]));
        }
    }
}

/// Ask for the mapping of buffers recorded in an earlier frame, collect the ones that are
/// ready and hand their frames to the worker. Never blocks.
fn pump(stage: &mut [Stage; 2], worker: &Worker, ctx: &GpuContext, (w, h): (u32, u32)) {
    for s in stage.iter_mut() {
        if let (StageState::Recorded, Some(buf)) = (&s.state, &s.buf) {
            let flag = Arc::new(AtomicU8::new(0));
            let f = flag.clone();
            buf.slice(..).map_async(wgpu::MapMode::Read, move |r| f.store(if r.is_ok() { 1 } else { 2 }, Ordering::Release));
            s.state = StageState::Mapping(flag);
        }
    }
    let _ = ctx.device.poll(wgpu::PollType::Poll);
    for s in stage.iter_mut() {
        let StageState::Mapping(flag) = &s.state else { continue };
        match flag.load(Ordering::Acquire) {
            1 => {
                let buf = s.buf.as_ref().unwrap();
                let row = padded_row(w) as usize;
                let mut px = Vec::with_capacity((w * h * 4) as usize);
                if let Ok(data) = buf.slice(..).get_mapped_range() {
                    // rows come bottom-up; the network wants top-down
                    for y in (0..h as usize).rev() {
                        px.extend_from_slice(&data[y * row..y * row + w as usize * 4]);
                    }
                }
                buf.unmap();
                if !px.is_empty() {
                    worker.submit(px, w, h);
                }
                s.state = StageState::Free;
            }
            2 => s.state = StageState::Free,
            _ => {}
        }
    }
}

impl ShaderHook for RemoveBackground {
    fn prepare_input(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect, input: &Target) {
        if let Some(m) = rc.person_mask {
            if self.given.as_ref() != Some(m) {
                self.set_mask(rc, m);
                self.given = Some(m.clone());
            }
        } else if rc.blocking {
            self.segment_blocking(rc, input);
        } else {
            self.segment_async(rc, input);
        }
        if self.ready {
            self.process(rc, e);
        }
    }

    fn uniform(&self, key: &str) -> Option<UVal> {
        // without a mask the image passes through unchanged
        (!self.ready && matches!(key, "amount" | "b_invert" | "b_showMask")).then_some(UVal::F(0.0))
    }

    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        if name != "tMask" {
            return None;
        }
        let t = match &self.masks {
            Some(m) if self.ready => &m.soft,
            _ => &self.white,
        };
        Some(TexInput { view: &t.view, sampler: SamplerKind::LinearClamp })
    }
}
