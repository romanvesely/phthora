//! Blue-noise threshold map, built by the void-and-cluster method.
//!
//! Deterministic (fixed PRNG seed, integer energies), so every run and every machine
//! gets the same 64x64 map. Built once and cached.

use std::sync::OnceLock;

pub const SIZE: usize = 64;
const N: usize = SIZE * SIZE;
/// Gaussian sigma of the energy kernel, in pixels
const SIGMA: f64 = 1.5;
/// kernel radius in pixels (weights beyond it are below 1e-9 of the peak)
const RADIUS: i32 = 9;

struct Energy {
    kernel: Vec<i64>,
    e: Vec<i64>,
}

impl Energy {
    fn new() -> Self {
        let w = (2 * RADIUS + 1) as usize;
        let mut kernel = vec![0i64; w * w];
        for dy in -RADIUS..=RADIUS {
            for dx in -RADIUS..=RADIUS {
                let d2 = (dx * dx + dy * dy) as f64;
                // quantized so tie-breaking cannot depend on the platform's libm
                kernel[(dy + RADIUS) as usize * w + (dx + RADIUS) as usize] = ((-d2 / (2.0 * SIGMA * SIGMA)).exp() * 1e9).round() as i64;
            }
        }
        Self { kernel, e: vec![0; N] }
    }

    /// Add (`sign` = 1) or remove (-1) a point's kernel, wrapping around the edges.
    fn splat(&mut self, idx: usize, sign: i64) {
        let w = (2 * RADIUS + 1) as usize;
        let (x, y) = ((idx % SIZE) as i32, (idx / SIZE) as i32);
        for dy in -RADIUS..=RADIUS {
            let yy = (y + dy).rem_euclid(SIZE as i32) as usize;
            for dx in -RADIUS..=RADIUS {
                let xx = (x + dx).rem_euclid(SIZE as i32) as usize;
                self.e[yy * SIZE + xx] += sign * self.kernel[(dy + RADIUS) as usize * w + (dx + RADIUS) as usize];
            }
        }
    }

    /// Point (with `on[i] == want`) of highest energy; the lowest index wins ties.
    fn argmax(&self, on: &[bool], want: bool) -> usize {
        let mut best = usize::MAX;
        for (i, _) in on.iter().enumerate().filter(|(_, o)| **o == want) {
            if best == usize::MAX || self.e[i] > self.e[best] {
                best = i;
            }
        }
        best
    }

    /// Point (with `on[i] == want`) of lowest energy; the lowest index wins ties.
    fn argmin(&self, on: &[bool], want: bool) -> usize {
        let mut best = usize::MAX;
        for (i, _) in on.iter().enumerate().filter(|(_, o)| **o == want) {
            if best == usize::MAX || self.e[i] < self.e[best] {
                best = i;
            }
        }
        best
    }
}

/// xorshift64*, enough to scatter the initial points
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Rank (0..4095) of every pixel, row-major. The ranks are a permutation.
fn generate() -> Vec<u16> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut order: Vec<usize> = (0..N).collect();
    for i in (1..N).rev() {
        order.swap(i, (rng.next() % (i as u64 + 1)) as usize);
    }
    // initial binary pattern: ~10% random points, relaxed until no move improves it
    let ones = N / 10;
    let mut on = vec![false; N];
    let mut en = Energy::new();
    for &i in &order[..ones] {
        on[i] = true;
        en.splat(i, 1);
    }
    for _ in 0..N {
        let cluster = en.argmax(&on, true);
        on[cluster] = false;
        en.splat(cluster, -1);
        let void = en.argmin(&on, false);
        on[void] = true;
        en.splat(void, 1);
        if void == cluster {
            break;
        }
    }
    let initial = on.clone();
    let mut rank = vec![0u16; N];

    // phase 1: peel the points off the tightest cluster, highest rank first
    for r in (0..ones).rev() {
        let cluster = en.argmax(&on, true);
        on[cluster] = false;
        en.splat(cluster, -1);
        rank[cluster] = r as u16;
    }
    // phases 2 and 3: starting again from the initial pattern, fill the largest void each time
    on = initial;
    en = Energy::new();
    for (i, &o) in on.iter().enumerate() {
        if o {
            en.splat(i, 1);
        }
    }
    for r in ones..N {
        let void = en.argmin(&on, false);
        on[void] = true;
        en.splat(void, 1);
        rank[void] = r as u16;
    }
    rank
}

/// The cached map: rank of every pixel, row-major, `SIZE` x `SIZE`.
pub fn ranks() -> &'static [u16] {
    static MAP: OnceLock<Vec<u16>> = OnceLock::new();
    MAP.get_or_init(generate)
}

/// RGBA8 texels for the GPU: rank / 16 in red, (rank % 16) * 16 in green, so the
/// shader can rebuild the exact 12-bit rank.
pub fn rgba() -> Vec<u8> {
    ranks().iter().flat_map(|&r| [(r / 16) as u8, ((r % 16) * 16) as u8, 0, 255]).collect()
}
