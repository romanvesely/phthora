//! Device/queue handling, render targets, samplers, readback.

use std::sync::Arc;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Clone)]
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub samplers: Arc<Samplers>,
}

impl GpuContext {
    pub fn from_parts(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let samplers = Arc::new(Samplers::new(&device));
        Self { device, queue, samplers }
    }

    /// Headless device (tests, CLI).
    pub fn headless() -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|e| format!("no GPU adapter: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("phthora"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| format!("device: {e}"))?;
        Ok(Self::from_parts(device, queue))
    }

    pub fn wait(&self) {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SamplerKind {
    #[default]
    LinearClamp,
    NearestClamp,
    LinearRepeat,
    NearestRepeat,
}

pub struct Samplers {
    pub linear_clamp: wgpu::Sampler,
    pub nearest_clamp: wgpu::Sampler,
    pub linear_repeat: wgpu::Sampler,
    pub nearest_repeat: wgpu::Sampler,
}

impl Samplers {
    fn new(device: &wgpu::Device) -> Self {
        let mk = |filter: wgpu::FilterMode, mode: wgpu::AddressMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: mode,
                address_mode_v: mode,
                address_mode_w: mode,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };
        use wgpu::{AddressMode::*, FilterMode::*};
        Self {
            linear_clamp: mk(Linear, ClampToEdge),
            nearest_clamp: mk(Nearest, ClampToEdge),
            linear_repeat: mk(Linear, Repeat),
            nearest_repeat: mk(Nearest, Repeat),
        }
    }

    pub fn get(&self, k: SamplerKind) -> &wgpu::Sampler {
        match k {
            SamplerKind::LinearClamp => &self.linear_clamp,
            SamplerKind::NearestClamp => &self.nearest_clamp,
            SamplerKind::LinearRepeat => &self.linear_repeat,
            SamplerKind::NearestRepeat => &self.nearest_repeat,
        }
    }
}

/// A color render target / texture.
pub struct Target {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl Target {
    pub fn new(ctx: &GpuContext, width: u32, height: u32, label: &str) -> Self {
        Self::with_format(ctx, width, height, FORMAT, label)
    }

    pub fn with_format(ctx: &GpuContext, width: u32, height: u32, format: wgpu::TextureFormat, label: &str) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view, width, height, format }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Upload tightly packed RGBA8 rows (row 0 first).
    pub fn upload_rgba8(&self, ctx: &GpuContext, data: &[u8]) {
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(self.width * 4), rows_per_image: Some(self.height) },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
    }

    pub fn clear(&self, ctx: &GpuContext, color: [f64; 4]) {
        let mut enc = ctx.device.create_command_encoder(&Default::default());
        self.clear_in(&mut enc, color);
        ctx.queue.submit([enc.finish()]);
    }

    pub fn clear_in(&self, enc: &mut wgpu::CommandEncoder, c: [f64; 4]) {
        let _ = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: c[0], g: c[1], b: c[2], a: c[3] }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
}

/// Read an RGBA8 target back to the CPU. Returns rows in texture order (row 0 first).
pub fn read_rgba8(ctx: &GpuContext, t: &Target) -> Vec<u8> {
    let mut enc = ctx.device.create_command_encoder(&Default::default());
    let buf = copy_to_buffer(ctx, &mut enc, t);
    ctx.queue.submit([enc.finish()]);
    map_buffer(ctx, &buf, t.width, t.height)
}

pub fn padded_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(align) * align
}

pub fn copy_to_buffer(ctx: &GpuContext, enc: &mut wgpu::CommandEncoder, t: &Target) -> wgpu::Buffer {
    let row = padded_row(t.width);
    let buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (row * t.height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &t.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(t.height) },
        },
        wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
    );
    buf
}

pub fn map_buffer(ctx: &GpuContext, buf: &wgpu::Buffer, width: u32, height: u32) -> Vec<u8> {
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    ctx.wait();
    let row = padded_row(width) as usize;
    let data = slice.get_mapped_range().expect("mapped");
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height as usize {
        out.extend_from_slice(&data[y * row..y * row + width as usize * 4]);
    }
    drop(data);
    buf.unmap();
    out
}

/// Convert premultiplied to straight alpha in place (same rounding as the output pass).
pub fn unpremultiply(px: &mut [u8]) {
    for p in px.as_chunks_mut::<4>().0 {
        let a = p[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// Reverse row order of a tightly packed RGBA8 image.
pub fn flip_rows(data: &mut [u8], width: u32, height: u32) {
    let row = width as usize * 4;
    let h = height as usize;
    for y in 0..h / 2 {
        let (a, b) = data.split_at_mut((h - 1 - y) * row);
        a[y * row..y * row + row].swap_with_slice(&mut b[..row]);
    }
}
