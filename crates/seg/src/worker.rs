//! Background segmentation for live previews: frames go in, the newest finished mask comes
//! out, and nobody ever waits.

use crate::{Mask, Segmenter};
use std::sync::{Arc, Condvar, Mutex};

struct Frame {
    rgba: Vec<u8>,
    w: u32,
    h: u32,
}

#[derive(Default)]
struct Slot {
    pending: Option<Frame>,
    mask: Option<Arc<Mask>>,
    serial: u64,
    stop: bool,
}

struct Shared {
    slot: Mutex<Slot>,
    cv: Condvar,
}

/// A thread that loads the downloaded model and segments the most recent submitted frame.
/// Frames submitted while it is busy replace each other, so it always works on the newest.
pub struct Worker {
    shared: Arc<Shared>,
}

impl Worker {
    pub fn spawn() -> Self {
        let shared = Arc::new(Shared { slot: Mutex::new(Slot::default()), cv: Condvar::new() });
        let s = shared.clone();
        std::thread::Builder::new().name("phthora-seg".into()).spawn(move || run(s)).expect("spawn thread");
        Worker { shared }
    }

    /// Queue a top-down RGBA8 frame, replacing one that has not been picked up yet.
    pub fn submit(&self, rgba: Vec<u8>, w: u32, h: u32) {
        self.shared.slot.lock().unwrap().pending = Some(Frame { rgba, w, h });
        self.shared.cv.notify_one();
    }

    /// The newest finished mask with a counter that grows with every new one.
    pub fn latest(&self) -> Option<(u64, Arc<Mask>)> {
        let s = self.shared.slot.lock().unwrap();
        s.mask.clone().map(|m| (s.serial, m))
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.shared.slot.lock().unwrap().stop = true;
        self.shared.cv.notify_one();
    }
}

fn run(shared: Arc<Shared>) {
    // failures are reported through `status()`
    let Ok(seg) = Segmenter::load_installed() else { return };
    loop {
        let frame = {
            let mut s = shared.slot.lock().unwrap();
            loop {
                if s.stop {
                    return;
                }
                if let Some(f) = s.pending.take() {
                    break f;
                }
                s = shared.cv.wait(s).unwrap();
            }
        };
        if let Ok(mask) = seg.run(&frame.rgba, frame.w, frame.h) {
            let mut s = shared.slot.lock().unwrap();
            s.mask = Some(Arc::new(mask));
            s.serial += 1;
        }
    }
}
