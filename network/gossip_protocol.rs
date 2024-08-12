use crate::shard::shard::Shard;

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
        let shards_to_gossip: Vec<usize> = shards
            .iter()
            .filter(|shard| !self.known_shards.contains(&shard.id))
            .map(|shard| shard.id)
            .collect();

        // gossip state to others
        for &shard_id in &shards_to_gossip {
            self.known_shards.push(shard_id);

            // get data from shard
            let shard_data = {
                let shard = shards.iter().find(|shard| shard.id == shard_id).unwrap();
                (shard.id, shard.get_transactions())
            };

            println!("Gossiping shard {} state to other shards...", shard_data.0);

            // update other shards with current shard's state
            for other_shard in shards.iter_mut() {
                if other_shard.id != shard_data.0 {
                    other_shard.update_state_from_gossip_data(shard_data.1.clone());
                }
            }
        }
    }
}
