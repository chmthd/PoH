use crate::poh::entry::PohEntry;
use crate::validator::validator::ValidatorPerformance;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PohGenerator {
    pub previous_hash: String,
    pub batch_size: usize,
}

impl PohGenerator {
    pub fn new(batch_size: usize) -> Self {
        PohGenerator {
            previous_hash: String::from("0"),
            batch_size,
        }
    }

    pub fn generate_entry(
        &mut self,
        transactions: Vec<String>,
        validator_performance: &HashMap<usize, ValidatorPerformance>,
    ) -> Result<PohEntry, &'static str> {
        for tx in &transactions {
            PohEntry::validate_transaction(tx)?;
        }

        let entry = PohEntry::new(transactions, &self.previous_hash);

        // update the poh hash based on validator performance
        for performance in validator_performance.values() {
            let contribution_factor = performance.honesty_score * (1.0 / (performance.response_time + 1.0));
            self.previous_hash = format!("{}:{}", self.previous_hash, contribution_factor);
        }

        self.previous_hash = entry.hash.clone();
        Ok(entry)
    }

    pub fn generate_entries(
        &mut self,
        transactions: Vec<String>,
        validator_performance: HashMap<usize, ValidatorPerformance>,
    ) -> Result<Vec<PohEntry>, &'static str> {
        if transactions.is_empty() {
            return Err("No transactions provided");
        }
        let mut entries = Vec::new();
        for chunk in transactions.chunks(self.batch_size) {
            let entry = self.generate_entry(chunk.to_vec(), &validator_performance)?;
            entries.push(entry);
        }
        Ok(entries)
    }
}
