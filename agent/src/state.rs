use once_cell::sync::Lazy;
use std::sync::Mutex;
use crate::dormant::MemoryProtector;

//  Simplified initialization - no initial state needed
pub static MEMORY_PROTECTOR: Lazy<Mutex<MemoryProtector>> = Lazy::new(|| {
    Mutex::new(MemoryProtector::new())
});