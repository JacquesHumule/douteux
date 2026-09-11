//! Tiny xorshift64 PRNG seeded from wall-clock time plus a process-wide call
//! counter, so back-to-back calls within the same millisecond still diverge.
//!
//! Not cryptographically secure — slugs only need enough entropy to not be
//! trivially guessable and to keep accidental collisions unlikely.

use std::sync::atomic::{AtomicU64, Ordering};

use worker::Date;

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut hasher = DefaultHasher::new();
        Date::now().as_millis().hash(&mut hasher);
        count.hash(&mut hasher);
        Rng(hasher.finish() | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u64() % items.len() as u64) as usize]
    }

    /// A `len`-character uppercase hex string.
    pub fn hex(&mut self, len: usize) -> String {
        const HEX: &[u8] = b"0123456789ABCDEF";
        (0..len)
            .map(|_| HEX[(self.next_u64() & 0xf) as usize] as char)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    #[test]
    fn hex_has_the_requested_length_and_alphabet() {
        let s = seeded(0xDEAD_BEEF).hex(8);
        assert_eq!(s.len(), 8);
        assert!(
            s.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_lowercase())
        );
    }

    #[test]
    fn hex_is_deterministic_for_a_given_seed() {
        assert_eq!(seeded(42).hex(8), seeded(42).hex(8));
        assert_ne!(seeded(1).hex(8), seeded(2).hex(8));
    }
}
