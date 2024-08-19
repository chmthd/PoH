mod poh;
mod block;
mod shard;
mod network;

use shard::shard::{Shard, Transaction, TransactionStatus};
use network::gossip_protocol::GossipProtocol;
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

const MAX_TRANSACTIONS_PER_BLOCK: usize = 10;

fn hash_to_shard(target: &str, shard_count: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    (hasher.finish() as usize % shard_count) + 1
}

fn print_shard_states(shards: &[Shard]) {
    for shard in shards {
        println!("Shard {} state:", shard.id);
        println!("  Transactions in pool: {}", shard.get_transaction_pool_len());
        println!("  Processed transactions: {}", shard.get_processed_transactions_len());
        println!("  Pending cross-shard transactions: {}", shard.get_pending_cross_shard_txs_len());
        println!("  Blocks: {}", shard.blocks.len());
    }
}

fn send_random_transactions(shards: &mut Vec<Shard>, gossip_protocol: &mut GossipProtocol) {
    let mut rng = rand::thread_rng();
    let mut tx_count = 1;
    let mut block_start_time = Instant::now();
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("blockchain_metrics.log")
        .expect("Cannot open log file");

    loop {
        let amount = rng.gen_range(1..1000);
        let shard_index = rng.gen_range(0..shards.len());

        let transaction_id = format!("tx{}", tx_count);
        let to_shard = hash_to_shard(&transaction_id, shards.len());

        let transaction = Transaction {
            id: transaction_id.clone(),
            amount,
            from_shard: shard_index + 1,
            to_shard,
            status: TransactionStatus::Pending,
        };

        println!(
            "Sending Transaction {} from Shard {} to Shard {} (Status: {:?})",
            transaction.id, transaction.from_shard, transaction.to_shard, transaction.status
        );

        if transaction.from_shard == transaction.to_shard {
            shards[shard_index].process_transactions(vec![transaction.clone()]);
        } else {
            shards[shard_index].add_pending_cross_shard_tx(transaction.clone());
        }

        gossip_protocol.gossip(shards);

        if shards[shard_index].blocks.len() > tx_count / MAX_TRANSACTIONS_PER_BLOCK {
            let block_gen_time = block_start_time.elapsed().as_secs_f64();
            let last_block = shards[shard_index].blocks.last().unwrap();
            let block_size = std::mem::size_of_val(&last_block);
            let tx_count_in_block = last_block.poh_entries.len();
            let log_message = format!(
                "Shard {}: Block {} Generated: Time: {:.2} seconds, Transactions: {}, Block Size: {} bytes\n",
                shard_index + 1,
                last_block.block_number,
                block_gen_time,
                tx_count_in_block,
                block_size
            );

            log_file
                .write_all(log_message.as_bytes())
                .expect("Failed to write to log file");

            block_start_time = Instant::now();
        }

        tx_count += 1;
        thread::sleep(Duration::from_secs(1));

        if tx_count % 5 == 0 {
            gossip_protocol.periodic_gossip(shards);
        }

        if tx_count % 10 == 0 {
            print_shard_states(shards);
        }
    }
}

fn main() {
    let mut shards = vec![
        Shard::new(1, 3, MAX_TRANSACTIONS_PER_BLOCK),
        Shard::new(2, 3, MAX_TRANSACTIONS_PER_BLOCK),
    ];

    let mut gossip_protocol = GossipProtocol::new();

    send_random_transactions(&mut shards, &mut gossip_protocol);
}
