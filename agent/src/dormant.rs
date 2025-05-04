use rand::RngCore;
use zeroize::Zeroize;

/// Simple XOR-based memory protector for lightweight in-memory obfuscation.
pub struct XorProtector {
    key: Vec<u8>,
}

impl XorProtector {
    /// Create a new protector with a random key of the given length.
    pub fn new(len: usize) -> Self {
        let mut key = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut key);
        Self { key }
    }

    /// XOR encrypt/decrypt the buffer in-place.
    pub fn xor(&self, data: &mut [u8]) {
        for (i, byte) in data.iter_mut().enumerate() {
            *byte ^= self.key[i % self.key.len()];
        }
    }

    /// Zeroize the key from memory.
    pub fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

/// Helper to zeroize sensitive buffers.
pub fn zeroize_vec(data: &mut Vec<u8>) {
    data.zeroize();
}