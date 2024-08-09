mod poh;

use poh::generator::PohGenerator;

fn main() {
    let mut poh_gen = PohGenerator::new(3); 

    let transactions = vec![
        "Transaction 1".to_string(),
        "Transaction 2".to_string(),
        "Transaction 3".to_string(),
        "Transaction 4".to_string(),
        "Transaction 5".to_string(),
    ];

    match poh_gen.generate_entries(transactions) {
        Ok(entries) => {
            for (i, entry) in entries.iter().enumerate() {
                println!("PoH Entry {}: {:?}", i + 1, entry);
            }
        }
        Err(e) => {
            println!("Error generating PoH entries: {}", e);
        }
    }
}
