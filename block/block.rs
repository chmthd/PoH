use crate::poh::entry::PohEntry;
use chrono::Utc;
use sha2::{Sha256, Digest};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct Block {
    pub block_number: u64,
    pub poh_entries: Vec<PohEntry>,
    pub previous_hash: String,
    pub block_hash: String,
    pub timestamp: i64,  // New timestamp field for block generation
}

impl Block {
    pub fn new(block_number: u64, poh_entries: Vec<PohEntry>, previous_hash: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(block_number.to_string());
        for entry in &poh_entries {
            hasher.update(&entry.hash);
        }
        hasher.update(previous_hash);
        let result = hasher.finalize();
        let mut hash_str = String::new();
        for byte in result {
            write!(&mut hash_str, "{:02x}", byte).expect("Unable to write");
        }

        let timestamp = Utc::now().timestamp();  // Capture the timestamp when block is created.

        Block {
            block_number,
            poh_entries,
            previous_hash: previous_hash.to_string(),
            block_hash: hash_str,
            timestamp,  // Assign the block's generation timestamp.
        }
    }
}
