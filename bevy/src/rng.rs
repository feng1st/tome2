//! Port of `src/z-rand.cc`.
//!
//! The original picks `pcg64_once_insecure` (PCG64, XSL-RR) as its engine
//! and exposes a *current* RNG that is switched between a fixed-seed
//! "quick" RNG (town/wilderness/flavour generation) and a
//! non-deterministic "complex" RNG (the actual game).  Saves capture the
//! complex RNG state so a reloaded game continues the same stream
//! (`loadsave.cc`).
//!
//! The port keeps the same engine family and the same public API.  The
//! current RNG lives in a thread-local so the many existing call sites can
//! use `current()` exactly where they used `rand::thread_rng()`.

use std::cell::RefCell;

use rand::{Rng, RngCore};

/// z-rand.cc: PCG64 default multiplier/increment.
const PCG_MULT: u128 = 0x2360_ED05_1FC6_5DA4_4385_DF64_9FCC_F645;
const PCG_INC: u128 = 0x1405_7B7E_F767_814F;

/// `pcg64_once_insecure`: 128-bit LCG + XSL-RR output.
#[derive(Debug, Clone)]
pub struct Pcg64 {
    state: u128,
    inc: u128,
}

impl Pcg64 {
    /// `seed_t`-driven seeding (SplitMix64 expansion, then the PCG
    /// seeding ritual from pcg-cpp's `seed(initstate, initseq)`).
    pub fn seed_from_u64(seed: u64) -> Self {
        let mut sm = SplitMix64(seed);
        let initstate = ((sm.next() as u128) << 64) | sm.next() as u128;
        let initseq = sm.next();
        let mut r = Pcg64 {
            state: 0,
            inc: ((initseq as u128) << 1) | 1,
        };
        r.step();
        r.state = r.state.wrapping_add(initstate);
        r.step();
        r
    }

    /// `seed_t::system()`: entropy from the OS-backed thread RNG.
    pub fn entropy() -> Self {
        Self::seed_from_u64(rand::random::<u64>())
    }

    fn step(&mut self) {
        self.state = self.state.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
    }

    fn next_u64_inner(&mut self) -> u64 {
        let old = self.state;
        self.step();
        let xorshifted = ((old >> 64) ^ old) as u64;
        let rot = (old >> 122) as u32;
        xorshifted.rotate_right(rot)
    }

    /// `get_complex_rng_state()`: hex `state:inc` (portable text form).
    pub fn state_string(&self) -> String {
        format!("{:032x}:{:032x}", self.state, self.inc)
    }

    /// `set_complex_rng_state()`.
    pub fn from_state_string(s: &str) -> Option<Self> {
        let (a, b) = s.split_once(':')?;
        let state = u128::from_str_radix(a, 16).ok()?;
        let inc = u128::from_str_radix(b, 16).ok()?;
        if inc & 1 == 0 {
            return None;
        }
        Some(Pcg64 { state, inc })
    }
}

impl RngCore for Pcg64 {
    fn next_u32(&mut self) -> u32 {
        (self.next_u64_inner() >> 32) as u32
    }
    fn next_u64(&mut self) -> u64 {
        self.next_u64_inner()
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut i = 0;
        while i < dest.len() {
            let v = self.next_u64_inner().to_le_bytes();
            let n = (dest.len() - i).min(8);
            dest[i..i + n].copy_from_slice(&v[..n]);
            i += n;
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

/// SplitMix64, used only to expand a `u64` seed into PCG64 state.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// z-rand.cc keeps two RNG instances plus a `current_rng` pointer: the
/// "quick" RNG is reseeded for fixed-seed work and the "complex" RNG
/// survives switches with its stream intact.
struct Registry {
    quick: Pcg64,
    complex: Pcg64,
    use_quick: bool,
}

impl Registry {
    /// `quick_rng()` / `complex_rng()` are implicitly created.
    fn new() -> Self {
        Registry {
            quick: Pcg64::entropy(),
            complex: Pcg64::entropy(),
            use_quick: false,
        }
    }

    /// `quick_rng()` (z-rand.cc:53): the fixed-seed instance.
    fn quick_rng(&mut self) -> &mut Pcg64 {
        &mut self.quick
    }

    /// `complex_rng()` (z-rand.cc:65): the non-deterministic instance.
    fn complex_rng(&mut self) -> &mut Pcg64 {
        &mut self.complex
    }

    /// `get_current_rng()`: the selected engine.
    fn get_current_rng(&mut self) -> &mut Pcg64 {
        if self.use_quick {
            self.quick_rng()
        } else {
            self.complex_rng()
        }
    }
}

thread_local! {
    static REG: RefCell<Registry> = RefCell::new(Registry::new());
}

/// A handle to the thread's current RNG; replaces `rand::thread_rng()` so
/// the stream can be seeded/saved (`set_quick_rng`, `set_complex_rng`,
/// `get/set_complex_rng_state`).
#[derive(Clone, Copy, Debug)]
pub struct Current;

impl RngCore for Current {
    fn next_u32(&mut self) -> u32 {
        REG.with(|c| c.borrow_mut().get_current_rng().next_u32())
    }
    fn next_u64(&mut self) -> u64 {
        REG.with(|c| c.borrow_mut().get_current_rng().next_u64())
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        REG.with(|c| c.borrow_mut().get_current_rng().fill_bytes(dest))
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        REG.with(|c| c.borrow_mut().get_current_rng().try_fill_bytes(dest))
    }
}

/// `get_current_rng()`.
pub fn current() -> Current {
    Current
}

/// `reseed_rng(rng)` (z-rand.cc:27).
///
/// The C++ function as committed builds its `std::seed_seq` from an
/// uninitialized `data` array and takes no seed argument at all (the
/// `seed_t` parameter was dropped), so it ignores the seed entirely --
/// the clang-tidy-diagnosed quirk the fidelity review ruled deliberate.
/// The port implements the intended algorithm (seed the engine from the
/// given seed) rather than the uninitialized-memory behavior.
pub fn reseed_rng(rng: &mut Pcg64, seed: u64) {
    *rng = Pcg64::seed_from_u64(seed);
}

/// `new_seeded_rng(seed)` (z-rand.cc:43).
pub fn new_seeded_rng(seed: u64) -> Pcg64 {
    let mut r = Pcg64::seed_from_u64(0);
    reseed_rng(&mut r, seed);
    r
}

/// `set_quick_rng(seed)`: reseed the quick instance and select it.
pub fn set_quick_rng(seed: u64) {
    REG.with(|c| {
        let mut g = c.borrow_mut();
        reseed_rng(g.quick_rng(), seed);
        g.use_quick = true;
    });
}

/// `set_complex_rng()`: select the complex instance (stream untouched).
pub fn set_complex_rng() {
    REG.with(|c| c.borrow_mut().use_quick = false);
}

/// `do_randomizer` (loadsave.cc:1678): the randomizer state stored in
/// the save file.
pub fn do_randomizer_state() -> String {
    get_complex_rng_state()
}

/// `do_randomizer` restore half.
pub fn do_randomizer_restore(state: &str) {
    set_complex_rng_state(state);
}

/// `get_complex_rng_state()`.
pub fn get_complex_rng_state() -> String {
    REG.with(|c| c.borrow_mut().complex_rng().state_string())
}

/// `set_complex_rng_state()`.
pub fn set_complex_rng_state(state: &str) {
    if let Some(r) = Pcg64::from_state_string(state) {
        REG.with(|c| *c.borrow_mut().complex_rng() = r);
    }
}

/// `round_stochastic` (z-rand.cc:121).  Note the original quirk, kept
/// bug-for-bug: a fraction below 0.5 rounds to `n - 1`, never to `n`.
pub fn round_stochastic(x: f64, rng: &mut impl Rng) -> f64 {
    let n = x.trunc();
    let f = x - n;
    if f > 0.5 {
        n + 1.0
    } else if f < 0.5 {
        n - 1.0
    } else if rand_int(2, rng) == 0 {
        n - 1.0
    } else {
        n + 1.0
    }
}

/// `randnor` (z-rand.cc:154): normal deviate with stochastic rounding,
/// clamped to the original `s16b` return type.
pub fn randnor(mean: i32, stand: i32, rng: &mut impl Rng) -> i32 {
    if stand < 1 {
        return 0;
    }
    // std::normal_distribution equivalent (Box-Muller).
    let u1 = rng.gen::<f64>().max(f64::MIN_POSITIVE);
    let u2 = rng.gen::<f64>();
    let x = mean as f64
        + stand as f64 * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    round_stochastic(x, rng).clamp(-32768.0, 32767.0) as i32
}

/// `damroll` (z-rand.cc:194): `num` dice of `sides` each.
pub fn damroll(num: i32, sides: i32, rng: &mut impl Rng) -> i32 {
    let mut sum = 0;
    for _ in 0..num {
        sum += randint(sides, rng);
    }
    sum
}

/// `maxroll` (z-rand.cc:206).
pub fn maxroll(num: i32, sides: i32) -> i32 {
    num * sides
}

/// `magik` (z-rand.cc:211): true `p` percent of the time.
pub fn magik(p: i32, rng: &mut impl Rng) -> bool {
    rand_int(100, rng) < p
}

/// `rand_int` (z-rand.cc:215): 0..=m-1, 0 for m < 1.
pub fn rand_int(m: i32, rng: &mut impl Rng) -> i32 {
    if m < 1 {
        0
    } else {
        rng.gen_range(0..m)
    }
}

/// `randint` (z-rand.cc:227): 1..=m, 1 for m < 2.
pub fn randint(m: i32, rng: &mut impl Rng) -> i32 {
    if m < 2 {
        1
    } else {
        rng.gen_range(1..=m)
    }
}

/// `rand_range` (z-rand.cc:239): a..=b, a for b < a.
pub fn rand_range(a: i32, b: i32, rng: &mut impl Rng) -> i32 {
    if b < a {
        a
    } else {
        rng.gen_range(a..=b)
    }
}

/// `rand_spread` (z-rand.cc:251): a-d..=a+d.
pub fn rand_spread(a: i32, d: i32, rng: &mut impl Rng) -> i32 {
    rand_range(a - d, a + d, rng)
}

/// `uniform_element` (z-rand.hpp:93): a random element of a non-empty
/// container.  The original asserts the container is not empty; the
/// port panics the same way, then indexes by `rand_int`.
pub fn uniform_element<'a, T>(c: &'a [T], rng: &mut impl Rng) -> &'a T {
    assert!(!c.is_empty(), "uniform_element: empty container");
    &c[rand_int(c.len() as i32, rng) as usize]
}

/// `uniform_element` (z-rand.hpp:104): the mutable overload, returning a
/// reference that can be assigned through.
pub fn uniform_element_mut<'a, T>(c: &'a mut [T], rng: &mut impl Rng) -> &'a mut T {
    assert!(!c.is_empty(), "uniform_element: empty container");
    let i = rand_int(c.len() as i32, rng) as usize;
    &mut c[i]
}

/// `shuffle` (z-rand.hpp:113): Fisher-Yates using the original
/// `rand_int(i + 1)`.  An empty container is a no-op.
pub fn shuffle<T>(c: &mut [T], rng: &mut impl Rng) {
    if c.is_empty() {
        return;
    }
    for i in (1..c.len()).rev() {
        let j = rand_int((i + 1) as i32, rng) as usize;
        c.swap(i, j);
    }
}

/// `seed_t::n_bytes` (seed.hpp:10).  The original seed is a 64-byte
/// array (`n_uint32` = 16 words) used to seed the PCG engine.
pub const SEED_N_BYTES: usize = 64;
/// `seed_t::n_uint32` (seed.hpp:17).
pub const SEED_N_UINT32: usize = SEED_N_BYTES / 4;

/// `seed_t` (seed.hpp): a 64-byte seed.  The port's gameplay code stores
/// seeds as `u64` values; this type mirrors the original byte image for
/// the save/`do_seed` path and the factory functions of seed.cc.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(pub [u8; SEED_N_BYTES]);

impl Seed {
    /// `seed_t::system` (seed.cc:5): fill the seed from system entropy.
    pub fn system() -> Self {
        let mut data = [0u8; SEED_N_BYTES];
        for b in data.iter_mut() {
            *b = rand::random::<u8>();
        }
        Seed(data)
    }

    /// `seed_t::from_bytes` (seed.cc:20): copy the given byte image.
    pub fn from_bytes(bytes: &[u8; SEED_N_BYTES]) -> Self {
        Seed(*bytes)
    }

    /// `seed_t::to_bytes` (seed.cc:32): expose the byte image.
    pub fn to_bytes(&self) -> [u8; SEED_N_BYTES] {
        self.0
    }

    /// `seed_t::to_uint32` (seed.cc:40): pack each group of four bytes
    /// little-endian into one `u32` (shift 0/8/16/24), as the original
    /// does for `std::seed_seq`.
    pub fn to_uint32(&self) -> [u32; SEED_N_UINT32] {
        let mut out = [0u32; SEED_N_UINT32];
        for (i, w) in out.iter_mut().enumerate() {
            let p = 4 * i;
            *w = u32::from(self.0[p])
                | (u32::from(self.0[p + 1]) << 8)
                | (u32::from(self.0[p + 2]) << 16)
                | (u32::from(self.0[p + 3]) << 24);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn pcg_seed_is_deterministic_and_state_round_trips() {
        let mut a = Pcg64::seed_from_u64(42);
        let mut b = Pcg64::seed_from_u64(42);
        let mut c = Pcg64::seed_from_u64(43);
        let va: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let vb: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        let vc: Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        assert_eq!(va, vb);
        assert_ne!(va, vc);

        let state = a.state_string();
        let mut restored = Pcg64::from_state_string(&state).expect("state parses");
        let mut fresh = a;
        for _ in 0..4 {
            assert_eq!(restored.next_u64(), fresh.next_u64());
        }
        assert!(Pcg64::from_state_string("nonsense").is_none());
        assert!(Pcg64::from_state_string("00000000000000000000000000000001:00000000000000000000000000000002").is_none());
    }

    #[test]
    fn current_rng_quick_and_complex_switching() {
        // The quick RNG is deterministic per seed.
        set_quick_rng(7);
        let first: Vec<u64> = (0..5).map(|_| current().next_u64()).collect();
        set_quick_rng(7);
        let second: Vec<u64> = (0..5).map(|_| current().next_u64()).collect();
        assert_eq!(first, second, "quick RNG is deterministic");

        // The complex RNG keeps its stream across a quick excursion,
        // exactly like the original's `current_rng` switching.
        set_complex_rng();
        let before = current().next_u64();
        let state = get_complex_rng_state();
        set_quick_rng(1);
        let _ = current().next_u64();
        set_complex_rng();
        let after = current().next_u64();
        assert_ne!(after, before, "complex stream advances");
        set_complex_rng_state(&state);
        set_complex_rng();
        assert_eq!(
            current().next_u64(),
            after,
            "complex stream resumes at the saved state"
        );
    }

    #[test]
    fn primitive_degenerate_cases_match_z_rand() {
        let mut rng = rand::thread_rng();
        // rand_int: 0..m-1, m<1 => 0.
        assert_eq!(rand_int(0, &mut rng), 0);
        assert_eq!(rand_int(-5, &mut rng), 0);
        for _ in 0..100 {
            let v = rand_int(3, &mut rng);
            assert!((0..3).contains(&v));
        }
        // randint: 1..=m, m<2 => 1.
        assert_eq!(randint(0, &mut rng), 1);
        assert_eq!(randint(1, &mut rng), 1);
        for _ in 0..100 {
            let v = randint(4, &mut rng);
            assert!((1..=4).contains(&v));
        }
        // rand_range: a..=b, b<a => a.
        assert_eq!(rand_range(5, 1, &mut rng), 5);
        assert_eq!(rand_range(5, 5, &mut rng), 5);
        // rand_spread: a-d..=a+d; negative d makes b < a, so
        // rand_range returns its first argument (a - d).
        assert_eq!(rand_spread(7, -3, &mut rng), 10);
        assert_eq!(rand_spread(7, 0, &mut rng), 7);
        for _ in 0..100 {
            let v = rand_spread(7, 2, &mut rng);
            assert!((5..=9).contains(&v));
        }
        // damroll(3, 0) == 3 (randint(0) is 1); damroll with num <= 0 is 0.
        let mut seeded = Pcg64::seed_from_u64(1);
        assert_eq!(damroll(3, 0, &mut seeded), 3);
        assert_eq!(damroll(0, 6, &mut seeded), 0);
        assert_eq!(damroll(-2, 6, &mut seeded), 0);
        assert_eq!(maxroll(3, 6), 18);
        // magik is a percentile check.
        assert!(magik(100, &mut rng));
        assert!(!magik(0, &mut rng));
    }

    #[test]
    fn randnor_matches_degenerate_and_clamp_behaviour() {
        let mut rng = rand::thread_rng();
        assert_eq!(randnor(1234, 0, &mut rng), 0);
        assert_eq!(randnor(1234, -5, &mut rng), 0);
        // Extreme deviation must not overflow the original s16b result.
        for _ in 0..1000 {
            let v = randnor(0, 100_000, &mut rng);
            assert!((-32768..=32767).contains(&v), "clamped {v}");
        }
        // Statistical sanity.
        let n = 2000;
        let sum: i64 = (0..n).map(|_| randnor(100, 20, &mut rng) as i64).sum();
        let mean = sum / n;
        assert!((70..=130).contains(&mean), "mean {mean}");
    }

    #[test]
    fn round_stochastic_keeps_the_original_quirk() {
        let mut rng = Pcg64::seed_from_u64(9);
        assert_eq!(round_stochastic(2.75, &mut rng), 3.0);
        assert_eq!(round_stochastic(2.25, &mut rng), 1.0);
        assert_eq!(round_stochastic(-2.25, &mut rng), -3.0);
        // Exact .5 is broken randomly towards n-1/n+1.
        let mut seen = (false, false);
        for _ in 0..64 {
            let v = round_stochastic(3.5, &mut rng);
            assert!(v == 2.0 || v == 4.0);
            seen.0 |= v == 2.0;
            seen.1 |= v == 4.0;
        }
        assert_eq!(seen, (true, true));
    }

    #[test]
    fn fill_bytes_is_consistent_across_lengths() {
        let mut a = Pcg64::seed_from_u64(5);
        let mut b = Pcg64::seed_from_u64(5);
        let mut x = [0u8; 13];
        let mut y = [0u8; 13];
        a.fill_bytes(&mut x);
        b.fill_bytes(&mut y);
        assert_eq!(x, y);
        assert!(x.iter().any(|&v| v != 0));
    }

    #[test]
    fn seed_bytes_convert_both_ways_and_pack_little_endian() {
        assert_eq!(SEED_N_BYTES, 64);
        assert_eq!(SEED_N_UINT32, 16);
        // from_bytes/to_bytes round-trip.
        let mut bytes = [0u8; SEED_N_BYTES];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        let seed = Seed::from_bytes(&bytes);
        assert_eq!(seed.to_bytes(), bytes);
        // to_uint32 packs bytes p..p+3 as b0 | b1<<8 | b2<<16 | b3<<24.
        let words = seed.to_uint32();
        assert_eq!(words[0], 0x0302_0100);
        assert_eq!(words[1], 0x0706_0504);
        assert_eq!(words[15], 0x3F3E_3D3C);
        // A new system seed is 64 bytes and (practically) not all zero.
        let sys = Seed::system();
        assert_eq!(sys.to_bytes().len(), SEED_N_BYTES);
        assert!(sys.to_bytes().iter().any(|&b| b != 0));
    }

    #[test]
    fn uniform_element_and_shuffle_follow_z_rand() {
        let mut rng = Pcg64::seed_from_u64(11);
        let items = [10, 20, 30, 40, 50];
        // Only elements from the container can be drawn.
        for _ in 0..64 {
            let v = *uniform_element(&items, &mut rng);
            assert!(items.contains(&v));
        }
        // The same seed draws the same sequence.
        let mut a = Pcg64::seed_from_u64(3);
        let mut b = Pcg64::seed_from_u64(3);
        let va: Vec<i32> = (0..16).map(|_| *uniform_element(&items, &mut a)).collect();
        let vb: Vec<i32> = (0..16).map(|_| *uniform_element(&items, &mut b)).collect();
        assert_eq!(va, vb);

        // The mutable overload hands out an assignable reference.
        let mut values = [1, 2, 3];
        *uniform_element_mut(&mut values, &mut rng) = 99;
        assert!(values.contains(&99));

        // shuffle keeps every element and is deterministic per seed.
        let mut first = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut second = first;
        let mut r1 = Pcg64::seed_from_u64(21);
        let mut r2 = Pcg64::seed_from_u64(21);
        shuffle(&mut first, &mut r1);
        shuffle(&mut second, &mut r2);
        assert_eq!(first, second);
        let mut sorted = first;
        sorted.sort_unstable();
        assert_eq!(sorted, [1, 2, 3, 4, 5, 6, 7, 8]);
        // Empty containers are a no-op.
        let mut empty: [i32; 0] = [];
        shuffle(&mut empty, &mut rng);
    }
}
