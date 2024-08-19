use crate::shard::shard::{Shard, Transaction};
use crate::block::block::Block;

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
        // gossip blocks across shards
        let blocks_to_gossip: Vec<(usize, Vec<Block>)> = shards
            .iter()
            .filter(|shard| !self.known_shards.contains(&shard.id))
            .map(|shard| (shard.id, shard.blocks.clone()))
            .collect();

        for (shard_id, blocks) in blocks_to_gossip {
            self.known_shards.push(shard_id);
            println!("Gossiping blocks from shard {} to other shards...", shard_id);

            for other_shard in shards.iter_mut() {
                if other_shard.id != shard_id {
                    other_shard.update_state_from_gossip_data(blocks.clone());
                }
            }
        }

        // process cross-shard txs
        let mut cross_shard_transactions = Vec::new();
        for shard in shards.iter_mut() {
            cross_shard_transactions.extend(shard.extract_cross_shard_transactions());
        }

        for tx in cross_shard_transactions {
            if let Some(target_shard) = shards.iter_mut().find(|s| s.id == tx.to_shard) {
                println!(
                    "Gossip: Forwarding transaction {} from Shard {} to Shard {}",
                    tx.id, tx.from_shard, tx.to_shard
                );
                target_shard.process_cross_shard_transaction(tx.clone());

                // process block after transaction is received
                target_shard.process_block_with_prioritization();
            } else {
                println!("Gossip: No target shard found for transaction {}", tx.id);
            }
        }
    }

    pub fn periodic_gossip(&mut self, shards: &mut [Shard]) {
        println!("Performing periodic gossip...");
        self.gossip(shards);
    }
}
