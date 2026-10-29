//! Frame pipeline shared by the preview and the export: which input frame belongs to
//! output frame n, and when the preview has to render again.

use phthora_core::{ClipTiming, OutputMode, Playback, ResolvedEffect, TimingSettings};
use phthora_gpu::Engine;
use phthora_media::{ClipReader, Frame, MediaInfo};
use std::path::Path;
use std::sync::Arc;

/// Output frames to read ahead.
const AHEAD: u64 = 12;

/// Size input video is decoded at for an output size (aspect kept, longest side = output's).
pub fn decode_size(info: &MediaInfo, out: (u32, u32)) -> (u32, u32) {
    phthora_media::fit_size(info.width, info.height, out.0.max(out.1))
}

/// The input video with trim, speed and direction.
pub struct InputVideo {
    pub reader: ClipReader,
    pub playback: Playback,
}

impl InputVideo {
    pub fn open(path: &Path, info: &MediaInfo, out: (u32, u32), playback: Playback) -> Self {
        let (w, h) = decode_size(info, out);
        Self { reader: ClipReader::open(path, info.clone(), w, h), playback }
    }

    pub fn clip(&self) -> ClipTiming {
        ClipTiming { fps: self.reader.fps(), frames: self.reader.frame_count() }
    }

    /// Input frame index for output frame `n`.
    pub fn index(&self, n: u64, out_fps: f64) -> u64 {
        self.playback.source_index(self.clip(), n as f64 / out_fps)
    }

    /// Input frames for the output frames after `n` (wrapping at `total`), in order.
    fn ahead(&self, n: u64, out_fps: f64, total: u64) -> Vec<u64> {
        let mut v: Vec<u64> = Vec::new();
        for k in 1..=AHEAD {
            let i = self.index((n + k) % total.max(1), out_fps);
            if v.last() != Some(&i) {
                v.push(i);
            }
        }
        v
    }

    /// Input frame for output frame `n`. Without `blocking` it returns `None` until the
    /// frame is decoded (and asks for it and the following ones).
    pub fn frame(&self, n: u64, out_fps: f64, total: u64, blocking: bool) -> Option<(u64, Arc<Frame>)> {
        let idx = self.index(n, out_fps);
        let ahead = self.ahead(n, out_fps, total);
        if blocking {
            return self.reader.get_blocking(idx, &ahead).map(|f| (idx, f));
        }
        let mut want = vec![idx];
        want.extend(ahead);
        self.reader.want(&want);
        self.reader.get(idx).map(|f| (idx, f))
    }
}

/// What a rendered preview frame depended on.
#[derive(PartialEq)]
struct Rendered {
    frame: u64,
    source: Option<u64>,
    effects: Vec<ResolvedEffect>,
    look: ([u32; 4], bool, (u32, u32)),
    /// audio spectrum, only while an effect draws it
    spectrum: Vec<u32>,
    /// versions of the live drawing canvases
    paint: Vec<(u64, u64)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Rendered,
    Unchanged,
    /// the input frame is still being decoded
    Waiting,
}

/// Decides when the preview renders: once per new output frame, or when something that
/// affects the image changed. History effects (feedback, data mosh, pixel sort) therefore
/// advance once per frame, exactly like in the export.
#[derive(Default)]
pub struct Preview {
    last: Option<Rendered>,
}

impl Preview {
    /// Force the next step to render (new input, size change).
    pub fn invalidate(&mut self) {
        self.last = None;
    }

    /// Render output frame `n` if needed. `effects` are resolved for `timing.at_frame(n)`.
    pub fn step(&mut self, engine: &mut Engine, video: Option<&InputVideo>, n: u64, timing: &TimingSettings, effects: Vec<ResolvedEffect>) -> Step {
        let (fps, ft) = (timing.fps, timing.at_frame(n));
        let total = if timing.mode == OutputMode::Image { 1 } else { timing.frame_count() };
        let mut source = None;
        if let Some(v) = video {
            let idx = v.index(n, fps);
            let uploaded = self.last.as_ref().and_then(|l| l.source);
            if uploaded != Some(idx) {
                let Some((idx, f)) = v.frame(n, fps, total, false) else { return Step::Waiting };
                engine.set_source_rgba(f.width, f.height, &f.data);
                source = Some(idx);
            } else {
                // keep reading ahead while the frame does not change (slow motion, pause)
                let _ = v.frame(n, fps, total, false);
                source = uploaded;
            }
        }
        let look = (engine.background.map(f32::to_bits), engine.flip_x, engine.size());
        let spectrum = if effects.iter().any(|e| e.desc.id == "audio_visualizer") { engine.audio_spectrum.iter().map(|v| v.to_bits()).collect() } else { Vec::new() };
        let mut paint: Vec<(u64, u64)> = engine.paint.iter().map(|(u, c)| (*u, c.version)).collect();
        paint.sort_unstable();
        let next = Rendered { frame: n, source, effects, look, spectrum, paint };
        if self.last.as_ref() == Some(&next) {
            return Step::Unchanged;
        }
        match (ft.mode, &self.last) {
            // a still renders from a clean state, like the single export frame
            (OutputMode::Image, _) => engine.reset_history(),
            // the export starts with empty history at frame 0: so does every preview loop
            (OutputMode::Video, Some(l)) if n < l.frame => engine.reset_history(),
            _ => {}
        }
        engine.render(&next.effects, &ft);
        self.last = Some(next);
        Step::Rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{render_frames, ExportJob, Progress};
    use phthora_audio::{AudioSettings, AudioTrack, FrameLevels};
    use phthora_core::modulator::{Band, LiveInputs, Modulator, Shape};
    use phthora_core::{Chain, Direction, TimingSettings};
    use phthora_gpu::GpuContext;
    use phthora_media::{Backend, Format, Quality};
    use std::collections::HashMap;

    #[test]
    fn preview_renders_the_same_frames_as_the_export() {
        if !phthora_media::ffmpeg_available() {
            eprintln!("skipped: ffmpeg not found");
            return;
        }
        let dir = std::env::temp_dir().join(format!("phthora-pipeline-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("in.mp4");
        let st = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=96x54:rate=12:duration=2", "-pix_fmt", "yuv420p"])
            .arg(&src)
            .status()
            .unwrap();
        assert!(st.success());
        let mut chain = Chain::default();
        for id in ["feedback", "pixel_sort", "data_mosh", "audio_visualizer", "rgb_shift"] {
            let uid = chain.add(id).unwrap();
            chain.get_mut(uid).unwrap().seed = 0.37;
        }
        let last = chain.effects.last_mut().unwrap();
        last.modulators.insert(
            "amount".into(),
            Modulator { enabled: true, shape: Shape::Keys, keyframes: vec![[0.0, 0.0], [0.5, 1.0], [1.0, 0.2]], seed: 1.0, ..Default::default() },
        );
        // an audio-driven parameter on the first effect
        let fb = chain.effects[0].desc.params.iter().find(|p| p.is_float()).unwrap().key.clone();
        chain.effects[0].modulators.insert(fb, Modulator { enabled: true, shape: Shape::Audio, band: Band::Low, seed: 2.0, ..Default::default() });
        // a MIDI-mapped parameter holds the value it had when the export started
        let dm = chain.effects[2].desc.params.iter().find(|p| p.is_float()).unwrap().key.clone();
        chain.effects[2].modulators.insert(dm, Modulator { enabled: true, shape: Shape::Midi, midi_cc: Some(21), midi_device: "knobs".into(), ..Default::default() });
        let midi: HashMap<(String, u8), f64> = HashMap::from([(("knobs".to_string(), 21), 0.7)]);
        let pcm: Vec<f32> = (0..44100 * 2).map(|i| (i as f32 * 0.012).sin() * ((i / 4000) % 3) as f32 * 0.4).collect();
        let mut track = AudioTrack::from_pcm(pcm);
        track.start = 0.3;
        track.end = Some(1.2);
        let audio_settings = AudioSettings { smoothing: 0.7, ..Default::default() };
        let playback = Playback { trim_start: 0.25, trim_end: Some(1.5), speed: 1.5, direction: Direction::PingPong };
        let timing = TimingSettings { duration: 2.5, fps: 12.0, ..Default::default() };
        let (w, h) = (96, 54);
        let job = ExportJob {
            input: Some(src.clone()),
            playback,
            effects: chain.effects.clone(),
            timing,
            width: w,
            height: h,
            background: [0.1, 0.2, 0.3, 1.0],
            flip_x: true,
            format: Format::Mp4,
            quality: Quality::Low,
            backend: Backend::Software,
            audio: None,
            audio_track: Some(track.clone()),
            audio_settings: audio_settings.clone(),
            midi: midi.clone(),
            out: dir.join("unused.mp4"),
        };
        let ctx = GpuContext::headless().expect("gpu");
        let mut exported = Vec::new();
        render_frames(&ctx, &job, &Progress::default(), |px| {
            exported.push(px);
            Ok(())
        })
        .unwrap();
        let total = timing.frame_count();
        assert_eq!(exported.len() as u64, total);

        // the preview path, as the editor drives it during playback
        let mut engine = Engine::new(&ctx, w, h);
        crate::video::install(&mut engine.assets, false);
        engine.background = job.background;
        engine.flip_x = job.flip_x;
        let info = phthora_media::probe(&src).unwrap();
        let video = InputVideo::open(&src, &info, (w, h), playback);
        let mut preview = Preview::default();
        let mut levels = FrameLevels::default();
        let mut audio_moved = false;
        for n in 0..total {
            let ft = timing.at_frame(n);
            let mut live = LiveInputs::default();
            let lv = levels.at(n, timing.fps, Some(&track), &audio_settings);
            for (k, v) in lv.bands.iter().take(4).enumerate() {
                live.audio[k] = *v;
            }
            engine.audio_spectrum.clone_from(&lv.spectrum);
            audio_moved |= live.audio[1] > 0.05;
            loop {
                let fx = chain.resolve(&ft, &live, &midi);
                match preview.step(&mut engine, Some(&video), n, &timing, fx) {
                    Step::Rendered => break,
                    Step::Waiting => std::thread::sleep(std::time::Duration::from_millis(1)),
                    Step::Unchanged => panic!("frame {n} was not rendered"),
                }
            }
            // a repaint without changes does not render again (history would advance)
            let fx = chain.resolve(&ft, &live, &midi);
            assert_eq!(preview.step(&mut engine, Some(&video), n, &timing, fx), Step::Unchanged);
            let mut px = engine.read_output();
            phthora_gpu::context::unpremultiply(&mut px);
            assert!(px == exported[n as usize], "preview frame {n} differs from the export");
        }
        assert!(audio_moved, "the audio modulator never moved");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
