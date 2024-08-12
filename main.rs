mod poh;
mod block;
mod shard;
mod network;

use shard::shard::{Shard, Transaction};
use network::gossip_protocol::GossipProtocol;
use std::thread;
use std::time::Duration;
use rand::Rng;

fn send_random_transactions(shards: &mut Vec<Shard>, gossip_protocol: &mut GossipProtocol) {
    let mut rng = rand::thread_rng();
    let mut tx_count = 1;

    loop {
        let amount = rng.gen_range(1..1000);
        let shard_index = rng.gen_range(0..shards.len());

        let transaction = Transaction {
            id: format!("tx{}", tx_count),
            amount,
            from_shard: shard_index + 1,
        };

        println!("Sending Transaction {} to Shard {}", transaction.id, transaction.from_shard);

        shards[shard_index].process_transactions(vec![transaction]);
        gossip_protocol.gossip(shards);

        tx_count += 1;

        thread::sleep(Duration::from_secs(1));
    }
}

fn main() {
    // change this to dynamic later/////////////////////////////////////////////////
    let mut shards = vec![
        Shard::new(1, 3), // id 1, batch size 3
        Shard::new(2, 2), // 
    ];

    // run gossip protocol
    let mut gossip_protocol = GossipProtocol::new();
    
    // distribute across shards
    send_random_transactions(&mut shards, &mut gossip_protocol);
}
