//! Messages sent to a virtual ALSA sequencer port reach the input watcher.
#![cfg(target_os = "linux")]

use phthora_audio::MidiIn;
use std::time::{Duration, Instant};

#[test]
fn virtual_port_is_picked_up() {
    use midir::os::unix::VirtualOutput;
    let Ok(out) = midir::MidiOutput::new("vsrc") else {
        eprintln!("skipped: no MIDI sequencer");
        return;
    };
    let Ok(mut conn) = out.create_virtual("knobs") else {
        eprintln!("skipped: cannot create a virtual port");
        return;
    };
    let midi = MidiIn::start();
    let deadline = Instant::now() + Duration::from_secs(10);
    let dev = loop {
        if let Some(d) = midi.devices().into_iter().find(|d| d.contains("knobs")) {
            break d;
        }
        assert!(Instant::now() < deadline, "virtual port not found: {:?}", midi.devices());
        std::thread::sleep(Duration::from_millis(50));
    };
    // the connection may need a moment after the port shows up
    let since = midi.seq();
    loop {
        conn.send(&[0xB0, 21, 127]).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        if midi.last_since(since).is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "no message arrived");
    }
    assert_eq!(midi.last_since(since), Some((dev.clone(), 21)));
    conn.send(&[0x90, 60, 127]).unwrap();
    conn.send(&[0x80, 60, 0]).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let v = midi.values();
    assert_eq!(v.get(&(dev.clone(), 21)), Some(&1.0));
    assert_eq!(v.get(&(dev, 60)), Some(&0.0), "note off resets the note");
}
