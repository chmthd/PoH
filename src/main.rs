mod poh;
mod shard;

use shard::shard::Shard;

fn distribute_transactions(shards: &mut Vec<Shard>, transactions: Vec<String>) {
    let mut shard_index = 0;
    for tx in transactions {
        shards[shard_index].process_transactions(vec![tx]);
        shard_index = (shard_index + 1) % shards.len();
    }
}

fn main() {
    // change this to dynamic later
    let mut shards = vec![
        Shard::new(1, 3), // id 1, batch size 3
        Shard::new(2, 2), // 
    ];

    // txs
    let transactions = vec![
        "Transaction 1".to_string(),
        "Transaction 2".to_string(),
        "Transaction 3".to_string(),
        "Transaction 4".to_string(),
        "Transaction 5".to_string(),
        "Transaction 6".to_string(),
    ];

    // distribute across shards
    distribute_transactions(&mut shards, transactions);
    // run gossip protocol
    let mut gossip_protocol = GossipProtocol::new();
    gossip_protocol.gossip(&mut shards);
}