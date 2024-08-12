use crate::shard::shard::Shard;
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
        // collect all shard ids that haven't been gossiped yet
        let blocks_to_gossip: Vec<(usize, Vec<Block>)> = shards
            .iter()
            .filter(|shard| !self.known_shards.contains(&shard.id))
            .map(|shard| (shard.id, shard.blocks.clone()))
            .collect();

        // gossip each shard's blocks to other shards
        for (shard_id, blocks) in blocks_to_gossip {
            self.known_shards.push(shard_id);
            println!("Gossiping blocks from shard {} to other shards...", shard_id);

            // update other shards with current shard's blocks
            for other_shard in shards.iter_mut() {
                if other_shard.id != shard_id {
                    other_shard.update_state_from_gossip_data(blocks.clone());
                }
            }
        }
    }
}
