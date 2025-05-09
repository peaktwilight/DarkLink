use zeroize::Zeroize;
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{AeadCore, Aead, OsRng, KeyInit}; // AeadCore for NonceSize

/// AES-256-GCM based memory protector.
pub struct AesProtector {
    key: Key<Aes256Gcm>,
}

impl AesProtector {
    /// Create a new protector with a randomly generated AES-256 key.
    pub fn new() -> Self {
        Self { key: Aes256Gcm::generate_key(OsRng) }
    }

    /// Encrypt data in-place using AES-256-GCM.
    /// Returns a tuple of (nonce, ciphertext_and_tag).
    /// The nonce is required for decryption.
    /// The ciphertext includes the authentication tag.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), aes_gcm::Error> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext_and_tag = cipher.encrypt(nonce, plaintext)?;
        Ok((nonce_bytes.to_vec(), ciphertext_and_tag))
    }

    /// Decrypt data in-place using AES-256-GCM.
    /// Requires the nonce used during encryption.
    pub fn decrypt(&self, nonce_bytes: &[u8], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher.decrypt(nonce, ciphertext_and_tag)
    }

    /// Zeroize the key from memory.
    pub fn zeroize(&mut self) {
        self.key.as_mut_slice().zeroize();
    }
}

impl Default for AesProtector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to zeroize sensitive buffers.
pub fn zeroize_vec(data: &mut Vec<u8>) {
    data.zeroize();
}

pub struct SensitiveState {
    pub command_queue: Vec<Vec<u8>>, // Plaintext commands
    pub file_buffer: Vec<u8>,        // Plaintext file buffer
    pub config: Option<Vec<u8>>,     // Plaintext serialized config
}

impl SensitiveState {
    pub fn zeroize(&mut self) {
        self.command_queue.iter_mut().for_each(|cmd| cmd.zeroize());
        self.command_queue.clear();
        self.file_buffer.zeroize();
        self.file_buffer.clear();
        if let Some(cfg) = &mut self.config {
            cfg.zeroize();
        }
        self.config = None;
    }
}

pub struct MemoryProtector {
    protector: AesProtector,
    pub state: SensitiveState, // Holds plaintext when unencrypted
    // Internal storage for encrypted data
    encrypted_command_queue: Vec<(Vec<u8>, Vec<u8>)>, // (nonce, ciphertext)
    encrypted_file_buffer: Option<(Vec<u8>, Vec<u8>)>,
    encrypted_config: Option<(Vec<u8>, Vec<u8>)>,
    is_encrypted: bool, // Renamed from `encrypted` to avoid conflict
}

impl MemoryProtector {
    pub fn new(initial_state: SensitiveState) -> Self {
        Self {
            protector: AesProtector::new(),
            state: initial_state, // Starts with plaintext
            encrypted_command_queue: Vec::new(),
            encrypted_file_buffer: None,
            encrypted_config: None,
            is_encrypted: false, // Initially, state is plaintext and not yet "protected"
        }
    }

    pub fn protect(&mut self) {
        if !self.is_encrypted {
            // Encrypt command_queue
            self.encrypted_command_queue.clear();
            for cmd_plaintext in &self.state.command_queue {
                if let Ok((nonce, ciphertext)) = self.protector.encrypt(cmd_plaintext) {
                    self.encrypted_command_queue.push((nonce, ciphertext));
                } else {
                    // TODO: Log or handle encryption error for this command
                    // Consider not adding it or adding a placeholder indicating an error.
                }
            }
            self.state.command_queue.iter_mut().for_each(|cmd| cmd.zeroize()); 
            self.state.command_queue.clear(); // Clear plaintext store

            // Encrypt file_buffer
            if !self.state.file_buffer.is_empty() {
                if let Ok((nonce, ciphertext)) = self.protector.encrypt(&self.state.file_buffer) {
                    self.encrypted_file_buffer = Some((nonce, ciphertext));
                } else {
                    // TODO: Log or handle encryption error for file_buffer
                    self.encrypted_file_buffer = None; // Ensure no partial state
                }
                self.state.file_buffer.zeroize(); 
                self.state.file_buffer.clear(); // Clear plaintext store
            } else {
                self.encrypted_file_buffer = None;
            }

            // Encrypt config
            if let Some(config_plaintext) = &self.state.config {
                if !config_plaintext.is_empty() { // Only encrypt if not empty
                    if let Ok((nonce, ciphertext)) = self.protector.encrypt(config_plaintext) {
                        self.encrypted_config = Some((nonce, ciphertext));
                    } else {
                        // TODO: Log or handle encryption error for config
                        self.encrypted_config = None; // Ensure no partial state
                    }
                } else {
                    self.encrypted_config = None; // Config was present but empty
                }
                // Zeroize and clear the plaintext config from state
                // self.state.config.as_mut().unwrap().zeroize(); // This would panic if None, already handled by Some()
                if let Some(pt_cfg) = self.state.config.as_mut() { pt_cfg.zeroize(); }
                self.state.config = None; 
            } else {
                self.encrypted_config = None;
            }

            self.is_encrypted = true;
        }
    }

    pub fn unprotect(&mut self) {
        if self.is_encrypted {
            // Decrypt command_queue
            self.state.command_queue.clear(); // Clear before populating
            for (nonce, ciphertext) in &self.encrypted_command_queue {
                if let Ok(plaintext) = self.protector.decrypt(nonce, ciphertext) {
                    self.state.command_queue.push(plaintext);
                } else {
                    // TODO: Log or handle decryption error for this command
                    // Consider not adding it or adding a placeholder.
                }
            }
            self.encrypted_command_queue.iter_mut().for_each(|(n,c)| {n.zeroize(); c.zeroize();});
            self.encrypted_command_queue.clear();

            // Decrypt file_buffer
            self.state.file_buffer.clear(); // Clear before populating
            if let Some((nonce, ciphertext)) = &self.encrypted_file_buffer {
                if let Ok(plaintext) = self.protector.decrypt(nonce, ciphertext) {
                    self.state.file_buffer = plaintext;
                } else {
                    // TODO: Log or handle decryption error for file_buffer
                }
                // Zeroize the encrypted copy after decryption attempt
                // self.encrypted_file_buffer.as_mut().unwrap().0.zeroize();
                // self.encrypted_file_buffer.as_mut().unwrap().1.zeroize();
                // This zeroization is tricky because the Option is moved or re-assigned.
                // Best to clear the Option itself.
                if let Some(mut enc_fb) = self.encrypted_file_buffer.take() {
                    enc_fb.0.zeroize();
                    enc_fb.1.zeroize();
                }
            } else {
                 self.state.file_buffer.clear(); // Ensure it's empty if no encrypted buffer was present
            }

            // Decrypt config
            self.state.config = None; // Clear before populating
            if let Some((nonce, ciphertext)) = &self.encrypted_config {
                if let Ok(plaintext) = self.protector.decrypt(nonce, ciphertext) {
                    self.state.config = Some(plaintext);
                } else {
                    // TODO: Log or handle decryption error for config
                }
                if let Some(mut enc_cfg) = self.encrypted_config.take() {
                    enc_cfg.0.zeroize();
                    enc_cfg.1.zeroize();
                }
            }
            self.is_encrypted = false;
        }
    }

    pub fn zeroize(&mut self) {
        self.protector.zeroize();
        self.state.zeroize();
        self.encrypted_command_queue.iter_mut().for_each(|(n, c)| { n.zeroize(); c.zeroize(); });
        self.encrypted_command_queue.clear();
        if let Some((ref mut n, ref mut c)) = self.encrypted_file_buffer { n.zeroize(); c.zeroize(); }
        self.encrypted_file_buffer = None;
        if let Some((ref mut n, ref mut c)) = self.encrypted_config { n.zeroize(); c.zeroize(); }
        self.encrypted_config = None;
        self.is_encrypted = false; // Or true, to reflect it's zeroized and "protected"
    }
}