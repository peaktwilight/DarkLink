use once_cell::sync::Lazy;
use std::sync::Mutex;
use crate::dormant::{MemoryProtector, SensitiveState};

pub static MEMORY_PROTECTOR: Lazy<Mutex<MemoryProtector>> = Lazy::new(|| {
    Mutex::new(MemoryProtector::new(SensitiveState {
        command_queue: Vec::new(),
        file_buffer: Vec::new(),
        config: None,
    }))
});