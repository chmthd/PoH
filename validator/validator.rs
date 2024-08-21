#[derive(Debug)]
pub struct Validator {
    pub id: usize,
}

impl Validator {
    pub fn new(id: usize) -> Self {
        Validator { id }
    }

    pub fn validate_transaction(&self, transaction_id: &str) -> bool {
        println!("Validator {} is validating transaction {}", self.id, transaction_id);
        true 
    }
}
