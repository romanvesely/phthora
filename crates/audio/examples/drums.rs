//! Print the drum hits found in an audio file:
//! `cargo run -p phthora-audio --example drums -- FILE [SENSITIVITY] [--times]`

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let times = args.iter().any(|a| a == "--times");
    let mut args = args.into_iter().filter(|a| a != "--times");
    let path = args.next().expect("usage: drums FILE [SENSITIVITY] [--times]");
    let s: f32 = args.next().map(|v| v.parse().expect("sensitivity")).unwrap_or(1.0);
    let t0 = std::time::Instant::now();
    let track = phthora_audio::AudioTrack::load(std::path::Path::new(&path)).expect("load").with_sensitivity(s);
    if !times {
        println!("{:.1} s of audio analysed in {:.2} s", track.duration(), t0.elapsed().as_secs_f64());
    }
    for (d, name) in phthora_audio::onset::DRUM_NAMES.iter().enumerate() {
        let hits = &track.hits.0[d];
        if times {
            let list: Vec<String> = hits.iter().map(|h| format!("{:.4}", h.time)).collect();
            println!("{name} {}", list.join(" "));
            continue;
        }
        let list: Vec<String> = hits.iter().take(24).map(|h| format!("{:.2}({:.1})", h.time, h.strength)).collect();
        println!("{name}: {} hits, {:.1}/s\n  {}", hits.len(), hits.len() as f64 / track.duration(), list.join(" "));
    }
}
