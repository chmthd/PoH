use crate::poh::entry::PohEntry;

pub struct PohGenerator {
    pub previous_hash: String,
}

impl PohGenerator {
    pub fn new() -> Self {
        PohGenerator {
            previous_hash: String::from("0"),
        }
    }

    pub fn generate_entry(&mut self, data: &str) -> PohEntry {
        let entry = PohEntry::new(data, &self.previous_hash);
        self.previous_hash = entry.hash.clone();
        entry
    }
}