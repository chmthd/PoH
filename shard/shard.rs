use crate::poh::generator::PohGenerator;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub amount: u64,
    pub from_shard: usize,
}

#[derive(Debug)]
pub struct Shard {
    pub id: usize,
    generator: PohGenerator,
    epoch: usize,
    transaction_count: usize,
    transactions: Vec<Transaction>, // processed transactions
    ledger: HashMap<String, u64>,
}

impl Shard {
    pub fn new(id: usize, batch_size: usize) -> Self {
        Shard {
            id,
            generator: PohGenerator::new(batch_size),
            epoch: 0,
            transaction_count: 0,
            transactions: Vec::new(),
            ledger: HashMap::new(),
        }
    }

    pub fn get_transactions(&self) -> Vec<Transaction> {
        self.transactions.clone()
    }

    pub fn process_transactions(&mut self, transactions: Vec<Transaction>) {
        self.transaction_count += transactions.len();

        // add txs to the shard's tx list
        for tx in &transactions {
            self.transactions.push(tx.clone());
            *self.ledger.entry(tx.id.clone()).or_insert(0) += tx.amount;
        }

        let tx_strings: Vec<String> = transactions.iter().map(|tx| tx.id.clone()).collect();
        match self.generator.generate_entries(tx_strings) {
            Ok(entries) => {
                for entry in entries {
                    println!("Shard {}: Processed PoH Entry: {:?}", self.id, entry);
                }
            }
            Err(e) => {
                println!("Shard {}: Error processing transactions: {}", self.id, e);
            }
        }

        // dynamic epoch management based on the number of transactions
        if self.transaction_count >= 10 {
            self.epoch += 1;
            self.transaction_count = 0;
            println!("Shard {}: Moving to next epoch: {}", self.id, self.epoch);
        }
    }

    // incorporate gossiped transaction data
    pub fn update_state_from_gossip_data(&mut self, transactions: Vec<Transaction>) {
        println!("Shard {} is incorporating gossiped transactions.", self.id);
        for tx in transactions {
            if !self.transactions.iter().any(|t| t.id == tx.id) {
                self.transactions.push(tx.clone());
                *self.ledger.entry(tx.id.clone()).or_insert(0) += tx.amount;
            }
        }
    }
}
