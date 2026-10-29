//! How output time maps to frames of the input clip: trim, speed and direction.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Forward,
    Reverse,
    /// forward, then backward, without repeating the turning frames
    PingPong,
}

impl Direction {
    pub const ALL: [Direction; 3] = [Direction::Forward, Direction::Reverse, Direction::PingPong];
    pub fn label(self) -> &'static str {
        match self {
            Direction::Forward => "forward",
            Direction::Reverse => "reverse",
            Direction::PingPong => "ping-pong",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.label() == s || (s == "pingpong" && *d == Direction::PingPong))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    /// trim start in seconds of the input
    pub trim_start: f64,
    /// trim end in seconds of the input; `None` = end of the input
    pub trim_end: Option<f64>,
    /// 0.1 ..= 4, frames are picked by index (no blending)
    pub speed: f64,
    pub direction: Direction,
}

impl Default for Playback {
    fn default() -> Self {
        Self { trim_start: 0.0, trim_end: None, speed: 1.0, direction: Direction::Forward }
    }
}

/// Source clip: frame rate and number of frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipTiming {
    pub fps: f64,
    pub frames: u64,
}

impl ClipTiming {
    pub fn duration(&self) -> f64 {
        self.frames as f64 / self.fps
    }
}

impl Playback {
    pub const MIN_SPEED: f64 = 0.1;
    pub const MAX_SPEED: f64 = 4.0;

    fn speed(&self) -> f64 {
        self.speed.clamp(Self::MIN_SPEED, Self::MAX_SPEED)
    }

    /// First and one-past-last source frame inside the trim range (at least one frame).
    pub fn frame_range(&self, clip: ClipTiming) -> (u64, u64) {
        let n = clip.frames.max(1);
        let a = ((self.trim_start.max(0.0) * clip.fps).round() as u64).min(n - 1);
        let b = match self.trim_end {
            Some(e) => ((e * clip.fps).round() as u64).clamp(a + 1, n),
            None => n,
        };
        (a, b)
    }

    /// Seconds of input inside the trim range.
    pub fn trimmed_duration(&self, clip: ClipTiming) -> f64 {
        let (a, b) = self.frame_range(clip);
        (b - a) as f64 / clip.fps
    }

    /// Length of one pass through the clip at this speed and direction, in output seconds.
    pub fn natural_duration(&self, clip: ClipTiming) -> f64 {
        let (a, b) = self.frame_range(clip);
        let len = b - a;
        let frames = if self.direction == Direction::PingPong && len > 1 { 2 * len - 2 } else { len };
        frames as f64 / clip.fps / self.speed()
    }

    /// Source frame index for output time `t` (seconds since output start). Loops.
    pub fn source_index(&self, clip: ClipTiming, t: f64) -> u64 {
        let (a, b) = self.frame_range(clip);
        let len = b - a;
        // small epsilon so t = k / fps lands on frame k despite float error
        let p = (t.max(0.0) * clip.fps * self.speed() + 1e-6).floor() as u64;
        match self.direction {
            Direction::Forward => a + p % len,
            Direction::Reverse => b - 1 - p % len,
            Direction::PingPong => {
                if len < 2 {
                    return a;
                }
                let period = 2 * len - 2;
                let q = p % period;
                if q < len { a + q } else { a + period - q }
            }
        }
    }

    /// Which way playback moves at time `t` (+1 forward, -1 backward), for read-ahead.
    pub fn step_direction(&self, clip: ClipTiming, t: f64) -> i8 {
        match self.direction {
            Direction::Forward => 1,
            Direction::Reverse => -1,
            Direction::PingPong => {
                let (a, b) = self.frame_range(clip);
                let len = b - a;
                if len < 2 {
                    return 1;
                }
                let p = (t.max(0.0) * clip.fps * self.speed() + 1e-6).floor() as u64;
                if p % (2 * len - 2) < len - 1 { 1 } else { -1 }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIP: ClipTiming = ClipTiming { fps: 10.0, frames: 5 };

    fn seq(pb: Playback, n: u64) -> Vec<u64> {
        (0..n).map(|i| pb.source_index(CLIP, i as f64 / 10.0)).collect()
    }

    #[test]
    fn directions() {
        let pb = Playback::default();
        assert_eq!(seq(pb, 7), [0, 1, 2, 3, 4, 0, 1]);
        let pb = Playback { direction: Direction::Reverse, ..Default::default() };
        assert_eq!(seq(pb, 7), [4, 3, 2, 1, 0, 4, 3]);
        let pb = Playback { direction: Direction::PingPong, ..Default::default() };
        assert_eq!(seq(pb, 10), [0, 1, 2, 3, 4, 3, 2, 1, 0, 1]);
        assert!((pb.natural_duration(CLIP) - 0.8).abs() < 1e-9);
        assert_eq!(pb.step_direction(CLIP, 0.35), 1);
        assert_eq!(pb.step_direction(CLIP, 0.45), -1);
    }

    #[test]
    fn speed_and_trim() {
        let pb = Playback { speed: 2.0, ..Default::default() };
        assert_eq!(seq(pb, 4), [0, 2, 4, 1]);
        let pb = Playback { speed: 0.5, ..Default::default() };
        assert_eq!(seq(pb, 4), [0, 0, 1, 1]);
        let pb = Playback { trim_start: 0.1, trim_end: Some(0.4), ..Default::default() };
        assert_eq!(pb.frame_range(CLIP), (1, 4));
        assert_eq!(seq(pb, 5), [1, 2, 3, 1, 2]);
        assert!((pb.trimmed_duration(CLIP) - 0.3).abs() < 1e-9);
        // trim end before start still leaves one frame
        let pb = Playback { trim_start: 0.3, trim_end: Some(0.1), ..Default::default() };
        assert_eq!(pb.frame_range(CLIP), (3, 4));
    }
}
