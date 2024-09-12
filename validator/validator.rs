#[derive(Debug)]
pub struct Validator {
    pub id: usize,
    pub shard_id: usize,
    pub votes_cast: usize,  // number of votes the validator has cast
    pub successful_votes: usize,  // number of successful votes (aligned with the final outcome)
    pub average_response_time_ms: f64,  // average time taken to respond
    pub participation_count: usize, // number of consensus rounds participated in
    pub consensus_contribution_count: usize, // number of times aligned with consensus
    pub epochs_active: usize,  // number of epochs the validator has been active
    pub penalized: bool,       // whether the validator has been penalized
    pub final_vote_weight: f64, // the weight used in consensus
}

impl Validator {
    // New constructor with the final_vote_weight parameter
    pub fn new(id: usize, shard_id: usize, final_vote_weight: f64) -> Self {
        Validator {
            id,
            shard_id,
            votes_cast: 0,
            successful_votes: 0,
            average_response_time_ms: 0.0,
            participation_count: 0,
            consensus_contribution_count: 0,
            epochs_active: 1,  // start with 1 to prevent division by zero issues in the early epochs
            penalized: false,
            final_vote_weight,
        }
    }

    pub fn cast_vote(&mut self, is_successful: bool, response_time: u128, aligns_with_consensus: bool) {
        self.votes_cast += 1;

        if is_successful {
            self.successful_votes += 1;
        }

        self.average_response_time_ms =
            (self.average_response_time_ms * (self.votes_cast as f64 - 1.0) + response_time as f64)
            / self.votes_cast as f64;

        self.participation_count += 1;
        if aligns_with_consensus {
            self.consensus_contribution_count += 1;
        }
    }

    // this method now includes the final_vote_weight parameter, which will affect the outcome.
    pub fn get_final_vote_weight(&self, current_epoch: usize) -> f64 {
        // honesty Score: fraction of successful votes
        let honesty_score = if self.votes_cast == 0 { 
            1.0
        } else { 
            self.successful_votes as f64 / self.votes_cast as f64 
        };
    
        // time Weight: faster response times result in higher weight
        let time_weight = 1.0 / (self.average_response_time_ms + 1.0);  
    
        // participation Score: fraction of participation in consensus rounds
        let participation_score = if current_epoch == 0 { 
            1.0  // Default to 1.0 for the first epoch
        } else { 
            self.participation_count as f64 / current_epoch as f64 
        };
    
        // consensus contribution score: fraction of times the validator aligned with consensus
        let consensus_contribution_score = if self.participation_count == 0 { 
            1.0  // Default to 1.0 if no participation yet
        } else { 
            self.consensus_contribution_count as f64 / self.participation_count as f64 
        };
    
        // longevity score: fraction of epochs the validator has been active
        let longevity_score = if current_epoch == 0 { 
            1.0  // Default to 1.0 for the first epoch
        } else { 
            self.epochs_active as f64 / current_epoch as f64 
        };
    
        // decay factor: reduces weight for inactivity over multiple epochs
        let decay_factor = if current_epoch > self.epochs_active {
            let age = current_epoch - self.epochs_active;
            1.0 / (1.0 + (age as f64 * 0.1))  // decay for inactive epochs
        } else {
            1.0
        };
    
        // integrity Penalty: Reduce weight if penalized
        let integrity_penalty = if self.penalized { 
            0.5 
        } else { 
            1.0 
        };
    
        // apply final_vote_weight as a multiplier to the final weight calculation
        let final_weight = (self.final_vote_weight * honesty_score 
            * time_weight 
            * participation_score 
            * consensus_contribution_score 
            * longevity_score 
            * decay_factor 
            * integrity_penalty)
            .max(0.3); // Ensure a minimum weight of 0.3 in early epochs
    
        println!(
            "Validator {} (Shard {}): Honesty: {:.2}, Time: {:.2}, Participation: {:.2}, Consensus: {:.2}, Longevity: {:.2}, Decay: {:.2}, Final Weight: {:.2}",
            self.id, self.shard_id, honesty_score, time_weight, participation_score, consensus_contribution_score, longevity_score, decay_factor, final_weight
        );
    
        final_weight
    }

    pub fn validate_transaction(&self, transaction_id: &str, current_epoch: usize) -> bool {
        println!(
            "Validator {} (Shard {}) is validating transaction {} for epoch {}",
            self.id, self.shard_id, transaction_id, current_epoch
        );
    
        let final_weight = self.get_final_vote_weight(current_epoch);
    
        // lower threshold for early epochs
        let validation_threshold = if current_epoch < 5 { 0.3 } else { 0.5 };
    
        final_weight > validation_threshold
    }
}

#[derive(Debug, Clone)]
pub struct ValidatorPerformance {
    pub id: usize,
    pub honesty_score: f64,
    pub response_time: f64,
}

impl ValidatorPerformance {
    pub fn from_validator(validator: &Validator) -> Self {
        ValidatorPerformance {
            id: validator.id,
            honesty_score: validator.get_final_vote_weight(1),  // use epoch 1 for now
            response_time: validator.average_response_time_ms,
        }
    }
}
