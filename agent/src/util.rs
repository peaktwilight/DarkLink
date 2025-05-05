use rand::Rng;

pub fn random_jitter(base: u64, jitter: u64) -> u64 {
    let mut rng = rand::thread_rng();
    base + rng.gen_range(0..=jitter)
}