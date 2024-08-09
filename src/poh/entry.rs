use sha2::{Sha256, Digest};
use chrono::Utc;
use std::fmt::Write;

#[derive(Debug)]
pub struct PohEntry {
    pub data: String,
    pub timestamp: i64,
    pub hash: String,
}

impl PohEntry {
    pub fn new(data: &str, prev_hash: &str) -> Self {
        let timestamp = Utc::now().timestamp();
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(prev_hash);
        hasher.update(timestamp.to_string());
        let result = hasher.finalize();
        let mut hash_str = String::new();
        for byte in result {
            write!(&mut hash_str, "{:02x}", byte).expect("Unable to write");
        }
        
        PohEntry {
            data: data.to_string(),
            timestamp,
            hash: hash_str,
        }
    }
}
