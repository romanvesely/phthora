//! Drum hits in a synthetic beat: each drum is found at its own times and nowhere else.

use phthora_audio::{AudioTrack, SAMPLE_RATE};

const SR: f32 = SAMPLE_RATE as f32;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32 / (1u64 << 31) as f32) * 2.0 - 1.0
    }
}

fn add(pcm: &mut [f32], at: f64, sig: &[f32]) {
    let a = (at * SR as f64) as usize;
    for (i, s) in sig.iter().enumerate() {
        if let Some(p) = pcm.get_mut(a + i) {
            *p += s;
        }
    }
}

fn kick() -> Vec<f32> {
    // pitch sweep 180 -> 48 Hz with a short click, like a drum machine kick
    let n = (0.35 * SR) as usize;
    let mut rng = Lcg(3);
    let mut ph = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let f = 48.0 + 132.0 * (-t / 0.035).exp();
            ph += std::f32::consts::TAU * f / SR;
            let body = ph.sin() * (-t / 0.18).exp();
            let click = 0.3 * rng.next() * (-t / 0.004).exp();
            (1.5 * (body + click)).tanh()
        })
        .collect()
}

fn one_pole_hp(x: &mut [f32], fc: f32) {
    let a = (-std::f32::consts::TAU * fc / SR).exp();
    let (mut px, mut py) = (0.0f32, 0.0f32);
    for v in x.iter_mut() {
        py = a * (py + *v - px);
        px = *v;
        *v = py;
    }
}

fn one_pole_lp(x: &mut [f32], fc: f32) {
    let a = (-std::f32::consts::TAU * fc / SR).exp();
    let mut py = 0.0f32;
    for v in x.iter_mut() {
        py = (1.0 - a) * *v + a * py;
        *v = py;
    }
}

/// Noise bursts through a band pass, three quick ones and a tail.
fn clap(rng: &mut Lcg) -> Vec<f32> {
    let n = (0.3 * SR) as usize;
    let mut x: Vec<f32> = (0..n).map(|_| rng.next()).collect();
    one_pole_lp(&mut x, 2500.0);
    one_pole_hp(&mut x, 900.0);
    for (i, v) in x.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let mut e = 0.0;
        for o in [0.0, 0.011, 0.022] {
            if t >= o {
                e += (-(t - o) / 0.006).exp();
            }
        }
        if t >= 0.03 {
            e += (-(t - 0.03) / 0.09).exp();
        }
        *v *= 0.9 * e;
    }
    x
}

/// Noise and detuned square waves, high passed twice: a closed hi-hat.
fn hat(rng: &mut Lcg) -> Vec<f32> {
    let n = (0.12 * SR) as usize;
    let freqs = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
    let mut x: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let metal: f32 = freqs.iter().map(|f| (std::f32::consts::TAU * f * 1.7 * t).sin().signum()).sum::<f32>() / 6.0;
            0.6 * rng.next() + 0.6 * metal
        })
        .collect();
    one_pole_hp(&mut x, 7000.0);
    one_pole_hp(&mut x, 7000.0);
    for (i, v) in x.iter_mut().enumerate() {
        *v *= 0.5 * (-(i as f32 / SR) / 0.045).exp();
    }
    x
}

struct Beat {
    pcm: Vec<f32>,
    times: [Vec<f64>; 3],
}

/// 8 s of four-on-the-floor at 120 BPM: kick on every beat, clap on 2 and 4 (with the kick),
/// hi-hat on the off-beats, a bass note on the other sixteenths and a quiet pad.
fn beat() -> Beat {
    let mut rng = Lcg(7);
    let len: f64 = 8.0;
    let mut pcm: Vec<f32> = (0..(len * SR as f64) as usize).map(|i| 0.03 * (std::f32::consts::TAU * 330.0 * i as f32 / SR).sin()).collect();
    let mut times: [Vec<f64>; 3] = Default::default();
    let step = 0.125;
    let bass: Vec<f32> = (0..(0.11 * SR) as usize).map(|i| 0.3 * (std::f32::consts::TAU * 55.0 * i as f32 / SR).sin() * (-(i as f32 / SR) / 0.08).exp()).collect();
    for s in 0..60 {
        let t = 0.25 + s as f64 * step;
        match s % 4 {
            0 => {
                add(&mut pcm, t, &kick());
                times[0].push(t);
                if s % 8 == 4 {
                    add(&mut pcm, t, &clap(&mut rng));
                    times[1].push(t);
                }
            }
            2 => {
                add(&mut pcm, t, &hat(&mut rng));
                times[2].push(t);
                add(&mut pcm, t, &bass);
            }
            _ => add(&mut pcm, t, &bass),
        }
    }
    Beat { pcm, times }
}

#[test]
fn each_drum_at_its_times() {
    let beat = beat();
    let track = AudioTrack::from_pcm(beat.pcm);
    for d in 0..3 {
        let found: Vec<f64> = track.hits.0[d].iter().map(|h| h.time).collect();
        assert_eq!(found.len(), beat.times[d].len(), "drum {d}: found {found:?}, expected {:?}", beat.times[d]);
        for (f, t) in found.iter().zip(&beat.times[d]) {
            assert!((f - t).abs() < 0.02, "drum {d}: hit at {f}, expected {t}");
        }
    }
}

#[test]
fn modulator_pulses_follow_the_hits() {
    let beat = beat();
    let track = AudioTrack::from_pcm(beat.pcm);
    let fps = 30.0;
    let frame_of = |t: f64| (t * fps).floor() as u64;
    for d in 0..3 {
        for &t in &beat.times[d] {
            let n = frame_of(t);
            let imp = track.impulses(n, fps);
            assert!(imp[d].envelope(0.15) >= 0.6, "drum {d} at {t}: {}", imp[d].envelope(0.15));
            for (o, other) in imp.iter().enumerate() {
                // drums hit together (the clap on the kick) are not checked against each other
                if o != d && !beat.times[o].iter().any(|&u| (u - t).abs() < 0.03) {
                    // the other drums are at least 125 ms away and have faded
                    assert!(other.envelope(0.05) <= 0.2, "drum {o} at drum {d}'s hit {t}: {}", other.envelope(0.05));
                }
            }
        }
    }
    // between hits the pulse fades: 230 ms after a kick, before the next hi-hat? the hat comes
    // after 250 ms, so look at frame 7 after the kick
    let k = beat.times[0][1];
    let imp = track.impulses(frame_of(k) + 7, fps);
    assert!(imp[0].envelope(0.05) < 0.05);
}

#[test]
fn hits_at_the_end_of_a_loop_come_before_its_start() {
    let beat = beat();
    let mut track = AudioTrack::from_pcm(beat.pcm);
    // loop 1.0 .. 1.5 s: the kick at 1.25 lies inside it
    track.start = 1.0;
    track.end = Some(1.5);
    // output 0.3 s = track 1.3 s: the kick was 50 ms ago
    let imp = track.impulses(9, 30.0);
    let (age, s) = imp[0].recent.iter().copied().filter(|h| h.1 > 0.0).min_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
    assert!((age - 0.05).abs() < 0.03 && s > 0.5, "{age} {s}");
    // output 0.05 s = track 1.05: the kick was at 1.25 of the previous pass, 0.3 s ago
    let imp = track.impulses(1, 30.0);
    let age = imp[0].recent.iter().filter(|h| h.1 > 0.0).map(|h| h.0).fold(f32::INFINITY, f32::min);
    assert!((age - 0.3).abs() < 0.04, "{age}");
}

#[test]
fn sensitivity_changes_only_the_picking() {
    let beat = beat();
    let track = AudioTrack::from_pcm(beat.pcm);
    let strict = track.with_sensitivity(0.5);
    let loose = track.with_sensitivity(2.0);
    assert!(std::sync::Arc::ptr_eq(&strict.bands, &track.bands));
    for d in 0..3 {
        assert!(strict.hits.0[d].len() <= loose.hits.0[d].len());
    }
}


#[test]
fn live_finds_the_same_hits() {
    let beat = beat();
    let mut live = phthora_audio::LiveDrums::default();
    let mut found: [Vec<f64>; 3] = Default::default();
    let mut fed = 0usize;
    // 20 ms blocks, as an audio callback delivers them
    for block in beat.pcm.chunks(882) {
        fed += block.len();
        for (d, age, s) in live.feed(block, 1.0) {
            assert!(s > 0.0 && s <= 1.0);
            found[d].push(fed as f64 / SR as f64 - age as f64);
        }
    }
    for (d, (times, found)) in beat.times.iter().zip(&found).enumerate() {
        // the first second has no history for the threshold yet
        let expected: Vec<f64> = times.iter().copied().filter(|&t| t > 0.98).collect();
        let got: Vec<f64> = found.iter().copied().filter(|&t| t > 0.98).collect();
        assert_eq!(got.len(), expected.len(), "drum {d}: {got:?} vs {expected:?}");
        for (g, t) in got.iter().zip(&expected) {
            assert!((g - t).abs() < 0.02, "drum {d}: {g} vs {t}");
        }
    }
}
