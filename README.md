# phthora

Real-time GPU effect mixer for images, video and GIFs.

Load a picture or a clip, stack effects, animate their parameters (by curves, by the
music or by a MIDI controller) and export the result as PNG, MP4, WebM (with
transparency) or GIF. Everything renders on the GPU
through [wgpu](https://wgpu.rs), so the preview runs at full frame rate and the
export looks exactly like the preview.

## Features

- **Effect stack**: 63 effects in eleven groups, applied top to bottom. Each effect can be
  toggled, locked, duplicated, renamed, reset and reordered by drag and drop.
- **Chaos**: one click builds a random chain with random settings (locked effects stay).
  Shuffle reorders the stack.
- **Undo**: every edit of the stack can be undone and redone (50 steps).
- **Modulators**: every numeric parameter can be animated with linear, sine, bounce,
  step, noise, random, constant or keyframe curves, by an audio band or by a MIDI
  controller. Speed, range, chance and a time window are adjustable. Modulators are
  part of the preset and render identically in the export.
- **Audio**: the input's own sound, a separate file or a microphone drives audio
  modulators and the audio visualizer. Bands for all, low, mid and high frequencies
  plus up to four custom bands, with gain, smoothing and noise reduction. The preview
  plays the sound (mutable); file and input audio are analysed ahead of time, so the
  export reacts exactly like the preview.
- **MIDI**: all connected MIDI inputs are used. Controllers and notes map to a
  parameter with Learn; the export holds the values set when it starts.
- **Input playback**: trim the input, change its speed (0.1× to 4×) and play it
  forwards, backwards or ping-pong.
- **Image and video mode**: in image mode a seed replaces time, so every still can be
  varied. In video mode effect loops fit evenly into the output length, optionally
  synced to a tempo (BPM, with tap tempo).
- **Presets**: saved as small JSON files (`*.phthora.json`), including all seeds.
  Step through them, pick one at random or load them from the command line.
- **Export**: PNG, MP4 (H.264, four quality levels, hardware encoding through VAAPI
  when available), WebM (VP9 with alpha), GIF. Audio can be taken from the input or
  from a separate file. Progress and cancel.
- **Preview equals export**: the preview renders each output frame once, through the
  same pipeline as the export, so time-based effects look the same in both. A preset
  with fixed seeds on the same input renders the same frames every time.
- **Command line**: render without opening a window, for scripts and batch jobs.

### Effects

| Group | Effects |
|---|---|
| Reframing | Transform, Transform 3D |
| Displacement and Glitch | Luma Mesh, Bulge, Slices, Stretch, Jitter, Melt, Wobble, Wave, Shake, Bad TV, Soft Glitch, Hard Glitch, Smear, Strobe, Light Streak, Pixel Sort |
| Time-Based | Data Mosh, Decimate, Optical Flow, Feedback, Watercolor |
| Repetition and Symmetry | Mirror, Tile, Kaleido, Splitter |
| Tone and Contrast | Posterize, Solarize, Edges, Bleach, Sharpen |
| Dots and Pixels | Pixelate, Dot Matrix, Polar, 8-Bit, Half Tone, LinoCut, Dither, ASCII |
| Color | Color Correction, Hue Cycle, DuoTone, Rainbow, InstaColor |
| Optics | RGB Shift, Vignette, Tilt Shift, Barrel Blur, Glow, Blur |
| Screen and Film | Super8, VHS, CRT, ScanLines, Grain |
| Content Layers | Media, Caption, Color Gradient, Audio Visualizer |
| Masking | Mask, Mask Blocks, ChromaKey |

Mask effects cut out the effect directly above them.

## Requirements

- A GPU with Vulkan, Metal or DirectX 12 support
- `ffmpeg` and `ffprobe` in `PATH` (video input and all exports)
- Rust 1.95 or newer to build

Developed and tested on Linux (Wayland, AMD RADV). Other platforms supported by
wgpu and eframe should work but are untested.

## Build and run

```sh
cargo build --release
./target/release/phthora                 # empty editor
./target/release/phthora photo.jpg       # open a file directly
```

Files can also be dropped onto the window. Dropping a `*.phthora.json` file loads it
as a preset.

## Keyboard shortcuts

| Key | Action |
|---|---|
| Space | Play / pause |
| Ctrl+O | Open file |
| Ctrl+S | Export |
| Ctrl+M | Chaos |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |
| Ctrl+E | Shuffle effect order |
| Ctrl+D | Disable all effects |
| Ctrl+← / Ctrl+→ | Previous / next preset |
| Ctrl+R | Random preset |
| Ctrl+U | Update current preset |
| Ctrl+1 … Ctrl+0 | Load preset 1 … 10 |

Double-click a slider to reset it, double-click an effect name to rename it.

## Command line

```sh
phthora render [INPUT] [options]

  --preset FILE        effect chain (.phthora.json)
  --effect ID,...      add effects with default settings (see --list)
  --out FILE           output file (format from extension: mp4, webm, gif, png)
  --duration SEC       output length (default: one pass through the input, 4 s for stills)
  --fps N              frame rate (default: input rate or 30)
  --size WxH           output size, even numbers up to 4096 (default: input size)
  --quality Q          ultra | high | medium | low
  --bpm N              sync effect loops to a tempo
  --background COLOR   #rrggbb, #rrggbbaa or transparent (default: black)
  --flip               mirror the input horizontally
  --trim START END     use only this part of the input (seconds; END may be 'end')
  --speed X            input playback speed, 0.1 to 4
  --reverse            play the input backwards
  --pingpong           play the input forwards, then backwards
  --audio FILE|none    audio track (default: the input's own audio)
  --audio-start SEC    skip the start of the audio track
  --audio-gain G       level of audio modulators, 0 to 5 (default 1)
  --audio-smoothing S  smoothing of audio modulators, 0 to 1 (default 0.5)
  --no-reduce-noise    do not push quiet audio levels down
  --backend B          auto | software | vaapi (MP4 encoder)
  --list               list effect ids

The input's own audio is kept at normal speed and direction (trimmed with the input).
MIDI modulators keep their slider value on the command line.
```

Examples:

```sh
phthora render --list
phthora render clip.mp4 --preset mine.phthora.json --out clip-fx.mp4
phthora render photo.jpg --effect glow,vhs --out photo-fx.png
phthora render --effect color_gradient,kaleido --size 1080x1080 --duration 8 --out loop.gif
phthora render clip.mp4 --trim 2 6 --pingpong --preset mine.phthora.json --out bounce.mp4
phthora render --effect audio_visualizer --audio song.mp3 --size 1280x720 --out bars.mp4
```

## Files

| What | Where (Linux) |
|---|---|
| Presets | `$XDG_DATA_HOME/phthora/presets` (default `~/.local/share/phthora/presets`) |
| Settings | `$XDG_CONFIG_HOME/phthora/settings.json` |
| Assets | `assets/` next to the binary, or `PHTHORA_ASSETS` |

## Development

```sh
cargo test --workspace
```

| Crate | Purpose |
|---|---|
| `crates/core` | effect catalog, parameters, chain, timing, modulators, presets |
| `crates/gpu` | wgpu engine, shader effects, multi-pass and history effects, text, assets |
| `crates/media` | probing, decoding (background clip reader) and encoding through ffmpeg |
| `crates/audio` | spectrum analysis, band levels, analysed tracks, playback, microphone, MIDI |
| `crates/app` | the editor (eframe/egui) and the command line |

Shaders live in `shaders/` as WGSL, one file per pass.

## Not yet included

Webcam input, the drawing effects (push and mask painting) and effect groups are
planned for a later version.

## License

MIT OR Apache-2.0. Fonts, lookup tables and shader code from other authors keep
their own licenses, listed in [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES).
