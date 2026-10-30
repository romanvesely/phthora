//! Tempo of synthetic beats: BPM, beat phase and the start of the bar.

use phthora_audio::{AudioTrack, SAMPLE_RATE};

const SR: f32 = SAMPLE_RATE as f32;

fn kick(gain: f32) -> Vec<f32> {
    let mut ph = 0.0f32;
    (0..(0.25 * SR) as usize)
        .map(|i| {
            let t = i as f32 / SR;
            ph += std::f32::consts::TAU * (50.0 + 120.0 * (-t / 0.03).exp()) / SR;
            gain * ph.sin() * (-t / 0.12).exp()
        })
        .collect()
}

fn hat(seed: u64) -> Vec<f32> {
    let mut s = seed;
    let mut prev = 0.0f32;
    (0..(0.05 * SR) as usize)
        .map(|i| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let v = (s >> 33) as f32 / (1u64 << 31) as f32 - 1.0;
            // first difference: a crude high pass
            let out = (v - prev) * 0.15 * (-(i as f32 / SR) / 0.02).exp();
            prev = v;
            out
        })
        .collect()
}

/// Kicks on the beats from `offset`, the one starting each bar (beat `downbeat` of every four)
/// louder, hi-hats in between.
fn beat(bpm: f64, offset: f64, downbeat: usize, secs: f64) -> Vec<f32> {
    let mut pcm = vec![0.0f32; (secs * SR as f64) as usize];
    let beat = 60.0 / bpm;
    let mut add = |at: f64, sig: &[f32]| {
        let a = (at * SR as f64) as usize;
        for (i, s) in sig.iter().enumerate() {
            if let Some(p) = pcm.get_mut(a + i) {
                *p += s;
            }
        }
    };
    let mut j = 0;
    while offset + j as f64 * beat < secs {
        let t = offset + j as f64 * beat;
        add(t, &kick(if j % 4 == downbeat { 0.9 } else { 0.45 }));
        add(t + beat / 2.0, &hat(j as u64));
        j += 1;
    }
    pcm
}

#[test]
fn finds_tempo_phase_and_bar() {
    let track = AudioTrack::from_pcm(beat(128.0, 0.37, 1, 20.0));
    let t = track.tempo.expect("tempo");
    assert!((t.bpm - 128.0).abs() < 0.5, "{t:?}");
    let beat = 60.0 / 128.0;
    // on the kick at 0.37 s or a little before its attack (never after: a loop must not cut it)
    let near = |x: f64, want: f64| (-0.03..0.01).contains(&(x - want));
    assert!(near(t.first_beat, 0.37), "{t:?}");
    assert_eq!(t.downbeat, 1, "{t:?}");
    assert!(near(t.bar_start(0), 0.37 + beat));
    assert!(near(t.bar_start(2), 0.37 + 9.0 * beat));
    assert!(t.confidence > 0.3, "{t:?}");
}

#[test]
fn octaves_stay_musical() {
    for (bpm, ok) in [(64.0, [64.0, 128.0]), (174.0, [87.0, 174.0]), (140.0, [140.0, 140.0])] {
        let track = AudioTrack::from_pcm(beat(bpm, 0.1, 0, 24.0));
        let t = track.tempo.expect("tempo");
        assert!(ok.iter().any(|b| (t.bpm - b).abs() < 0.5), "{bpm}: {t:?}");
    }
}

#[test]
fn no_tempo_in_silence_or_short_tracks() {
    assert!(AudioTrack::from_pcm(vec![0.0; 44100 * 10]).tempo.is_none());
    assert!(AudioTrack::from_pcm(beat(120.0, 0.0, 0, 3.0)).tempo.is_none());
}
