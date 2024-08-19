use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
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
    max_transactions_per_block: usize,
    last_block_time: Instant,
    block_time_threshold: Duration,
    epoch_threshold: usize,
    processed_transactions: HashSet<String>,
    pending_cross_shard_txs: Vec<Transaction>,
    //pending_acknowledgments: HashSet<String>,
    //acknowledged_transactions: HashSet<String>,
}

impl Shard {
    pub fn new(id: usize, batch_size: usize, max_transactions_per_block: usize) -> Self {
        Shard {
            id,
            generator: PohGenerator::new(batch_size),
            epoch: 0,
            transaction_count: 0,
            transaction_pool: Vec::new(),
            ledger: HashMap::new(),
            blocks: Vec::new(),
            max_transactions_per_block,
            last_block_time: Instant::now(),
            block_time_threshold: Duration::from_secs(10),
            epoch_threshold: 10,
            processed_transactions: HashSet::new(),
            pending_cross_shard_txs: Vec::new(),
            //pending_acknowledgments: HashSet::new(),
            //acknowledged_transactions: HashSet::new(),
        }
    }

    pub fn get_transaction_pool_len(&self) -> usize {
        self.transaction_pool.len()
    }

    pub fn get_processed_transactions_len(&self) -> usize {
        self.processed_transactions.len()
    }

    pub fn get_pending_cross_shard_txs_len(&self) -> usize {
        self.pending_cross_shard_txs.len()
    }

    /* pub fn get_pending_acknowledgments_len(&self) -> usize {
        self.pending_acknowledgments.len()
    } */

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

        //self.pending_acknowledgments.insert(transaction.id.clone());
        self.check_and_create_block();

        println!(
            "Shard {}: Completed processing cross-shard transaction {}. New status: {:?}",
            self.id, transaction.id, transaction.status
        );
    }

    /*
    pub fn acknowledge_transaction(&mut self, transaction_id: &str) {
        if self.acknowledged_transactions.contains(transaction_id) {
            println!(
                "Shard {}: Already acknowledged transaction {}",
                self.id, transaction_id
            );
            return;
        }

        if self.pending_acknowledgments.remove(transaction_id) {
            println!("Shard {}: Acknowledged transaction {}", self.id, transaction_id);
            self.acknowledged_transactions.insert(transaction_id.to_string());
            if let Some(tx) = self.transaction_pool.iter_mut().find(|tx| tx.id == transaction_id) {
                tx.status = TransactionStatus::Completed;
            }
        } else {
            println!(
                "Shard {}: Received acknowledgment for unknown transaction {}",
                self.id, transaction_id
            );
        }
    }

    pub fn cleanup_old_acknowledgments(&mut self) {
        self.acknowledged_transactions.clear();
        println!("Shard {}: Cleaned up old acknowledgments.", self.id);
    }

    pub fn send_acknowledgment(&mut self, transaction_id: &str) {
        if self.pending_acknowledgments.contains(transaction_id) {
            println!("Shard {}: Sending acknowledgment for transaction {}", self.id, transaction_id);
            self.acknowledged_transactions.insert(transaction_id.to_string());
            self.pending_acknowledgments.remove(transaction_id);  // Move after print
            if let Some(tx) = self.transaction_pool.iter_mut().find(|tx| tx.id == transaction_id) {
                tx.status = TransactionStatus::Completed;
            }
        } else {
            println!(
                "Shard {}: Attempted to acknowledge an unknown transaction {}",
                self.id, transaction_id
            );
        }
    }
    */

    pub fn update_state_from_gossip_data(&mut self, blocks: Vec<Block>) {
        println!("Shard {} is incorporating gossiped blocks.", self.id);
        for block in blocks {
            if !self.blocks.iter().any(|b| b.block_hash == block.block_hash) {
                self.blocks.push(block);
                println!("Shard {}: Added gossiped block to state.", self.id);
            }
        }
    }

    pub fn extract_cross_shard_transactions(&mut self) -> Vec<Transaction> {
        let cross_shard_transactions = self.pending_cross_shard_txs.clone();
        self.pending_cross_shard_txs.clear();
        cross_shard_transactions
    }

    pub fn check_and_create_block(&mut self) {
        let time_since_last_block = self.last_block_time.elapsed();
        if self.transaction_pool.len() >= self.max_transactions_per_block
            || time_since_last_block >= self.block_time_threshold
        {
            self.create_block();
            self.last_block_time = Instant::now();
        }
    }

    fn create_block(&mut self) {
        if self.transaction_pool.is_empty() && self.pending_cross_shard_txs.is_empty() {
            return;
        }

        let mut transactions_to_include = Vec::new();
        transactions_to_include.extend(
            self.pending_cross_shard_txs
                .drain(..std::cmp::min(
                    self.max_transactions_per_block,
                    self.pending_cross_shard_txs.len(),
                ))
                .collect::<Vec<_>>()
        );

        transactions_to_include.extend(
            self.transaction_pool
                .drain(..std::cmp::min(
                    self.max_transactions_per_block - transactions_to_include.len(),
                    self.transaction_pool.len(),
                ))
                .collect::<Vec<_>>()
        );

        let tx_strings: Vec<String> =
            transactions_to_include.iter().map(|tx| tx.id.clone()).collect();

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
    }

    pub fn process_block_with_prioritization(&mut self) {
        let mut transactions_to_include: Vec<Transaction> = Vec::new();

        // prioritize cross-shard transactions first
        transactions_to_include.extend(
            self.pending_cross_shard_txs
                .drain(..std::cmp::min(
                    self.max_transactions_per_block - transactions_to_include.len(),
                    self.pending_cross_shard_txs.len(),
                ))
                .collect::<Vec<_>>()
        );

        // add remaining transactions from the regular pool
        transactions_to_include.extend(
            self.transaction_pool
                .drain(..std::cmp::min(
                    self.max_transactions_per_block - transactions_to_include.len(),
                    self.transaction_pool.len(),
                ))
                .collect::<Vec<_>>()
        );

        if transactions_to_include.is_empty() {
            println!("Shard {}: No transactions to process, skipping block creation.", self.id);
            return;
        }

        let tx_strings: Vec<String> = transactions_to_include.iter().map(|tx| tx.id.clone()).collect();
        let unique_tx_strings: Vec<String> = tx_strings.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();

        match self.generator.generate_entries(unique_tx_strings) {
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
                self.transaction_pool.extend(transactions_to_include);
            }
        }
    }
}
