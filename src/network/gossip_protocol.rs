use crate::shard::shard::Shard;
use std::collections::HashSet;

#[derive(Debug)]
pub struct GossipProtocol {
    pub known_shards: HashSet<usize>,
}

impl GossipProtocol {
    pub fn new() -> Self {
        GossipProtocol {
            known_shards: HashSet::new(),
        }
    }

    pub fn gossip(&mut self, shards: &mut [Shard]) {
        for shard in shards {
            if self.known_shards.contains(&shard.id) {
                continue;
            }
            println!("Gossiping shard {} state to other shards...", shard.id);
            self.known_shards.insert(shard.id);
            // simulate state sharing between shards
            for other_shard in shards.iter_mut() {
                if shard.id != other_shard.id {
                    println!(
                        "Shard {} is updating state based on Shard {}'s gossip.",
                        other_shard.id, shard.id
                    );
                    // more updates here
                    //
                }
            }
        }
    }
}
