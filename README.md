# phthora

Real-time GPU effect mixer for images, video and GIFs.

Load a picture or a clip, stack effects, animate their parameters and export the
result as PNG, MP4, WebM (with transparency) or GIF. Everything renders on the GPU
through [wgpu](https://wgpu.rs), so the preview runs at full frame rate and the
export looks exactly like the preview.

## Features

- **Effect stack**: 62 effects in ten groups, applied top to bottom. Each effect can be
  toggled, locked, duplicated, renamed, reset and reordered by drag and drop.
- **Chaos**: one click builds a random chain with random settings (locked effects stay).
  Undo goes back up to ten steps. Shuffle reorders the stack.
- **Modulators**: every numeric parameter can be animated with linear, sine, bounce,
  step, noise, random, constant or keyframe curves. Speed, range, chance and a time
  window are adjustable. Modulators are part of the preset and render identically in
  the export.
- **Image and video mode**: in image mode a seed replaces time, so every still can be
  varied. In video mode effect loops fit evenly into the output length, optionally
  synced to a tempo (BPM, with tap tempo).
- **Presets**: saved as small JSON files (`*.phthora.json`), including all seeds.
  Step through them, pick one at random or load them from the command line.
- **Export**: PNG, MP4 (H.264, four quality levels), WebM (VP9 with alpha), GIF.
  Audio can be taken from the input or from a separate file. Progress and cancel.
- **Deterministic**: the same preset on the same input gives bit-identical frames.
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
| Content Layers | Media, Caption, Color Gradient |
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
| Ctrl+Z | Undo chaos |
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

  --preset FILE      effect chain (.phthora.json)
  --effect ID,...    add effects with default settings (see --list)
  --out FILE         output file (format from extension: mp4, webm, gif, png)
  --duration SEC     output length (default: input length, 4 s for stills)
  --fps N            frame rate (default: input rate or 30)
  --size WxH         output size (default: input size, max 4096)
  --quality Q        ultra | high | medium | low
  --bpm N            sync effect loops to a tempo
  --list             list effect ids
```

Examples:

```sh
phthora render --list
phthora render clip.mp4 --preset mine.phthora.json --out clip-fx.mp4
phthora render photo.jpg --effect glow,vhs --out photo-fx.png
phthora render --effect color_gradient,kaleido --size 1080x1080 --duration 8 --out loop.gif
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
| `crates/media` | probing, decoding and encoding through ffmpeg |
| `crates/app` | the editor (eframe/egui) and the command line |

Shaders live in `shaders/` as WGSL, one file per pass.

## Not yet included

Audio-reactive modulation and the audio visualizer, MIDI control, webcam input and
the drawing effects (push and mask painting) are planned for a later version.

## License

MIT OR Apache-2.0. Fonts, lookup tables and shader code from other authors keep
their own licenses, listed in [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES).
