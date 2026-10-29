//! The network itself: preprocessing, inference with tract, mask in 0..255.

use crate::{set_load, Error, Load, Mask, MODEL};
use std::path::Path;
use std::sync::Arc;
use tract_onnx::prelude::*;

pub struct Segmenter {
    plan: Arc<TypedRunnableModel>,
}

impl Segmenter {
    /// Load an ONNX model with the input size and output of `MODEL`.
    pub fn load(path: &Path) -> Result<Self, Error> {
        set_load(Load::Loading);
        let r = Self::load_inner(path);
        set_load(match &r {
            Ok(_) => Load::Ready,
            Err(e) => Load::Failed(e.to_string()),
        });
        r
    }

    /// Load the downloaded model after checking its checksum.
    pub fn load_installed() -> Result<Self, Error> {
        let path = crate::model_path();
        if let Err(e) = crate::verify_file(&path, MODEL.sha256) {
            set_load(Load::Failed(e.to_string()));
            return Err(e);
        }
        Self::load(&path)
    }

    fn load_inner(path: &Path) -> Result<Self, Error> {
        let m = |e: TractError| Error::Model(format!("{e:#}"));
        let shape = [1usize, 3, MODEL.input_h, MODEL.input_w];
        // the file declares a symbolic batch size that clashes with the fixed input
        let plan = tract_onnx::onnx()
            .with_ignore_value_info(true)
            .model_for_path(path)
            .and_then(|m| m.with_input_fact(0, f32::fact(shape).into()))
            .and_then(|m| m.into_optimized())
            .and_then(|m| m.into_runnable())
            .map_err(m)?;
        let seg = Segmenter { plan };
        // a dry run proves the output has the expected shape
        seg.run(&vec![128; MODEL.input_w * MODEL.input_h * 4], MODEL.input_w as u32, MODEL.input_h as u32)?;
        Ok(seg)
    }

    /// Network input size (the mask has the same size).
    pub fn input_size() -> (u32, u32) {
        (MODEL.input_w as u32, MODEL.input_h as u32)
    }

    /// Segment a top-down RGBA8 frame (alpha is ignored; premultiplied input is read as
    /// the frame over black). The frame is resampled to the network input without keeping
    /// the aspect ratio, so the mask covers the whole frame.
    pub fn run(&self, rgba: &[u8], w: u32, h: u32) -> Result<Mask, Error> {
        let (w, h) = (w as usize, h as usize);
        if w == 0 || h == 0 || rgba.len() < w * h * 4 {
            return Err(Error::Model(format!("bad frame: {w}x{h}, {} bytes", rgba.len())));
        }
        let (iw, ih) = (MODEL.input_w, MODEL.input_h);
        let chw = preprocess(rgba, w, h, iw, ih);
        let input = Tensor::from_shape(&[1, 3, ih, iw], &chw).map_err(|e| Error::Model(e.to_string()))?;
        let out = self.plan.run(tvec!(input.into())).map_err(|e| Error::Model(format!("{e:#}")))?;
        let view = out[0].to_plain_array_view::<f32>().map_err(|e| Error::Model(e.to_string()))?;
        if view.len() != iw * ih {
            return Err(Error::Model(format!("unexpected output shape {:?}", view.shape())));
        }
        let data = view.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8).collect();
        Ok(Mask { w: iw as u32, h: ih as u32, data })
    }
}

/// Resampling weights of one output index: (source index, weight) pairs.
fn taps(src: usize, dst: usize) -> Vec<Vec<(usize, f32)>> {
    let scale = src as f64 / dst as f64;
    (0..dst)
        .map(|i| {
            if scale > 1.0 {
                // shrinking: average everything the output pixel covers
                let (lo, hi) = (i as f64 * scale, (i + 1) as f64 * scale);
                let mut t: Vec<(usize, f32)> = (lo.floor() as usize..(hi.ceil() as usize).min(src))
                    .map(|j| (j, (hi.min(j as f64 + 1.0) - lo.max(j as f64)) as f32))
                    .collect();
                let sum: f32 = t.iter().map(|x| x.1).sum();
                t.iter_mut().for_each(|x| x.1 /= sum);
                t
            } else {
                let c = (i as f64 + 0.5) * scale - 0.5;
                let a = c.floor();
                let f = (c - a) as f32;
                let clamp = |j: f64| (j.max(0.0) as usize).min(src - 1);
                vec![(clamp(a), 1.0 - f), (clamp(a + 1.0), f)]
            }
        })
        .collect()
}

/// Frame to the normalized network input (planar RGB, f32).
fn preprocess(rgba: &[u8], w: usize, h: usize, dw: usize, dh: usize) -> Vec<f32> {
    // horizontal pass: h rows of dw pixels
    let tx = taps(w, dw);
    let mut tmp = vec![0f32; h * dw * 3];
    for y in 0..h {
        let row = &rgba[y * w * 4..(y + 1) * w * 4];
        for (x, t) in tx.iter().enumerate() {
            let mut acc = [0f32; 3];
            for &(sx, wt) in t {
                for c in 0..3 {
                    acc[c] += row[sx * 4 + c] as f32 * wt;
                }
            }
            tmp[(y * dw + x) * 3..(y * dw + x) * 3 + 3].copy_from_slice(&acc);
        }
    }
    // vertical pass into planar output
    let ty = taps(h, dh);
    let mut out = vec![0f32; 3 * dw * dh];
    let m = &crate::MODEL;
    for (y, t) in ty.iter().enumerate() {
        for x in 0..dw {
            let mut acc = [0f32; 3];
            for &(sy, wt) in t {
                let p = (sy * dw + x) * 3;
                for c in 0..3 {
                    acc[c] += tmp[p + c] * wt;
                }
            }
            for c in 0..3 {
                out[c * dw * dh + y * dw + x] = (acc[c] / 255.0 - m.mean[c]) / m.std[c];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_keeps_flat_and_averages_blocks() {
        // a flat frame stays flat at any size (up and down)
        for (w, h) in [(1280, 720), (300, 200), (100, 50), (256, 144), (7, 3)] {
            let px = [200u8, 100, 50, 255].repeat(w * h);
            let out = preprocess(&px, w, h, 256, 144);
            let n = 256 * 144;
            for (c, v) in [200.0, 100.0, 50.0].iter().enumerate() {
                assert!(out[c * n..(c + 1) * n].iter().all(|o| (o - v / 255.0).abs() < 1e-4), "{w}x{h} channel {c}");
            }
        }
        // 2x2 blocks average exactly
        let (w, h) = (512, 288);
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let v = if (x + y) % 2 == 0 { 255 } else { 0 };
                px[(y * w + x) * 4..(y * w + x) * 4 + 3].fill(v);
            }
        }
        let out = preprocess(&px, w, h, 256, 144);
        assert!(out[..256 * 144].iter().all(|o| (o - 0.5).abs() < 1e-4));
        // the left half bright maps to the left half
        let mut px = vec![255u8; w * h * 4];
        for y in 0..h {
            px[y * w * 4..(y * w + w / 2) * 4].fill(0);
        }
        let out = preprocess(&px, w, h, 256, 144);
        assert!(out[0] < 0.01 && out[255] > 0.99, "left dark, right bright: {} {}", out[0], out[255]);
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(Segmenter::load(Path::new("/nonexistent/model.onnx")).is_err());
    }
}
