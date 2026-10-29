//! Downloads the real model (network). Run with `cargo test -p phthora-seg -- --ignored`.

use phthora_seg::{Segmenter, Status};
use std::sync::atomic::AtomicBool;

#[test]
#[ignore = "needs network"]
fn real_download_verifies_and_loads() {
    let dir = std::env::temp_dir().join(format!("phthora-seg-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    phthora_seg::set_models_dir(Some(dir.clone()));
    assert_eq!(phthora_seg::status(), Status::Missing);
    let mut calls = 0;
    let path = phthora_seg::download(|_, _| calls += 1, &AtomicBool::new(false)).expect("download");
    assert!(calls > 0);
    assert_eq!(path, phthora_seg::model_path());
    assert!(phthora_seg::is_installed());
    assert_eq!(phthora_seg::status(), Status::Available);
    let seg = Segmenter::load_installed().expect("loads");
    let (w, h) = Segmenter::input_size();
    let mask = seg.run(&vec![90; (w * h * 4) as usize], w, h).unwrap();
    assert_eq!((mask.w, mask.h), (w, h));
    assert_eq!(phthora_seg::status(), Status::Ready);
    // a second call finds the file and does not download again
    phthora_seg::download(|_, _| {}, &AtomicBool::new(false)).expect("already installed");
    let _ = std::fs::remove_dir_all(&dir);
}
