//! MIDI input: every input port is opened (and ports plugged in later), controller and
//! note messages become values 0..1 keyed by (device, controller or note number).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use midir::{Ignore, MidiInput, MidiInputConnection};

/// Values per (device, number), as the modulators look them up.
pub type MidiValues = HashMap<(String, u8), f64>;

/// (device, number, value 0..1) for a controller change, note on (velocity) or note off (0).
pub fn parse_message(data: &[u8]) -> Option<(u8, f64)> {
    const NOTE_OFF: u8 = 8;
    const NOTE_ON: u8 = 9;
    const CONTROL: u8 = 11;
    let (&status, num) = (data.first()?, *data.get(1)?);
    let v = match status >> 4 {
        CONTROL | NOTE_ON => *data.get(2)?,
        NOTE_OFF => 0,
        _ => return None,
    };
    Some((num, v as f64 / 127.0))
}

/// Port name without the ALSA client:port address, which changes between sessions.
pub fn device_name(port: &str) -> String {
    match port.rsplit_once(' ') {
        Some((head, tail)) if tail.split(':').count() == 2 && tail.split(':').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())) => head.to_string(),
        _ => port.to_string(),
    }
}

#[derive(Default)]
struct Shared {
    values: MidiValues,
    /// counts every message, so a learn can wait for the next one
    seq: u64,
    last: Option<(String, u8)>,
    devices: Vec<String>,
}

impl Shared {
    fn receive(&mut self, device: &str, data: &[u8]) {
        if let Some((num, v)) = parse_message(data) {
            self.values.insert((device.to_string(), num), v);
            self.seq += 1;
            self.last = Some((device.to_string(), num));
        }
    }
}

pub struct MidiIn {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl MidiIn {
    /// Listen on all input ports; new ports are picked up every two seconds.
    pub fn start() -> MidiIn {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (sh, st) = (shared.clone(), stop.clone());
        let thread = std::thread::Builder::new().name("phthora-midi".into()).spawn(move || watch(sh, st)).ok();
        MidiIn { shared, stop, thread }
    }

    /// Last value of every controller and note seen so far.
    pub fn values(&self) -> MidiValues {
        self.shared.lock().unwrap().values.clone()
    }

    /// Message counter: compare with a later call to see if anything arrived.
    pub fn seq(&self) -> u64 {
        self.shared.lock().unwrap().seq
    }

    /// The controller or note of the newest message, if it is newer than `since`.
    pub fn last_since(&self, since: u64) -> Option<(String, u8)> {
        let s = self.shared.lock().unwrap();
        if s.seq > since { s.last.clone() } else { None }
    }

    /// Names of the connected input devices.
    pub fn devices(&self) -> Vec<String> {
        self.shared.lock().unwrap().devices.clone()
    }

    /// Feed a message as if it came from `device` (tests, virtual input).
    pub fn inject(&self, device: &str, data: &[u8]) {
        self.shared.lock().unwrap().receive(device, data);
    }
}

impl Drop for MidiIn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn watch(shared: Arc<Mutex<Shared>>, stop: Arc<AtomicBool>) {
    let mut open: HashMap<String, MidiInputConnection<()>> = HashMap::new();
    while !stop.load(Ordering::Relaxed) {
        if let Ok(probe) = MidiInput::new("phthora") {
            let ports: Vec<(String, String)> = probe.ports().iter().filter_map(|p| probe.port_name(p).ok().map(|n| (p.id(), n))).collect();
            open.retain(|id, _| ports.iter().any(|(i, _)| i == id));
            for (id, name) in &ports {
                // our own client shows up as a port too
                if open.contains_key(id) || name.starts_with("phthora") {
                    continue;
                }
                let Ok(mut input) = MidiInput::new("phthora") else { break };
                input.ignore(Ignore::All);
                let Some(port) = input.find_port_by_id(id) else { continue };
                let dev = device_name(name);
                let sh = shared.clone();
                if let Ok(conn) = input.connect(&port, "phthora-in", move |_, data, _| sh.lock().unwrap().receive(&dev, data), ()) {
                    open.insert(id.clone(), conn);
                }
            }
            let mut names: Vec<String> = ports.iter().filter(|(id, _)| open.contains_key(id)).map(|(_, n)| device_name(n)).collect();
            names.sort();
            shared.lock().unwrap().devices = names;
        }
        for _ in 0..20 {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages() {
        assert_eq!(parse_message(&[0xB0, 7, 127]), Some((7, 1.0)));
        assert_eq!(parse_message(&[0xB3, 7, 0]), Some((7, 0.0)));
        assert_eq!(parse_message(&[0x90, 60, 64]), Some((60, 64.0 / 127.0)));
        assert_eq!(parse_message(&[0x80, 60, 64]), Some((60, 0.0)));
        assert_eq!(parse_message(&[0xE0, 0, 64]), None, "pitch bend is ignored");
        assert_eq!(parse_message(&[0xF8]), None);
        assert_eq!(parse_message(&[0xB0, 7]), None);
    }

    #[test]
    fn names_drop_the_alsa_address() {
        assert_eq!(device_name("Scarlett 18i8 USB:Scarlett 18i8 USB MIDI 1 24:0"), "Scarlett 18i8 USB:Scarlett 18i8 USB MIDI 1");
        assert_eq!(device_name("nanoKONTROL2"), "nanoKONTROL2");
        assert_eq!(device_name("Pad 2"), "Pad 2");
    }

    #[test]
    fn learn_sees_only_newer_messages() {
        let m = MidiIn { shared: Default::default(), stop: Arc::new(AtomicBool::new(true)), thread: None };
        m.inject("dev", &[0xB0, 3, 10]);
        let s = m.seq();
        assert_eq!(m.last_since(s), None);
        m.inject("dev", &[0xB0, 21, 127]);
        assert_eq!(m.last_since(s), Some(("dev".into(), 21)));
        assert_eq!(m.values().get(&("dev".into(), 21)), Some(&1.0));
    }
}
