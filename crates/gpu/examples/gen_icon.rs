//! Draws the app icon and writes assets/icon/phthora.{png,ico,icns}.
//!
//! A dark rounded square with a phi-like sign (ring with a vertical bar) drawn three
//! times in red, green and blue, slightly shifted and mixed additively.
//!
//!     cargo run -p phthora-gpu --example gen_icon --release -- assets/icon

use tiny_skia::{BlendMode, Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

fn draw(size: u32) -> Pixmap {
    let s = size as f32;
    let mut pm = Pixmap::new(size, size).unwrap();

    // background: rounded square
    let r = s * 0.22;
    let k = 0.5523 * r;
    let mut pb = PathBuilder::new();
    pb.move_to(r, 0.0);
    pb.line_to(s - r, 0.0);
    pb.cubic_to(s - r + k, 0.0, s, r - k, s, r);
    pb.line_to(s, s - r);
    pb.cubic_to(s, s - r + k, s - r + k, s, s - r, s);
    pb.line_to(r, s);
    pb.cubic_to(r - k, s, 0.0, s - r + k, 0.0, s - r);
    pb.line_to(0.0, r);
    pb.cubic_to(0.0, r - k, r - k, 0.0, r, 0.0);
    pb.close();
    let mut bg = Paint::default();
    bg.set_color_rgba8(0x16, 0x16, 0x1a, 255);
    bg.anti_alias = true;
    pm.fill_path(&pb.finish().unwrap(), &bg, FillRule::Winding, Transform::identity(), None);

    // sign: ring plus a vertical bar that sticks out at both ends
    let stroke_w = (s * 0.11).max(2.6);
    let shift = (s * 0.032).max(1.0);
    let ring_r = s * 0.19;
    let bar_h = s * 0.13;
    let layers = [((-shift, 0.0), (1.0, 0.0, 0.0)), ((0.0, shift * 0.6), (0.0, 1.0, 0.0)), ((shift, 0.0), (0.0, 0.0, 1.0))];
    for ((dx, dy), (cr, cg, cb)) in layers {
        let (cx, cy) = (s * 0.5 + dx, s * 0.5 + dy);
        let mut p = Paint::default();
        p.set_color(Color::from_rgba(cr, cg, cb, 1.0).unwrap());
        p.blend_mode = BlendMode::Plus;
        p.anti_alias = true;
        let ring = PathBuilder::from_circle(cx, cy, ring_r).unwrap();
        let stroke = Stroke { width: stroke_w, ..Stroke::default() };
        pm.stroke_path(&ring, &p, &stroke, Transform::identity(), None);
        let bar = Rect::from_ltrb(cx - stroke_w * 0.5, cy - ring_r - bar_h, cx + stroke_w * 0.5, cy + ring_r + bar_h).unwrap();
        pm.fill_rect(bar, &p, Transform::identity(), None);
    }
    pm
}

fn png(size: u32) -> Vec<u8> {
    draw(size).encode_png().unwrap()
}

fn ico(sizes: &[u32]) -> Vec<u8> {
    let imgs: Vec<Vec<u8>> = sizes.iter().map(|&s| png(s)).collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend((sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, img) in sizes.iter().zip(&imgs) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        out.extend([dim, dim, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((img.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in imgs {
        out.extend(img);
    }
    out
}

fn icns(entries: &[(&[u8; 4], u32)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (tag, size) in entries {
        let img = png(*size);
        body.extend(tag.iter());
        body.extend((img.len() as u32 + 8).to_be_bytes());
        body.extend(img);
    }
    let mut out = b"icns".to_vec();
    out.extend((body.len() as u32 + 8).to_be_bytes());
    out.extend(body);
    out
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets/icon".into());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/phthora.png"), png(512)).unwrap();
    std::fs::write(format!("{dir}/phthora.ico"), ico(&[16, 32, 48, 64, 128, 256])).unwrap();
    let entries: [(&[u8; 4], u32); 7] =
        [(b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024)];
    std::fs::write(format!("{dir}/phthora.icns"), icns(&entries)).unwrap();
    println!("wrote {dir}/phthora.png, .ico, .icns");
}
