//! compare_ref SIGNAL.wav REF.json: compare the analyzer with reference byte frequency data.
use phthora_audio::{Analyzer, HOP};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let wav = std::fs::read(&a[0]).unwrap();
    let pcm: Vec<f32> = wav[44..].as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let r = std::fs::read_to_string(&a[1]).unwrap();
    // minimal parse of {"samples":N,"frames":[[...],...]}
    let fr = &r[r.find("\"frames\":").unwrap() + 9..];
    let frames: Vec<Vec<u8>> = fr.trim_matches(|c| c == '[' || c == ']' || c == '}').split("],[").map(|f| f.split(',').map(|v| v.trim().parse().unwrap()).collect()).collect();
    let mut an = Analyzer::new();
    let only: Option<(usize, usize)> = a.get(2).map(|s| { let (x, y) = s.split_once('-').unwrap(); (x.parse().unwrap(), y.parse().unwrap()) });
    for shift in (-512i64..=512).step_by(128) {
        let (mut diff, mut maxd, mut n, mut exact) = (0u64, 0u8, 0u64, 0u64);
        for (k, f) in frames.iter().enumerate() {
            if let Some((x, y)) = only && (k < x || k >= y) { continue; }
            let end = (k as i64 + 1) * HOP as i64 + shift;
            if end <= 0 || end as usize > pcm.len() {
                continue;
            }
            let d = an.analyze(&pcm[..end as usize]);
            for (x, y) in d.iter().zip(f) {
                let e = x.abs_diff(*y);
                diff += e as u64;
                maxd = maxd.max(e);
                exact += (e == 0) as u64;
                n += 1;
            }
        }
        println!("shift {shift:+}: mean abs diff {:.4}, max {maxd}, exact {:.2}%", diff as f64 / n as f64, exact as f64 * 100.0 / n as f64);
    }
}
