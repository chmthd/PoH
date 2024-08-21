#[derive(Debug)]
pub struct Validator {
    pub id: usize,
    pub shard_id: usize,
}

impl Validator {
    pub fn new(id: usize, shard_id: usize) -> Self {
        Validator { id, shard_id }
    }

    pub fn validate_transaction(&self, transaction_id: &str) -> bool {
        println!(
            "Validator {} (Shard {}) is validating transaction {}",
            self.id, self.shard_id, transaction_id
        );
        true
    }
}
