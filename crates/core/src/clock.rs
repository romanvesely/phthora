//! Timing: export progress, loop progress, BPM.

/// Longest effect loop in seconds. An integer number of loops fits into the output duration.
pub const MAX_LOOP_DURATION: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OutputMode {
    Image,
    #[default]
    Video,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimingSettings {
    pub mode: OutputMode,
    pub duration: f64,
    pub fps: f64,
    pub bpm: Option<f64>,
}

impl Default for TimingSettings {
    fn default() -> Self {
        Self { mode: OutputMode::Video, duration: 4.0, fps: 30.0, bpm: None }
    }
}

/// Everything time-related an effect or modulator needs for one frame.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct FrameTime {
    pub mode: OutputMode,
    /// 0..1 over the whole output.
    pub export_progress: f64,
    /// 0..1 over one effect loop.
    pub loop_progress: f64,
    /// seconds since output start
    pub export_time: f64,
    pub export_duration: f64,
    /// frame index since output start (counts during preview too)
    pub frame: u64,
}

impl TimingSettings {
    pub fn frame_count(&self) -> u64 {
        ((self.fps * self.duration).round() as u64).max(1)
    }

    pub fn loop_progress(&self, export_progress: f64) -> f64 {
        if let Some(bpm) = self.bpm.filter(|b| *b > 0.0) {
            let bar = 60.0 / bpm * 4.0;
            let t = export_progress * self.duration;
            return (t % bar) / bar;
        }
        let loops = (self.duration / MAX_LOOP_DURATION).ceil().max(1.0);
        (export_progress * loops) % 1.0
    }

    /// Time info for output frame `frame` (deterministic, used by export).
    pub fn at_frame(&self, frame: u64) -> FrameTime {
        let p = frame as f64 / self.frame_count() as f64;
        self.at_progress(p, frame)
    }

    /// Time info for a playback position in seconds (preview). Wraps at duration.
    pub fn at_seconds(&self, t: f64) -> FrameTime {
        let d = self.duration.max(1e-6);
        let t = t.rem_euclid(d);
        let p = t / d;
        self.at_progress(p, (t * self.fps) as u64)
    }

    fn at_progress(&self, p: f64, frame: u64) -> FrameTime {
        FrameTime {
            mode: self.mode,
            export_progress: p,
            loop_progress: self.loop_progress(p),
            export_time: p * self.duration,
            export_duration: self.duration,
            frame,
        }
    }
}

/// Tap tempo: feed tap timestamps in seconds, get BPM after the second tap.
#[derive(Default, Debug)]
pub struct BpmTap {
    first: f64,
    prev: f64,
    count: u32,
}

impl BpmTap {
    pub fn tap(&mut self, now: f64) -> Option<f64> {
        if now - self.prev > 2.0 {
            self.count = 0;
        }
        self.prev = now;
        if self.count == 0 {
            self.first = now;
            self.count = 1;
            None
        } else {
            let bpm = 60.0 * self.count as f64 / (now - self.first);
            self.count += 1;
            Some((bpm * 100.0).round() / 100.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loops_fit_duration() {
        let t = TimingSettings { duration: 10.0, ..Default::default() };
        // 10 s -> 3 loops of 3.33 s
        assert!((t.loop_progress(1.0 / 3.0) - 0.0).abs() < 1e-9 || (t.loop_progress(1.0 / 3.0) - 1.0).abs() < 1e-9);
        assert!((t.loop_progress(0.5) - 0.5).abs() < 1e-9);
        assert!(t.loop_progress(0.999_999) > 0.99);
    }
    #[test]
    fn bpm_bar() {
        let t = TimingSettings { duration: 8.0, bpm: Some(120.0), ..Default::default() };
        // one bar at 120 bpm = 2 s
        assert!((t.loop_progress(0.125) - 0.5).abs() < 1e-9);
    }
    #[test]
    fn frames() {
        let t = TimingSettings { duration: 2.0, fps: 30.0, ..Default::default() };
        assert_eq!(t.frame_count(), 60);
        assert!((t.at_frame(30).export_progress - 0.5).abs() < 1e-12);
        let mut tap = BpmTap::default();
        assert_eq!(tap.tap(10.0), None);
        assert_eq!(tap.tap(10.5), Some(120.0));
    }
}
