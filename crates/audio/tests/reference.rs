//! The analyzer against byte frequency data recorded from a Web Audio AnalyserNode
//! (fftSize 1024, smoothing 0, read every 512 samples from a script processor) for the
//! same test signal. Every 4th snapshot is stored.

use phthora_audio::track::AudioTrack;
use phthora_audio::{BINS, SAMPLE_RATE};

/// The test signal: chirp, chord, silence, noise, quiet tone (3 s).
fn signal() -> Vec<f32> {
    let sr = SAMPLE_RATE as f64;
    let mut x: u64 = 7;
    let mut lcg = || {
        x = (1103515245 * x + 12345) % 2147483648;
        x as f64 / 2147483648.0
    };
    let pi = std::f64::consts::PI;
    (0..SAMPLE_RATE as usize * 3)
        .map(|i| {
            let t = i as f64 / sr;
            let v = if t < 1.0 {
                let (f0, f1) = (40.0f64, 8000.0f64);
                let ph = 2.0 * pi * f0 * ((f1 / f0).powf(t) - 1.0) / (f1 / f0).ln();
                ph.sin() * (0.02 + 0.5 * t)
            } else if t < 1.5 {
                0.4 * (2.0 * pi * 80.0 * t).sin() + 0.2 * (2.0 * pi * 700.0 * t).sin() + 0.1 * (2.0 * pi * 2500.0 * t).sin()
            } else if t < 2.0 {
                0.0
            } else if t < 2.5 {
                (lcg() * 2.0 - 1.0) * (0.6 - (t - 2.0))
            } else {
                0.003 * (2.0 * pi * 440.0 * t).sin()
            };
            v as f32
        })
        .collect()
}

#[test]
fn matches_web_audio_analyser() {
    // stored as hex text: 512 bytes per snapshot
    let hex: String = include_str!("data/analyser_ref.hex").split_whitespace().collect();
    let reference: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
    let track = AudioTrack::from_pcm(signal());
    let (mut n, mut exact, mut maxd) = (0usize, 0usize, 0u8);
    for (j, expected) in reference.chunks(BINS).enumerate() {
        let got = &track.snapshots[j * 4];
        for (a, b) in got.iter().zip(expected) {
            let d = a.abs_diff(*b);
            maxd = maxd.max(d);
            exact += (d == 0) as usize;
            n += 1;
        }
    }
    let share = exact as f64 / n as f64;
    println!("{n} values, {:.3}% exact, max difference {maxd}", share * 100.0);
    // the signal is regenerated here, so last-bit float differences are possible
    assert!(share > 0.999 && maxd <= 2, "{:.3}% exact, max difference {maxd}", share * 100.0);
}
