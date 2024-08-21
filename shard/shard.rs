use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
use crate::validator::validator::Validator;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub amount: u64,
    pub from_shard: usize,
    pub to_shard: usize,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug)]
pub struct Shard {
    pub id: usize,
    generator: PohGenerator,
    epoch: usize,
    transaction_count: usize,
    transaction_pool: Vec<Transaction>,
    ledger: HashMap<String, u64>,
    pub blocks: Vec<Block>,
    validators: Vec<Validator>,
    min_transactions_per_block: usize,
    max_transactions_per_block: usize,
    last_block_time: Instant,
    block_time_threshold: Duration,
    epoch_threshold: usize,
    processed_transactions: HashSet<String>,
    pending_cross_shard_txs: Vec<Transaction>,
}

impl Shard {
    pub fn new(id: usize, batch_size: usize, max_transactions_per_block: usize, validators: Vec<Validator>) -> Self {
        Shard {
            id,
            generator: PohGenerator::new(batch_size),
            epoch: 0,
            transaction_count: 0,
            transaction_pool: Vec::new(),
            ledger: HashMap::new(),
            blocks: Vec::new(),
            validators,
            min_transactions_per_block: 10,
            max_transactions_per_block,
            last_block_time: Instant::now(),
            block_time_threshold: Duration::from_secs(10),
            epoch_threshold: 10,
            processed_transactions: HashSet::new(),
            pending_cross_shard_txs: Vec::new(),
        }
    }

    pub fn get_transaction_pool(&self) -> &Vec<Transaction> {
        &self.transaction_pool
    }

    pub fn get_processed_transactions(&self) -> &HashSet<String> {
        &self.processed_transactions
    }

    pub fn get_pending_cross_shard_txs_len(&self) -> usize {
        self.pending_cross_shard_txs.len()
    }

    pub fn drain_pending_cross_shard_txs(&mut self) -> Vec<Transaction> {
        self.pending_cross_shard_txs.drain(..).collect()
    }

    pub fn process_transactions(&mut self, transactions: Vec<Transaction>) {
        for mut tx in transactions {
            if !self.processed_transactions.contains(&tx.id) {
                if tx.to_shard == self.id {
                    if self.validate_transaction_with_validators(&tx) {
                        println!("Shard {}: Processing transaction {}", self.id, tx.id);
                        tx.status = TransactionStatus::Processing;
                        self.transaction_pool.push(tx.clone());
                        self.transaction_count += 1;
                    } else {
                        tx.status = TransactionStatus::Failed;
                        println!("Shard {}: Transaction {} failed validation", self.id, tx.id);
                    }
                }
                self.processed_transactions.insert(tx.id.clone());
            }
        }

        if self.transaction_pool.len() >= self.min_transactions_per_block {
            self.check_and_create_block();
        } else {
            println!(
                "Shard {}: Not enough transactions to create a block. Waiting for more transactions.",
                self.id
            );
        }
    }

    pub fn process_cross_shard_transaction(&mut self, mut transaction: Transaction) {
        if self.processed_transactions.contains(&transaction.id) {
            println!(
                "Shard {}: Ignoring duplicate cross-shard transaction {}",
                self.id, transaction.id
            );
            return;
        }

        if self.validate_transaction_with_validators(&transaction) {
            println!(
                "Shard {}: Processing cross-shard transaction {} from Shard {}",
                self.id, transaction.id, transaction.from_shard
            );

            transaction.status = TransactionStatus::Processing;
            self.transaction_pool.push(transaction.clone());
            self.transaction_count += 1;
            self.processed_transactions.insert(transaction.id.clone());

            if self.transaction_pool.len() >= self.min_transactions_per_block {
                self.check_and_create_block();
            } else {
                println!(
                    "Shard {}: Not enough transactions to create a block. Waiting for more transactions.",
                    self.id
                );
            }

            println!(
                "Shard {}: Completed processing cross-shard transaction {}. New status: {:?}",
                self.id, transaction.id, transaction.status
            );
        } else {
            println!("Shard {}: Cross-shard transaction {} failed validation", self.id, transaction.id);
        }
    }

    fn validate_transaction_with_validators(&mut self, transaction: &Transaction) -> bool {
        for validator in self.validators.iter_mut() {
            if !validator.validate_transaction(&transaction.id) {
                println!("Validator {} failed to validate transaction {}", validator.id, transaction.id);
                return false;
            }
        }
        true
    }

    pub fn check_and_create_block(&mut self) {
        let total_transactions = self.transaction_pool.len() + self.pending_cross_shard_txs.len();
        let time_since_last_block = self.last_block_time.elapsed();

        if total_transactions >= self.min_transactions_per_block && time_since_last_block >= self.block_time_threshold {
            self.create_block();
            self.last_block_time = Instant::now();
        } else {
            println!(
                "Shard {}: Not enough transactions or time not yet reached. Current pool size: {}, Time since last block: {:?}",
                self.id, total_transactions, time_since_last_block
            );
        }
    }

    fn create_block(&mut self) {
        let total_transactions = self.transaction_pool.len() + self.pending_cross_shard_txs.len();
        if total_transactions < self.min_transactions_per_block {
            println!(
                "Shard {}: Not enough transactions to fill the block. Current pool size: {}. Waiting for more transactions...",
                self.id, total_transactions
            );
            return;
        }
    
        let mut transactions_to_include = Vec::new();
    
        transactions_to_include.extend(self.pending_cross_shard_txs.drain(..));
        transactions_to_include.extend(self.transaction_pool.drain(..));
    
        transactions_to_include.truncate(self.max_transactions_per_block);
    
        for tx in transactions_to_include.iter_mut() {
            tx.status = TransactionStatus::Completed;
            println!("Transaction {} status updated to Completed.", tx.id);
        }
    
        let tx_strings: Vec<String> = transactions_to_include.iter().map(|tx| tx.id.clone()).collect();
    
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
    
                println!(
                    "Shard {}: Processed Block {:?} with {} transactions",
                    self.id,
                    self.blocks.last().unwrap(),
                    transactions_to_include.len()
                );
            }
            Err(e) => {
                println!("Shard {}: Error processing transactions: {}", self.id, e);
                self.transaction_pool.extend(transactions_to_include);
            }
        }
    }
    
    pub fn add_pending_cross_shard_tx(&mut self, transaction: Transaction) {
        if self.processed_transactions.contains(&transaction.id) {
            println!(
                "Shard {}: Ignoring pending cross-shard transaction {} (already processed).",
                self.id, transaction.id
            );
            return;
        }

        println!(
            "Shard {}: Adding pending cross-shard transaction {}",
            self.id, transaction.id
        );
        self.pending_cross_shard_txs.push(transaction);

        println!(
            "Shard {}: Pending cross-shard transactions count: {}",
            self.id,
            self.pending_cross_shard_txs.len()
        );
    }
}
