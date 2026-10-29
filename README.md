# phthora

Real-time GPU effect mixer for images, video and GIFs.

Load a picture, a clip or the webcam, stack effects, animate their parameters (by curves, by the
music or by a MIDI controller) and export the result as a video, GIF, animated WebP,
APNG, image sequence or still. Everything renders on the GPU
through [wgpu](https://wgpu.rs), so the preview runs at full frame rate and the
export looks exactly like the preview.

## Features

- **Effect stack**: 67 effects in eleven categories, applied top to bottom. Each effect can be
  toggled, locked, duplicated, renamed, reset and reordered by drag and drop.
- **Groups**: effects can be put into groups (right click on an effect). A group can be
  collapsed, renamed, toggled, locked, duplicated and moved as a whole. *Isolate* renders
  the group on a transparent layer; the blend mode puts that layer over the input
  (Normal), uses it to cut out the input (Mask) or puts it behind the input (Behind).
  A mask effect right below a group cuts out the whole group.
- **Chaos**: one click builds a random chain with random settings (locked effects stay).
  Shuffle reorders the stack.
- **Undo**: every edit of the stack can be undone and redone (50 steps).
- **Modulators**: every numeric parameter can be animated with linear, sine, bounce,
  step, noise, random, constant or keyframe curves, by an audio band or by a MIDI
  controller. Speed, range, chance and a time window are adjustable. Modulators are
  part of the preset and render identically in the export. Right click on `~` copies a
  modulator and pastes it onto another parameter.
- **Audio**: the input's own sound, a separate file or a microphone drives audio
  modulators and the audio visualizer. Bands for all, low, mid and high frequencies
  plus up to four custom bands, with gain, smoothing and noise reduction. The preview
  plays the sound (mutable); file and input audio are analysed ahead of time, so the
  export reacts exactly like the preview.
- **MIDI**: all connected MIDI inputs are used. Controllers and notes map to a
  parameter with Learn; the export holds the values set when it starts.
- **Drawing**: Push Draw smears the image along mouse strokes, Mask Draw paints a mask
  (or erases it). The drawing is part of the preset.
- **Mouse camera**: with *mouse controls* on, Transform 3D and Luma Mesh orbit with the
  left button, pan with the right button and zoom with the wheel. The view is saved in
  the preset and used by the export.
- **Remove Background**: cuts people out of the picture with a small segmentation
  network (462 KB, downloaded once after asking). Playing video and the webcam use the
  latest mask, so the preview never stalls; stills, a paused video and the export
  segment every frame, so the mask always fits.
- **Webcam**: live input from any V4L2 camera (Ctrl+W), mirrored by default. Effects
  loop every four seconds of real time. Recordings (with the microphone when it is the
  audio source) open as the input, ready for the export.
- **Virtual camera**: send the preview to a v4l2loopback device and use it in video
  calls, streaming or recording software.
- **Input playback**: trim the input, change its speed (0.1× to 4×) and play it
  forwards, backwards or ping-pong.
- **Image and video mode**: in image mode a seed replaces time, so every still can be
  varied. In video mode effect loops fit evenly into the output length, optionally
  synced to a tempo (BPM, with tap tempo).
- **Preview window**: the preview in a window of its own (Ctrl+P), for a second screen
  or a projector. Optionally always on top; double click or F11 for full screen.
- **Presets**: saved as small JSON files (`*.phthora.json`), including all seeds and
  groups. Sort them into folders, step through a folder, pick one at random, revert to
  the saved state or load them from the command line.
- **Export**: video as MP4 (H.264, hardware encoding through VAAPI when available),
  WebM (VP9 with alpha), GIF, animated WebP, APNG or a PNG sequence; stills as PNG,
  JPG or WebP. Four quality levels. *Match input* takes size, length, frame rate and
  format from the input, *Lock* keeps the export settings when another input is
  loaded. Audio can be taken from the input or from a separate file, with an offset.
  Progress and cancel. GIFs use one palette from all frames with Floyd-Steinberg
  dithering and at most 50 fps; large GIFs need a lot of memory while encoding.
- **Preview equals export**: the preview renders each output frame once, through the
  same pipeline as the export, so time-based effects look the same in both. A preset
  with fixed seeds on the same input renders the same frames every time.
- **Batch export**: render a list of images and videos with the same effects into one
  folder (☰ → Batch export…, or several inputs on the command line). Size, length, frame
  rate and format follow each input or are set for all; nothing is overwritten, and a
  file that fails does not stop the others.
- **Command line**: render without opening a window, for scripts and batch jobs.

### Effects

| Group | Effects |
|---|---|
| Reframing | Transform, Transform 3D |
| Displacement and Glitch | Luma Mesh, Bulge, Slices, Stretch, Jitter, Melt, Wobble, Wave, Shake, Bad TV, Soft Glitch, Hard Glitch, Smear, Strobe, Light Streak, Pixel Sort |
| Time-Based | Data Mosh, Decimate, Optical Flow, Feedback, Watercolor, Slit Scan |
| Repetition and Symmetry | Mirror, Tile, Kaleido, Splitter |
| Tone and Contrast | Posterize, Solarize, Edges, Bleach, Sharpen |
| Dots and Pixels | Pixelate, Dot Matrix, Polar, 8-Bit, Half Tone, LinoCut, Dither, ASCII |
| Color | Color Correction, Hue Cycle, DuoTone, Rainbow, InstaColor |
| Optics | RGB Shift, Vignette, Tilt Shift, Barrel Blur, Glow, Blur, Push Draw |
| Screen and Film | Super8, VHS, CRT, ScanLines, Grain |
| Content Layers | Media, Caption, Color Gradient, Audio Visualizer |
| Masking | Mask, Mask Blocks, Mask Draw, ChromaKey, Remove Background |

Mask effects cut out the effect (or group) directly above them. Luma Mesh draws a mesh,
lines or points; Dither offers Bayer, blue noise, random and cluster dot patterns; Grain
comes as digital or film grain; ASCII takes your own characters; ChromaKey can show its
mask and key the source picture instead of its input.

## Requirements

- A GPU with Vulkan, Metal or DirectX 12 support
- `ffmpeg` and `ffprobe` in `PATH` (video input, webcam and all exports)
- `curl` for the one-time download of the Remove Background model
- Optional: the `v4l2loopback` kernel module for the virtual camera, e.g.
  `modprobe v4l2loopback exclusive_caps=1 card_label=phthora`
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
| Ctrl+W | Webcam on / off |
| Ctrl+S | Export |
| Ctrl+P | Preview window on / off |
| Ctrl+, | Settings |
| Esc | Close the topmost window |
| Ctrl+M | Chaos |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |
| Ctrl+E | Shuffle effect order |
| Ctrl+D | Disable all effects |
| Ctrl+← / Ctrl+→ | Previous / next preset |
| Ctrl+R | Random preset |
| Ctrl+U | Update current preset |
| Ctrl+1 … Ctrl+0 | Load preset 1 … 10 |

Double-click a slider to reset it, double-click an effect or group name to rename it.
The mouse wheel over a slider moves it one step. The ☰ menu loads all effects, disables
or removes them, opens batch export, the preset, output, fonts and log folders and shows
the version.
Keyframe curves: click adds a point, drag moves it (Shift snaps), double or right click
deletes it; the background shows the loudness of the audio track.

## Command line

```sh
phthora render [INPUT] [options]
phthora render INPUT... --out-dir DIR [options]

  --preset FILE        effect chain (.phthora.json)
  --effect ID,...      add effects with default settings (see --list)
  --out FILE           output file, the format follows the name:
                         .mp4 .webm .gif .apng      video
                         .webp                      animated for video input, --duration or --fps, else a still
                         .png .jpg                  one still image
                         DIR/ or name-%05d.png      PNG sequence (a folder gets <folder>-00001.png ...)
  --duration SEC       output length (default: one pass through the input, 4 s for stills)
  --fps N              frame rate (default: input rate or 30; GIF at most 50)
  --size WxH           output size, even numbers up to 4096 (default: input size)
  --quality Q          ultra | high | medium | low (MP4, WebM, WebP, JPG; GIF: 256/256/128/64 colors)
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

batch (several inputs, or --out-dir with one):
  --out-dir DIR        folder for the results, instead of --out; every input is written as
                       <name>-phthora-<date>-<time>.<ext>, no existing file is overwritten
  --format EXT         one format for all: mp4 webm gif webp apng png jpg seq (PNG sequence)
                       (default: the input's own container where possible, else mp4 / png)

phthora --version      print the version

WebM, GIF, WebP, APNG, PNG and PNG sequences keep transparency with
--background transparent. The input's own audio is kept at normal speed and direction
(trimmed with the input). MIDI modulators keep their slider value on the command line.
In a batch, size, length and frame rate come from each input unless --size, --duration
or --fps are given; failed inputs are listed at the end and the exit status is 1.
```

Examples:

```sh
phthora render --list
phthora render clip.mp4 --preset mine.phthora.json --out clip-fx.mp4
phthora render photo.jpg --effect glow,vhs --out photo-fx.png
phthora render --effect color_gradient,kaleido --size 1080x1080 --duration 8 --out loop.gif
phthora render clip.mp4 --trim 2 6 --pingpong --preset mine.phthora.json --out bounce.mp4
phthora render --effect audio_visualizer --audio song.mp3 --size 1280x720 --out bars.mp4
phthora render clip.mp4 --preset mine.phthora.json --background transparent --out frames/
phthora render *.mp4 *.jpg --preset mine.phthora.json --out-dir done/
```

## Files

| What | Where (Linux) |
|---|---|
| Presets | `$XDG_DATA_HOME/phthora/presets` (default `~/.local/share/phthora/presets`) |
| Preset folders | subfolders of the presets folder |
| Webcam recordings | `$XDG_DATA_HOME/phthora/recordings` |
| Remove Background model | `$XDG_DATA_HOME/phthora/models` |
| Your own fonts for Caption (TTF, OTF) | `$XDG_DATA_HOME/phthora/fonts` |
| Log files | `$XDG_DATA_HOME/phthora/logs` (one per day, the last five are kept) |
| Settings | `$XDG_CONFIG_HOME/phthora/settings.json` |
| Exports | the last folder used, at first `$XDG_VIDEOS_DIR` (or the home folder) |
| Assets | `assets/` next to the binary, or `PHTHORA_ASSETS` |

## Development

```sh
cargo test --workspace
```

| Crate | Purpose |
|---|---|
| `crates/core` | effect catalog, parameters, chain, timing, modulators, presets |
| `crates/gpu` | wgpu engine, shader effects, multi-pass and history effects, text, assets |
| `crates/media` | probing, decoding (background clip reader), encoding, webcam and virtual camera through ffmpeg |
| `crates/audio` | spectrum analysis, band levels, analysed tracks, playback, microphone, MIDI |
| `crates/seg` | person segmentation for Remove Background (tract-onnx), model download |
| `crates/app` | the editor (eframe/egui) and the command line |

Shaders live in `shaders/` as WGSL, one file per pass.

## License

MIT OR Apache-2.0. Fonts, lookup tables and shader code from other authors keep
their own licenses, listed in [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES).
