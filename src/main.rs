mod poh;

use poh::generator::PohGenerator;

fn main() {
    let mut poh_gen = PohGenerator::new();

    for i in 1..=5 {
        let data = format!("Transaction {}", i);
        let entry = poh_gen.generate_entry(&data);
        println!("PoH Entry {}: {:?}", i, entry);
    }
}
