//! An analysed audio track: PCM plus one spectrum snapshot every `HOP` samples, sampled by
//! output time in the preview and the export alike.

use crate::analyzer::{Analyzer, BINS, HOP, SAMPLE_RATE};
use std::path::Path;
use std::sync::Arc;

/// Snapshot k sees the samples up to one render quantum (128 samples) before the end of
/// block k: this matches an analyser read from a 512-sample script processor exactly.
pub fn snapshot_end(k: usize) -> usize {
    (k + 1) * HOP - 128
}

#[derive(Clone)]
pub struct AudioTrack {
    /// mono samples at `SAMPLE_RATE`
    pub pcm: Arc<Vec<f32>>,
    pub snapshots: Arc<Vec<[u8; BINS]>>,
    /// part of the track that is used and looped: start and end in seconds
    pub start: f64,
    pub end: Option<f64>,
}

impl AudioTrack {
    pub fn load(path: &Path) -> Result<Self, String> {
        let pcm = phthora_media::decode_audio(path, SAMPLE_RATE).map_err(|e| e.to_string())?;
        if pcm.is_empty() {
            return Err("audio track is empty".into());
        }
        Ok(Self::from_pcm(pcm))
    }

    pub fn from_pcm(pcm: Vec<f32>) -> Self {
        let mut a = Analyzer::new();
        let snapshots = Arc::new((0..pcm.len().div_ceil(HOP)).map(|k| a.analyze(&pcm[..snapshot_end(k).min(pcm.len())])).collect());
        Self { pcm: Arc::new(pcm), snapshots, start: 0.0, end: None }
    }

    pub fn duration(&self) -> f64 {
        self.pcm.len() as f64 / SAMPLE_RATE as f64
    }

    /// Length of the looped part.
    pub fn span(&self) -> f64 {
        let end = self.end.unwrap_or(self.duration()).min(self.duration());
        (end - self.start.max(0.0)).max(1.0 / SAMPLE_RATE as f64)
    }

    /// Track time for output time `t` (the used part loops, like the exported audio).
    pub fn track_time(&self, t: f64) -> f64 {
        self.start.max(0.0) + t.max(0.0).rem_euclid(self.span())
    }

    /// Spectrum at output time `t`.
    pub fn snapshot(&self, t: f64) -> &[u8; BINS] {
        let per_sec = self.snapshots.len() as f64 / self.duration();
        let i = (self.track_time(t) * per_sec).floor() as usize;
        &self.snapshots[i % self.snapshots.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_and_looping() {
        // 1 s of silence, then 1 s of a loud 1 kHz tone
        let mut pcm = vec![0.0f32; SAMPLE_RATE as usize];
        pcm.extend((0..SAMPLE_RATE).map(|i| (std::f32::consts::TAU * 1000.0 * i as f32 / SAMPLE_RATE as f32).sin()));
        let mut t = AudioTrack::from_pcm(pcm);
        assert_eq!(t.snapshots.len(), (2 * SAMPLE_RATE as usize).div_ceil(HOP));
        assert_eq!(t.snapshot(0.5).iter().map(|v| *v as u32).sum::<u32>(), 0);
        assert!(t.snapshot(1.5)[23] > 200);
        // the used part starts at 1 s and loops every 0.5 s
        t.start = 1.0;
        t.end = Some(1.5);
        assert!((t.track_time(0.7) - 1.2).abs() < 1e-9);
        assert!(t.snapshot(0.0)[23] > 200);
    }
}

/// Smoothed levels per output frame. Frame n always gets the same levels, however the
/// frames are visited: after a jump back the smoothing is replayed from frame 0.
#[derive(Default)]
pub struct FrameLevels {
    follower: crate::bands::Follower,
    last: Option<u64>,
    key: Option<(usize, String)>,
}

impl FrameLevels {
    pub fn at(&mut self, n: u64, fps: f64, track: Option<&AudioTrack>, s: &crate::bands::AudioSettings) -> &crate::bands::Levels {
        // a different track or settings invalidate the replayed state
        let key = (track.map(|t| Arc::as_ptr(&t.pcm) as usize ^ t.start.to_bits() as usize ^ t.end.map(f64::to_bits).unwrap_or(1) as usize).unwrap_or(0), format!("{s:?}{fps}"));
        if self.key.as_ref() != Some(&key) || self.last.is_none_or(|l| l > n) {
            self.follower.reset();
            self.last = None;
            self.key = Some(key);
        }
        let from = self.last.map(|l| l + 1).unwrap_or(0);
        for i in from..=n {
            let data = track.map(|t| t.snapshot(i as f64 / fps));
            self.follower.step(data, s);
        }
        self.last = Some(n);
        self.follower.levels()
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    use crate::bands::AudioSettings;

    #[test]
    fn same_levels_in_any_order() {
        let pcm: Vec<f32> = (0..SAMPLE_RATE * 2).map(|i| (i as f32 * 0.05).sin() * (i % 9000) as f32 / 9000.0).collect();
        let t = AudioTrack::from_pcm(pcm);
        let s = AudioSettings { smoothing: 0.9, ..Default::default() };
        let mut seq = FrameLevels::default();
        let expected: Vec<Vec<f32>> = (0..40).map(|n| seq.at(n, 20.0, Some(&t), &s).bands.clone()).collect();
        let mut jumpy = FrameLevels::default();
        for n in [5u64, 30, 2, 39, 39, 17, 0, 18] {
            assert_eq!(jumpy.at(n, 20.0, Some(&t), &s).bands, expected[n as usize], "frame {n}");
        }
    }
}
