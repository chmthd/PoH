use sha2::{Sha256, Digest};
use chrono::{Utc, DateTime, NaiveDateTime};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct PohEntry {
    pub transactions: Vec<String>,
    pub timestamp: i64,           
    pub readable_timestamp: String,
    pub hash: String,
}

impl PohEntry {
    pub fn new(transactions: Vec<String>, prev_hash: &str) -> Self {
        let timestamp = Utc::now().timestamp();
        let readable_timestamp = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let mut hasher = Sha256::new();
        for tx in &transactions {
            hasher.update(tx);
        }
        hasher.update(prev_hash);
        hasher.update(timestamp.to_string());
        let result = hasher.finalize();
        let mut hash_str = String::new();
        for byte in result {
            write!(&mut hash_str, "{:02x}", byte).expect("Unable to write");
        }

        PohEntry {
            transactions,
            timestamp,
            readable_timestamp,
            hash: hash_str,
        }
    }

    pub fn validate_transaction(tx: &str) -> Result<(), &'static str> {
        if tx.is_empty() {
            return Err("Transaction is empty");
        }
        // more validation rules for later
        Ok(())
    }
}
