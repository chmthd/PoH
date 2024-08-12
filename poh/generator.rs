use crate::poh::entry::PohEntry;

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

    pub fn generate_entry(&mut self, transactions: Vec<String>) -> Result<PohEntry, &'static str> {
        for tx in &transactions {
            PohEntry::validate_transaction(tx)?;
        }
        let entry = PohEntry::new(transactions, &self.previous_hash);
        self.previous_hash = entry.hash.clone();
        Ok(entry)
    }

    pub fn generate_entries(&mut self, transactions: Vec<String>) -> Result<Vec<PohEntry>, &'static str> {
        if transactions.is_empty() {
            return Err("No transactions provided");
        }
        let mut entries = Vec::new();
        for chunk in transactions.chunks(self.batch_size) {
            let entry = self.generate_entry(chunk.to_vec())?;
            entries.push(entry);
        }
        Ok(entries)
    }
}
