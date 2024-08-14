mod poh;
mod block;
mod shard;
mod network;

use shard::shard::{Shard, Transaction};
use network::gossip_protocol::GossipProtocol;
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;

const MAX_TRANSACTIONS_PER_BLOCK: usize = 50;

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

        // send txs to the first shard for now
        let shard_index = 0;

        let transaction = Transaction {
            id: format!("tx{}", tx_count),
            amount,
            from_shard: shard_index + 1,
        };

        println!("Sending Transaction {} to Shard {}", transaction.id, transaction.from_shard);

        shards[shard_index].process_transactions(vec![transaction]);

        // metrics
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

        gossip_protocol.gossip(shards);

        tx_count += 1;
        thread::sleep(Duration::from_secs(1));
    }
}

fn main() {
    let mut shards = vec![
        Shard::new(1, 3, MAX_TRANSACTIONS_PER_BLOCK),
        Shard::new(2, 2, MAX_TRANSACTIONS_PER_BLOCK), 
    ];

    let mut gossip_protocol = GossipProtocol::new();
    
    send_random_transactions(&mut shards, &mut gossip_protocol);
}
