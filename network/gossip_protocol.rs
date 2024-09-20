use crate::shard::shard::{Shard, Transaction, Checkpoint};

pub struct GossipProtocol {
    pub known_shards: Vec<usize>,
}

impl GossipProtocol {
    pub fn new() -> Self {
        GossipProtocol {
            known_shards: Vec::new(),
        }
    }

    pub fn gossip(&mut self, shards: &mut [Shard]) {
        let mut transactions_to_forward: Vec<Transaction> = Vec::new();

        for shard in shards.iter_mut() {
            transactions_to_forward.extend(shard.drain_pending_cross_shard_txs());
        }

        for tx in transactions_to_forward {
            if let Some(target_shard) = shards.iter_mut().find(|s| s.id == tx.to_shard) {
                println!(
                    "Gossip: Forwarding transaction {} from Shard {} to Shard {}",
                    tx.id, tx.from_shard, tx.to_shard
                );
                target_shard.process_cross_shard_transaction(tx);
            } else {
                println!("Gossip: No target shard found for transaction {}", tx.id);
            }
        }
    }

    pub fn periodic_gossip(&mut self, shards: &mut [Shard]) {
        println!("Performing periodic gossip...");
        self.gossip(shards);
    }

    // Handle checkpoint gossiping across shards
    pub fn gossip_checkpoints(&mut self, checkpoint: &Checkpoint, shards: &mut [Shard]) {
        println!(
            "Gossip: Broadcasting checkpoint from Shard {} to other shards",
            checkpoint.shard_id
        );

        for shard in shards.iter_mut() {
            if shard.id != checkpoint.shard_id {
                println!(
                    "Gossip: Sending checkpoint from Shard {} to Shard {}",
                    checkpoint.shard_id, shard.id
                );
                shard.receive_checkpoint(checkpoint.clone());
            }
        }
    }
}
