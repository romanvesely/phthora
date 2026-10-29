//! Pipelined readback of rendered frames for export: the output pass converts each frame
//! on the GPU, and frames are read from a ring of buffers so rendering frame n overlaps
//! with copying frame n-2.

use crate::context::{padded_row, GpuContext, Target};
use crate::shader::{run_pass, Blend, CompiledShader, ShaderLibrary, TexInput};
use crate::context::FORMAT;
use std::collections::VecDeque;
use std::rc::Rc;

const RING: usize = 3;

struct Pending {
    buf: usize,
    submission: wgpu::SubmissionIndex,
}

pub struct Readback {
    ctx: GpuContext,
    shader: Rc<CompiledShader>,
    out: Target,
    bufs: Vec<wgpu::Buffer>,
    free: Vec<usize>,
    pending: VecDeque<Pending>,
}

impl Readback {
    pub fn new(ctx: &GpuContext, lib: &ShaderLibrary, width: u32, height: u32) -> Self {
        let size = (padded_row(width) * height) as u64;
        let bufs = (0..RING)
            .map(|_| {
                ctx.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("readback"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Self {
            ctx: ctx.clone(),
            shader: lib.get_with("output", Blend::Replace, FORMAT),
            out: Target::new(ctx, width, height, "output"),
            bufs,
            free: (0..RING).rev().collect(),
            pending: VecDeque::new(),
        }
    }

    /// Queue `frame` (the engine output) for reading. Returns the oldest queued frame
    /// (top-down, straight RGBA8) once the ring is full.
    pub fn push(&mut self, frame: &Target, blank: &Target) -> Option<Vec<u8>> {
        let done = if self.free.is_empty() { self.pop() } else { None };
        let i = self.free.pop().expect("a free buffer");
        let (w, h) = self.out.size();
        let mut enc = self.ctx.device.create_command_encoder(&Default::default());
        let tex = |n: &str| (n == "src").then_some(TexInput { view: &frame.view, sampler: Default::default() });
        run_pass(&self.ctx, &mut enc, &self.shader, None, &tex, &blank.view, &self.out.view, Some([0.0; 4]));
        let row = padded_row(w);
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &self.out.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.bufs[i],
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let submission = self.ctx.queue.submit([enc.finish()]);
        self.bufs[i].slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.pending.push_back(Pending { buf: i, submission });
        done
    }

    /// The oldest queued frame, waiting for the GPU if needed.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        let p = self.pending.pop_front()?;
        let _ = self.ctx.device.poll(wgpu::PollType::Wait { submission_index: Some(p.submission), timeout: None });
        let (w, h) = self.out.size();
        let row = padded_row(w) as usize;
        let buf = &self.bufs[p.buf];
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        {
            let data = buf.slice(..).get_mapped_range().expect("mapped");
            for y in 0..h as usize {
                px.extend_from_slice(&data[y * row..y * row + w as usize * 4]);
            }
        }
        buf.unmap();
        self.free.push(p.buf);
        Some(px)
    }
}
