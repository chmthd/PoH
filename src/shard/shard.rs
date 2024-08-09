use crate::poh::generator::PohGenerator;

#[derive(Debug)]
pub struct Shard {
    pub id: usize,
    generator: PohGenerator,
    epoch: usize,
    transaction_count: usize,
}

impl Shard {
    pub fn new(id: usize, batch_size: usize) -> Self {
        Shard {
            id,
            generator: PohGenerator::new(batch_size),
            epoch: 0,
            transaction_count: 0,
        }
    }

    pub fn process_transactions(&mut self, transactions: Vec<String>) {
        self.transaction_count += transactions.len();
        match self.generator.generate_entries(transactions) {
            Ok(entries) => {
                for entry in entries {
                    println!("Shard {}: Processed PoH Entry: {:?}", self.id, entry);
                }
            }
            Err(e) => {
                println!("Shard {}: Error processing transactions: {}", self.id, e);
            }
        }

        // dynamic epoch management based on the number of txs
        if self.transaction_count >= 10 {
            self.epoch += 1;
            self.transaction_count = 0;
            println!("Shard {}: Moving to next epoch: {}", self.id, self.epoch);
        }
    }
}
