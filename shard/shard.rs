use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
use crate::validator::validator::{Validator, ValidatorPerformance};
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
    epoch_start_time: Instant,
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
            epoch_start_time: Instant::now(),
        }
    }

    pub fn get_validators(&self) -> &Vec<Validator> {
        &self.validators
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

    // Process intra-shard transactions
    pub fn process_transactions(&mut self, transactions: Vec<Transaction>) {
        for mut tx in transactions {
            if !self.processed_transactions.contains(&tx.id) {
                if tx.to_shard == self.id {
                    println!("Shard {}: Processing transaction {}", self.id, tx.id);
                    tx.status = TransactionStatus::Processing;
                    self.transaction_pool.push(tx.clone());
                    self.transaction_count += 1;
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

    // Process cross-shard transactions
    pub fn process_cross_shard_transaction(&mut self, mut transaction: Transaction) {
        if self.processed_transactions.contains(&transaction.id) {
            println!(
                "Shard {}: Ignoring duplicate cross-shard transaction {}",
                self.id, transaction.id
            );
            return;
        }

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

        if self.check_epoch_transition() {
            self.transition_to_next_epoch();
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

        // Collect validator performance data
        let mut validator_performance: HashMap<usize, ValidatorPerformance> = HashMap::new();
        for validator in &self.validators {
            validator_performance.insert(validator.id, ValidatorPerformance::from_validator(validator));
        }

        // Generate PoH entries with validator performance data
        match self.generator.generate_entries(tx_strings, validator_performance) {
            Ok(entries) => {
                let block_number = self.blocks.len() as u64 + 1;
                let previous_hash = self
                    .blocks
                    .last()
                    .map(|block| block.block_hash.clone())
                    .unwrap_or_else(|| "0".to_string());

                let block = Block::new(block_number, entries.clone(), &previous_hash);

                // Validate the block before adding it to the chain
                if self.validate_block_with_validators(&block) {
                    self.blocks.push(block.clone());

                    println!(
                        "Shard {}: Processed Block #{} with {} transactions",
                        self.id,
                        block.block_number,
                        transactions_to_include.len()
                    );
                    println!("Block Hash: {}", block.block_hash);
                    println!("Previous Hash: {}", previous_hash);
                    println!(
                        "Included Transactions: {:?}",
                        transactions_to_include.iter().map(|tx| tx.id.clone()).collect::<Vec<_>>()
                    );
                } else {
                    println!("Shard {}: Block #{} failed validation. Discarding block.", self.id, block_number);
                }
            }
            Err(e) => {
                println!("Shard {}: Error processing transactions: {}", self.id, e);
                self.transaction_pool.extend(transactions_to_include);
            }
        }
    }

    fn validate_block_with_validators(&mut self, block: &Block) -> bool {
        let mut total_weight = 0.0;
        let mut positive_weight = 0.0;
        let current_epoch = self.epoch;

        for validator in self.validators.iter_mut() {
            let weight = validator.get_final_vote_weight(current_epoch);
            total_weight += weight;

            if weight > 0.5 {
                positive_weight += weight;
                println!("Validator {} voted positive for block with weight {:.2}", validator.id, weight);
            } else {
                println!("Validator {} voted negative for block with weight {:.2}", validator.id, weight);
            }
        }

        println!(
            "Shard {}: Block validation result: Positive Weight: {:.2}, Total Weight: {:.2}",
            self.id, positive_weight, total_weight
        );

        positive_weight > (total_weight * 0.5)
    }

    fn check_epoch_transition(&self) -> bool {
        self.transaction_count >= self.epoch_threshold || self.epoch_start_time.elapsed() >= Duration::from_secs(60)
    }

    fn transition_to_next_epoch(&mut self) {
        self.epoch += 1;
        self.epoch_start_time = Instant::now();
        self.transaction_count = 0;

        println!("Shard {}: Transitioning to epoch {}", self.id, self.epoch);

        // Recalculate validator rankings
        self.recalculate_validator_rankings();

        // Assign validators based on network conditions and shard needs
        self.dynamic_assign_validators();

        for validator in &mut self.validators {
            validator.epochs_active += 1;
        }

        self.transaction_pool.clear();
        self.processed_transactions.clear();
    }

    fn recalculate_validator_rankings(&mut self) {
        self.validators.sort_by(|a, b| {
            b.get_final_vote_weight(self.epoch)
                .partial_cmp(&a.get_final_vote_weight(self.epoch))
                .unwrap()
        });

        println!("Re-ranked validators for Shard {} at Epoch {}", self.id, self.epoch);
    }

    fn dynamic_assign_validators(&mut self) {
        // Assess the network conditions and assign validators based on the transaction volume
        // Load balancing and validator re-assignment logic would be added here

        println!("Dynamic assignment of validators for Shard {} at Epoch {}", self.id, self.epoch);
    }
}
