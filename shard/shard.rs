use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
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
    pub blocks: Vec<Block>,
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
            blocks: Vec::new(),
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
                let block_number = self.blocks.len() as u64 + 1;
                let previous_hash = if let Some(last_block) = self.blocks.last() {
                    &last_block.block_hash
                } else {
                    "0"
                };
                let block = Block::new(block_number, entries, previous_hash);
                self.blocks.push(block);

                println!("Shard {}: Processed Block {:?}", self.id, self.blocks.last().unwrap());
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

    // incorporate gossiped block data
    pub fn update_state_from_gossip_data(&mut self, blocks: Vec<Block>) {
        println!("Shard {} is incorporating gossiped blocks.", self.id);
        for block in blocks {
            if !self.blocks.iter().any(|b| b.block_hash == block.block_hash) {
                self.blocks.push(block);
                println!("Shard {}: Added gossiped block to state.", self.id);
            }
        }
    }
}
