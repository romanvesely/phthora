//! CPU noise helpers used by modulators and a few effects.

/// Positive modulo.
pub fn mod_wrap(n: f64, m: f64) -> f64 {
    ((n % m) + m) % m
}

pub fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// Hash-style pseudo random in 0..1.
pub fn random1d(seed: f64) -> f64 {
    mod_wrap(seed.sin() * 43758.5453, 1.0)
}

/// Smooth 1D value noise in 0..1.
pub fn noise1d(x: f64) -> f64 {
    let i = x.floor();
    let f = mod_wrap(x, 1.0);
    let u = f * f * (3.0 - 2.0 * f);
    lerp(u, random1d(i), random1d(i + 1.0))
}

/// Classic 2D simplex noise with a fixed permutation table (deterministic). Range ~ -1..1.
pub fn simplex2(xin: f64, yin: f64) -> f64 {
    const GRAD3: [[f64; 2]; 12] = [
        [1., 1.], [-1., 1.], [1., -1.], [-1., -1.],
        [1., 0.], [-1., 0.], [1., 0.], [-1., 0.],
        [0., 1.], [0., -1.], [0., 1.], [0., -1.],
    ];
    let perm = perm();
    let f2 = 0.5 * (3f64.sqrt() - 1.0);
    let g2 = (3.0 - 3f64.sqrt()) / 6.0;
    let s = (xin + yin) * f2;
    let i = (xin + s).floor();
    let j = (yin + s).floor();
    let t = (i + j) * g2;
    let x0 = xin - (i - t);
    let y0 = yin - (j - t);
    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
    let x1 = x0 - i1 as f64 + g2;
    let y1 = y0 - j1 as f64 + g2;
    let x2 = x0 - 1.0 + 2.0 * g2;
    let y2 = y0 - 1.0 + 2.0 * g2;
    let ii = (i as i64 & 255) as usize;
    let jj = (j as i64 & 255) as usize;
    let gi0 = perm[ii + perm[jj] as usize] as usize % 12;
    let gi1 = perm[ii + i1 + perm[jj + j1] as usize] as usize % 12;
    let gi2 = perm[ii + 1 + perm[jj + 1] as usize] as usize % 12;
    let corner = |g: usize, x: f64, y: f64| {
        let t = 0.5 - x * x - y * y;
        if t < 0.0 { 0.0 } else { let t = t * t; t * t * (GRAD3[g][0] * x + GRAD3[g][1] * y) }
    };
    70.0 * (corner(gi0, x0, y0) + corner(gi1, x1, y1) + corner(gi2, x2, y2))
}

fn perm() -> &'static [u8; 512] {
    use std::sync::OnceLock;
    static P: OnceLock<[u8; 512]> = OnceLock::new();
    P.get_or_init(|| {
        // deterministic shuffle of 0..255 (xorshift)
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut s: u32 = 0x9e37_79b9;
        for i in (1..256).rev() {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            p.swap(i, (s as usize) % (i + 1));
        }
        std::array::from_fn(|i| p[i & 255])
    })
}

/// Noise that loops seamlessly as `n` goes 0..1. Returns ~ -1..1.
pub fn loop_noise1d(n: f64, scale: f64, seed: f64) -> f64 {
    let x = (n * std::f64::consts::TAU).cos() + seed;
    let y = (n * std::f64::consts::TAU).sin() + seed;
    simplex2(x * scale, y * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges() {
        for k in 0..1000 {
            let x = k as f64 * 0.137;
            let n = noise1d(x);
            assert!((0.0..=1.0).contains(&n));
            let s = simplex2(x, x * 0.7 + 3.0);
            assert!((-1.01..=1.01).contains(&s));
        }
        let a = loop_noise1d(0.0, 2.0, 5.0);
        let b = loop_noise1d(1.0, 2.0, 5.0);
        assert!((a - b).abs() < 1e-9);
    }
}
