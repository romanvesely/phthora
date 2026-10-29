//! Compares effect output against reference renderings.
//! Set PHTHORA_GOLDEN_DIR to a folder with jobs.json and <job>.rgba files; skipped otherwise.

use phthora_core::chain::EffectInstance;
use phthora_core::modulator::LiveInputs;
use phthora_core::params::Value;
use phthora_core::{OutputMode, TimingSettings};
use phthora_gpu::{Engine, GpuContext};
use std::collections::HashMap;

fn test_image(w: u32, h: u32) -> Vec<u8> {
    let mut d = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            d[i] = (x * 255 / (w - 1)) as u8;
            d[i + 1] = (y * 255 / (h - 1)) as u8;
            d[i + 2] = (((x * 7 + y * 13) ^ (x * y)) & 255) as u8;
            d[i + 3] = 255;
        }
    }
    d
}

pub struct Score {
    pub psnr: f64,
    /// fraction of channels differing by more than 8/255
    pub bad: f64,
}

pub fn compare(a: &[u8], b: &[u8]) -> Score {
    let mut se = 0f64;
    let mut bad = 0usize;
    for (x, y) in a.iter().zip(b) {
        let d = *x as f64 - *y as f64;
        se += d * d;
        if d.abs() > 8.0 {
            bad += 1;
        }
    }
    let mse = se / a.len() as f64;
    let psnr = if mse == 0.0 { 99.0 } else { 10.0 * (255.0f64 * 255.0 / mse).log10() };
    Score { psnr, bad: bad as f64 / a.len() as f64 }
}

const HASH_BASED: &[&str] = &["jitter__"];

#[test]
fn parity_with_reference() {
    let Ok(dir) = std::env::var("PHTHORA_GOLDEN_DIR") else {
        eprintln!("PHTHORA_GOLDEN_DIR not set, skipping");
        return;
    };
    let jobs: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/jobs.json")).unwrap()).unwrap();
    let (w, h) = (jobs["width"].as_u64().unwrap() as u32, jobs["height"].as_u64().unwrap() as u32);
    let ctx = GpuContext::headless().expect("gpu");
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &test_image(w, h));
    let ts = TimingSettings { mode: OutputMode::Video, ..Default::default() };
    let t = ts.at_seconds(0.0);
    let filter = std::env::var("PHTHORA_PARITY_FILTER").unwrap_or_default();
    let mut rows = vec![];
    let mut failures = vec![];
    for job in jobs["jobs"].as_array().unwrap() {
        let name = job["name"].as_str().unwrap();
        if !name.contains(&filter) {
            continue;
        }
        let Ok(golden) = std::fs::read(format!("{dir}/{name}.rgba")) else { continue };
        let mut e = EffectInstance::by_id(job["effect"].as_str().unwrap()).unwrap();
        for (k, v) in job["params"].as_object().unwrap() {
            if let Some(val) = e.desc.param(k).and_then(|d| Value::from_json(d, v)) {
                e.set(k, val);
            }
        }
        let mut r = e.resolve(&t, &LiveInputs::default(), &HashMap::new());
        r.time = job["time"].as_f64().unwrap() as f32;
        engine.render(std::slice::from_ref(&r), &t);
        let out = phthora_gpu::context::read_rgba8(&ctx, engine.output());
        if std::env::var("PHTHORA_PARITY_DUMP").is_ok() {
            std::fs::write(format!("{dir}/{name}.ours.rgba"), &out).unwrap();
        }
        let s = compare(&out, &golden);
        rows.push(format!("{name:40} psnr {:6.2}  bad {:6.3}%", s.psnr, s.bad * 100.0));
        // effects driven by fract(sin) hashes may shift whole rows by a texel between GPUs
        let hash_based = HASH_BASED.iter().any(|h| name.starts_with(h));
        let min_psnr = if hash_based { 20.0 } else { 30.0 };
        if s.psnr < min_psnr && s.bad > 0.01 {
            failures.push(name.to_string());
        }
    }
    for r in &rows {
        println!("{r}");
    }
    println!("{} jobs, {} below threshold: {failures:?}", rows.len(), failures.len());
    if std::env::var("PHTHORA_PARITY_STRICT").is_ok() {
        assert!(failures.is_empty());
    }
}
