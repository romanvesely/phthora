//! Group routing on the GPU: isolation, blend modes and masks right below a group.

use phthora_core::chain::{Chain, GroupBlend};
use phthora_core::modulator::LiveInputs;
use phthora_core::params::Value;
use phthora_core::TimingSettings;
use phthora_gpu::{Engine, GpuContext};
use std::collections::HashMap;

mod render_all_image {
    pub fn test_image(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
                v.extend_from_slice(&[(fx * 255.0) as u8, (fy * 255.0) as u8, 128, 255]);
            }
        }
        v
    }
}
use render_all_image::test_image;

const W: u32 = 160;
const H: u32 = 90;

fn render(engine: &mut Engine, c: &Chain) -> Vec<u8> {
    let t = TimingSettings::default().at_frame(0);
    let r = c.resolve(&t, &LiveInputs::default(), &HashMap::new());
    engine.render(&r, &t);
    engine.read_output()
}

fn setup() -> (Engine, Vec<u8>) {
    let ctx = GpuContext::headless().expect("gpu");
    let mut engine = Engine::new(&ctx, W, H);
    let img = test_image(W, H);
    engine.set_source_rgba(W, H, &img);
    (engine, img)
}

fn chain(ids: &[&str]) -> Chain {
    let mut c = Chain::default();
    for id in ids {
        c.add(id).unwrap();
    }
    c
}

/// Group the effects at `range` (indices) and apply the settings.
fn group(c: &mut Chain, range: std::ops::Range<usize>, isolate: bool, blend: GroupBlend) -> u64 {
    let g = c.make_group(c.effects[range.start].uid).unwrap();
    for i in range {
        c.effects[i].group = Some(g);
    }
    let gr = c.group_mut(g).unwrap();
    gr.isolate = isolate;
    gr.blend = blend;
    g
}

fn px(v: &[u8], i: usize) -> &[u8] {
    &v[i * 4..i * 4 + 4]
}

/// Number of differing bytes (keeps failure messages short).
fn diff(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

#[test]
fn plain_group_renders_like_no_group() {
    let (mut engine, _) = setup();
    let mut c = chain(&["posterize", "pixelate", "rgb_shift"]);
    let flat = render(&mut engine, &c);
    group(&mut c, 0..2, false, GroupBlend::Normal);
    assert_eq!(diff(&render(&mut engine, &c), &flat), 0);
    // a disabled group drops its members
    let g = c.effects[0].group.unwrap();
    c.group_mut(g).unwrap().enabled = false;
    let only_last = render(&mut engine, &chain(&["rgb_shift"]));
    assert_eq!(diff(&render(&mut engine, &c), &only_last), 0);
}

#[test]
fn isolated_group_starts_transparent() {
    let (mut engine, img) = setup();
    // pixelate of nothing is nothing: laid over the input, the input stays
    let mut c = chain(&["pixelate"]);
    assert_ne!(diff(&render(&mut engine, &c), &img), 0);
    group(&mut c, 0..1, true, GroupBlend::Normal);
    assert_eq!(diff(&render(&mut engine, &c), &img), 0);
}

#[test]
fn blend_modes() {
    let (mut engine, img) = setup();
    let mut c = chain(&["caption"]);
    c.effects[0].set("text", Value::Text("XX".into()));
    c.effects[0].set("size", Value::Float(1.0));
    let flat = render(&mut engine, &c);
    assert_ne!(diff(&flat, &img), 0, "caption draws something");
    let g = group(&mut c, 0..1, true, GroupBlend::Normal);
    // isolated text over the input looks like text drawn on the input
    let over = render(&mut engine, &c);
    let off = over.iter().zip(&flat).filter(|(a, b)| a.abs_diff(**b) > 1).count();
    assert!(off * 1000 < over.len(), "{off} values differ");

    c.group_mut(g).unwrap().blend = GroupBlend::Mask;
    let masked = render(&mut engine, &c);
    let n = (W * H) as usize;
    let cut = (0..n).filter(|&i| px(&masked, i)[3] == 0).count();
    let kept = (0..n).filter(|&i| px(&masked, i) == px(&img, i)).count();
    assert!(cut > n / 10 && kept > n / 100, "cut {cut}, kept {kept}");
    assert!((0..n).all(|i| px(&masked, i)[3] == 0 || px(&masked, i)[3] <= px(&img, i)[3]));

    // behind an opaque input the group is hidden
    c.group_mut(g).unwrap().blend = GroupBlend::Behind;
    assert_eq!(diff(&render(&mut engine, &c), &img), 0);
}

#[test]
fn mask_below_a_group_cuts_the_whole_group() {
    let (mut engine, img) = setup();
    let inner = render(&mut engine, &chain(&["posterize", "pixelate"]));
    let mut c = chain(&["posterize", "pixelate", "mask_blocks"]);
    let flat = render(&mut engine, &c);
    group(&mut c, 0..2, false, GroupBlend::Normal);
    let grouped = render(&mut engine, &c);
    assert_ne!(diff(&grouped, &flat), 0);
    // every pixel is either the group's result or the image before the group
    let n = (W * H) as usize;
    let from_group = (0..n).filter(|&i| px(&grouped, i) == px(&inner, i)).count();
    let from_input = (0..n).filter(|&i| px(&grouped, i) == px(&img, i)).count();
    assert!(from_group > n / 10 && from_input > n / 10, "group {from_group}, input {from_input}");
    assert!((0..n).all(|i| px(&grouped, i) == px(&inner, i) || px(&grouped, i) == px(&img, i)));
}
