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
        rand::rng().fill_bytes(&mut key);
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

pub struct SensitiveState {
    pub command_queue: Vec<Vec<u8>>, // Commands as bytes
    pub file_buffer: Vec<u8>,
    pub config: Option<Vec<u8>>, // Serialized config
}

impl SensitiveState {
    pub fn zeroize(&mut self) {
        self.command_queue.iter_mut().for_each(|cmd| cmd.zeroize());
        self.file_buffer.zeroize();
        if let Some(cfg) = &mut self.config {
            cfg.zeroize();
        }
    }
}

pub struct MemoryProtector {
    xor: XorProtector,
    pub state: SensitiveState,
    encrypted: bool,
}

impl MemoryProtector {
    pub fn new(state: SensitiveState) -> Self {
        let xor = XorProtector::new(32); // 256-bit key
        Self { xor, state, encrypted: false }
    }

    pub fn protect(&mut self) {
        if !self.encrypted {
            for cmd in &mut self.state.command_queue {
                self.xor.xor(cmd);
            }
            self.xor.xor(&mut self.state.file_buffer);
            if let Some(cfg) = &mut self.state.config {
                self.xor.xor(cfg);
            }
            self.encrypted = true;
        }
    }

    pub fn unprotect(&mut self) {
        if self.encrypted {
            for cmd in &mut self.state.command_queue {
                self.xor.xor(cmd);
            }
            self.xor.xor(&mut self.state.file_buffer);
            if let Some(cfg) = &mut self.state.config {
                self.xor.xor(cfg);
            }
            self.encrypted = false;
        }
    }

    pub fn zeroize(&mut self) {
        self.state.zeroize();
        self.xor.zeroize();
    }
}