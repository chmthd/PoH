use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
use crate::validator::validator::{Validator, ValidatorPerformance};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

// Define the Checkpoint struct
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub shard_id: usize,
    pub block_height: u64,
    pub ledger_snapshot: HashMap<String, u64>,
    pub transaction_pool_snapshot: Vec<Transaction>,
    pub processed_transactions_snapshot: HashSet<String>,
}

impl Checkpoint {
    pub fn new(
        shard_id: usize,
        block_height: u64,
        ledger_snapshot: HashMap<String, u64>,
        transaction_pool_snapshot: Vec<Transaction>,
        processed_transactions_snapshot: HashSet<String>,
    ) -> Self {
        Checkpoint {
            shard_id,
            block_height,
            ledger_snapshot,
            transaction_pool_snapshot,
            processed_transactions_snapshot,
        }
    }
}

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
    pub epoch: usize,
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
    pub epoch_start_time: Instant,
    pub pending_checkpoint: Option<Checkpoint>, 
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
            min_transactions_per_block: 50,  
            max_transactions_per_block,
            last_block_time: Instant::now(),
            block_time_threshold: Duration::from_secs(30), 
            epoch_threshold: 3000,
            processed_transactions: HashSet::new(),
            pending_cross_shard_txs: Vec::new(),
            epoch_start_time: Instant::now(),
            pending_checkpoint: None,
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

        self.check_and_create_block();
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

        self.check_and_create_block();
    }

    // Add the pending cross-shard transaction to the queue
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

        // Check if time has passed since the last block creation
        if time_since_last_block >= self.block_time_threshold {
            println!("Shard {}: Block time threshold exceeded. Creating block.", self.id);
            self.create_block();
            self.last_block_time = Instant::now();
            return;
        }

        // If block time hasn't passed, check if there are enough transactions
        let dynamic_min_transactions = self.calculate_dynamic_min_transactions(total_transactions);

        if total_transactions >= dynamic_min_transactions {
            println!("Shard {}: Min transaction threshold reached. Creating block.", self.id);
            self.create_block();
            self.last_block_time = Instant::now();
        } else {
            println!(
                "Shard {}: Waiting for more transactions or block time threshold. Current pool size: {}.",
                self.id, total_transactions
            );
        }

        if self.check_epoch_transition() {
            self.transition_to_next_epoch();
        }
    }

    fn calculate_dynamic_min_transactions(&self, total_transactions: usize) -> usize {
        if total_transactions > 100 {
            100 // High traffic: larger blocks
        } else if total_transactions > 50 {
            (self.max_transactions_per_block as f64 * 0.8) as usize // 80% of max
        } else if total_transactions > 20 {
            (self.max_transactions_per_block as f64 * 0.5) as usize // 50% of max
        } else {
            self.min_transactions_per_block // Use the dynamic minimum from the configuration

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

        let mut validator_performance: HashMap<usize, ValidatorPerformance> = HashMap::new();
        for validator in &self.validators {
            validator_performance.insert(validator.id, ValidatorPerformance::from_validator(validator));
        }

        match self.generator.generate_entries(tx_strings, validator_performance) {
            Ok(entries) => {
                let block_number = self.blocks.len() as u64 + 1;
                let previous_hash = self
                    .blocks
                    .last()
                    .map(|block| block.block_hash.clone())
                    .unwrap_or_else(|| "0".to_string());

                let block = Block::new(block_number, entries.clone(), &previous_hash);

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

    // Validate block with validators
    pub fn validate_block_with_validators(&mut self, block: &Block) -> bool {
        let mut total_weight = 0.0;
        let mut positive_weight = 0.0;
        let current_epoch = self.epoch;

        for validator in self.validators.iter_mut() {
            let weight = validator.get_final_vote_weight(current_epoch);
            total_weight += weight;

            if weight > 0.2 {
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

        positive_weight > (total_weight * 0.3)
    }

    // Drain pending checkpoint for cross-shard gossip
    pub fn drain_pending_checkpoint(&mut self) -> Option<Checkpoint> {
        self.pending_checkpoint.take()
    }

    // Receive and process checkpoint from other shards
    pub fn receive_checkpoint(&mut self, checkpoint: Checkpoint) {
        println!("Shard {}: Received checkpoint from Shard {}", self.id, checkpoint.shard_id);
        self.ledger = checkpoint.ledger_snapshot;
        self.processed_transactions = checkpoint.processed_transactions_snapshot;
        self.transaction_pool = checkpoint.transaction_pool_snapshot;
        self.blocks.clear();
    }

    pub fn capture_checkpoint(&self) -> Checkpoint {
        Checkpoint::new(
            self.id,
            self.blocks.len() as u64,
            self.ledger.clone(),
            self.transaction_pool.clone(),
            self.processed_transactions.clone(),
        )
    }

    pub fn check_epoch_transition(&self) -> bool {
        self.transaction_count >= self.epoch_threshold || self.epoch_start_time.elapsed() >= Duration::from_secs(30)
    }

    pub fn transition_to_next_epoch(&mut self) {
        self.epoch += 1;
        self.epoch_start_time = Instant::now();
        self.transaction_count = 0;

        println!("Shard {}: Transitioning to epoch {}", self.id, self.epoch);

        self.recalculate_validator_rankings();

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
        println!("Dynamic assignment of validators for Shard {} at Epoch {}", self.id, self.epoch);
    }
}
