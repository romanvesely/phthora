/// Fit `(w, h)` so the longer side is at most `max`, keeping the aspect ratio.
/// Both results are even and at least 2. Smaller inputs are only rounded
/// down to even values, never upscaled.
pub fn fit_size(w: u32, h: u32, max: u32) -> (u32, u32) {
    let max = (max & !1).max(2);
    let (w, h) = (w.max(1), h.max(1));
    let (mut ow, mut oh) = (w as f64, h as f64);
    let longer = w.max(h);
    if longer > max {
        let s = max as f64 / longer as f64;
        ow = (ow * s).round();
        oh = (oh * s).round();
    }
    let even = |v: f64| ((v as u32) & !1).max(2);
    (even(ow).min(max), even(oh).min(max))
}

#[cfg(test)]
mod tests {
    use super::fit_size;

    #[test]
    fn fits_and_evens() {
        assert_eq!(fit_size(8000, 4000, 4096), (4096, 2048));
        assert_eq!(fit_size(4000, 8000, 4096), (2048, 4096));
        assert_eq!(fit_size(1921, 1081, 4096), (1920, 1080));
        assert_eq!(fit_size(64, 48, 4096), (64, 48));
        assert_eq!(fit_size(1, 1, 4096), (2, 2));
        assert_eq!(fit_size(5000, 3, 4096), (4096, 2));
        let (w, h) = fit_size(4097, 4095, 4096);
        assert!(w <= 4096 && h <= 4096 && w % 2 == 0 && h % 2 == 0);
    }
}
