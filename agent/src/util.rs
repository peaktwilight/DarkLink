pub fn random_jitter(base: u64, jitter: u64) -> u64 {
    base + rand::random_range(0..=jitter)
}