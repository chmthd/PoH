use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
use crate::validator::validator::{Validator, ValidatorPerformance};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use crate::BLOCK_GEN_TIMES;
use crate::LAST_BLOCK_TIMESTAMP;
use chrono::Utc;

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
            min_transactions_per_block: 100,
            max_transactions_per_block,
            last_block_time: Instant::now(),
            block_time_threshold: Duration::from_secs(15),
            epoch_threshold: 10,
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

    pub fn get_processed_transactions(&self) -> Vec<&Transaction> {
        self.transaction_pool.iter()
            .filter(|tx| tx.status == TransactionStatus::Completed)
            .collect()
    }

    pub fn get_processed_transaction_count(&self) -> usize {
        self.blocks.iter().map(|block| block.poh_entries[0].transactions.len()).sum()
    }

    pub fn get_pending_cross_shard_txs_len(&self) -> usize {
        self.pending_cross_shard_txs.len()
    }

    pub fn drain_pending_cross_shard_txs(&mut self) -> Vec<Transaction> {
        self.pending_cross_shard_txs.drain(..).collect()
    }

    pub fn get_transaction_by_id(&self, tx_id: &str) -> Option<&Transaction> {
        self.transaction_pool.iter().find(|tx| tx.id == tx_id)
    }

    pub fn process_transactions(&mut self, transactions: Vec<Transaction>) {
        for mut tx in transactions {
            if tx.to_shard == self.id {
                println!("Shard {}: Adding transaction {} to pool", self.id, tx.id);
                tx.status = TransactionStatus::Pending;
                self.transaction_pool.push(tx.clone());
            }
        }

        self.check_and_create_block();
    }

    pub fn process_cross_shard_transaction(&mut self, mut transaction: Transaction) {
        if !self.processed_transactions.contains(&transaction.id) {
            println!(
                "Shard {}: Processing cross-shard transaction {} from Shard {}",
                self.id, transaction.id, transaction.from_shard
            );

            transaction.status = TransactionStatus::Pending;
            self.transaction_pool.push(transaction.clone());

            self.check_and_create_block();
        } else {
            println!(
                "Shard {}: Ignoring duplicate cross-shard transaction {}",
                self.id, transaction.id
            );
        }
    }

    pub fn add_pending_cross_shard_tx(&mut self, transaction: Transaction) {
        if !self.processed_transactions.contains(&transaction.id) {
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
        } else {
            println!(
                "Shard {}: Ignoring pending cross-shard transaction {} (already processed).",
                self.id, transaction.id
            );
        }
    }

    pub fn check_and_create_block(&mut self) {
        let total_transactions = self.transaction_pool.len() + self.pending_cross_shard_txs.len();
        let time_since_last_block = self.last_block_time.elapsed();

        if time_since_last_block >= self.block_time_threshold {
            println!("Shard {}: Block time threshold exceeded. Creating block.", self.id);
            self.create_block();
            self.last_block_time = Instant::now();
            return;
        }

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
        if total_transactions > 1500 {
            1500
        } else if total_transactions > 750 {
            (self.max_transactions_per_block as f64 * 0.5) as usize
        } else if total_transactions > 300 {
            (self.max_transactions_per_block as f64 * 0.2) as usize
        } else {
            self.min_transactions_per_block
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
    
        let block_creation_time = Instant::now();
    
        let mut transactions_to_include = Vec::new();
    
        transactions_to_include.extend(self.pending_cross_shard_txs.drain(..));
        transactions_to_include.extend(self.transaction_pool.drain(..));
    
        transactions_to_include.truncate(self.max_transactions_per_block);
    
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
    
                    for tx in transactions_to_include.iter_mut() {
                        tx.status = TransactionStatus::Completed;
                        self.processed_transactions.insert(tx.id.clone());
                        self.transaction_count += 1;
                        println!("Transaction {} status updated to Completed.", tx.id);
                    }
    
                    let block_duration = block_creation_time.elapsed();
    
                    let mut last_block_ts = LAST_BLOCK_TIMESTAMP.lock().unwrap();
                    if let Some(prev_timestamp) = *last_block_ts {
                        let current_time = Utc::now();
                        let time_diff = current_time.signed_duration_since(prev_timestamp)
                            .to_std()
                            .unwrap_or(Duration::from_millis(1));
    
                        BLOCK_GEN_TIMES.lock().unwrap().push(time_diff);
                    }
    
                    *last_block_ts = Some(Utc::now());
    
                    let current_time = chrono::Utc::now();
                    println!("#{} created in {} ms at {}. {} transactions included",
                        block.block_number, block_duration.as_millis(),
                        current_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                        transactions_to_include.len()
                    );
                } else {
                    println!("Shard {}: Block #{} failed validation. Discarding block.", self.id, block_number);
                    self.transaction_pool.extend(transactions_to_include);
                }
            }
            Err(e) => {
                println!("Shard {}: Error processing transactions: {}", self.id, e);
                self.transaction_pool.extend(transactions_to_include);
            }
        }
    }

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

    pub fn drain_pending_checkpoint(&mut self) -> Option<Checkpoint> {
        self.pending_checkpoint.take()
    }

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
        self.transaction_count >= self.epoch_threshold || self.epoch_start_time.elapsed() >= Duration::from_secs(3600)
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
