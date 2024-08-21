// main.rs

mod poh;
mod block;
mod shard;
mod network;
mod validator;

use actix_files::NamedFile;
use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use serde::Serialize;
use shard::shard::{Shard, Transaction, TransactionStatus};
use network::gossip_protocol::GossipProtocol;
use crate::validator::validator::Validator;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;

const MAX_TRANSACTIONS_PER_BLOCK: usize = 100;
const NUM_VALIDATORS: usize = 3; 

#[derive(Serialize)]
struct TransactionDetail {
    id: String,
    status: String,
    processing_time_ms: Option<u128>, 
}

#[derive(Serialize)]
struct ShardStats {
    id: usize,
    transaction_pool_size: usize,
    transactions: Vec<TransactionDetail>,
}

#[derive(Serialize)]
struct NetworkStats {
    num_shards: usize,
    total_blocks: usize,
    total_transactions: usize,
    avg_block_size: usize,
    transaction_pool_size: usize,
    total_cross_shard_transactions: usize,
    shard_stats: Vec<ShardStats>, 
    avg_tx_confirmation_time_ms: Option<u128>, 
    avg_tx_size: usize,
}

struct AppState {
    shards: Arc<Mutex<Vec<Shard>>>,
    transaction_start_times: Arc<Mutex<HashMap<String, Instant>>>, 
}

#[get("/api/stats")]
async fn get_stats(data: web::Data<AppState>) -> impl Responder {
    let shards = data.shards.lock().unwrap();
    let tx_start_times = data.transaction_start_times.lock().unwrap();
    
    let total_blocks: usize = shards.iter().map(|shard| shard.blocks.len()).sum();
    let total_transactions: usize = shards.iter().map(|shard| shard.get_processed_transactions().len()).sum();
    
    let total_block_size: usize = shards.iter()
        .flat_map(|shard| shard.blocks.iter())
        .map(|block| std::mem::size_of_val(block))
        .sum();
    
    let avg_block_size = if total_blocks > 0 {
        total_block_size / total_blocks
    } else {
        0
    };

    let total_tx_size: usize = shards.iter()
        .flat_map(|shard| shard.get_processed_transactions())
        .map(|tx| std::mem::size_of_val(tx))
        .sum();
    
    let avg_tx_size = if total_transactions > 0 {
        total_tx_size / total_transactions
    } else {
        0
    };

    let transaction_pool_size: usize = shards.iter().map(|shard| shard.get_transaction_pool().len()).sum();
    let total_cross_shard_transactions: usize = shards.iter().map(|shard| shard.get_pending_cross_shard_txs_len()).sum();

    let mut shard_stats = Vec::new();
    let mut total_confirmation_time: u128 = 0;
    let mut confirmed_tx_count: usize = 0;

    for shard in shards.iter() {
        let mut transactions = Vec::new();
        for tx in shard.get_transaction_pool() {
            let tx_id = &tx.id;
            let status = format!("{:?}", tx.status);
            let processing_time_ms = match tx.status {
                TransactionStatus::Completed => {
                    if let Some(start_time) = tx_start_times.get(tx_id) {
                        let duration = start_time.elapsed().as_millis();
                        total_confirmation_time += duration;
                        confirmed_tx_count += 1;
                        Some(duration)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            transactions.push(TransactionDetail {
                id: tx_id.clone(),
                status,
                processing_time_ms,
            });
        }

        shard_stats.push(ShardStats {
            id: shard.id,
            transaction_pool_size: shard.get_transaction_pool().len(),
            transactions,
        });
    }

    let avg_tx_confirmation_time_ms = if confirmed_tx_count > 0 {
        Some(total_confirmation_time / confirmed_tx_count as u128)
    } else {
        None
    };

    let stats = NetworkStats {
        num_shards: shards.len(),
        total_blocks,
        total_transactions,
        avg_block_size,
        transaction_pool_size,
        total_cross_shard_transactions,
        shard_stats,
        avg_tx_confirmation_time_ms,
        avg_tx_size, 
    };

    HttpResponse::Ok().json(stats)
}

async fn index() -> impl Responder {
    NamedFile::open("./static/index.html").unwrap()
}

fn send_random_transactions(shards: Arc<Mutex<Vec<Shard>>>, gossip_protocol: Arc<Mutex<GossipProtocol>>, tx_start_times: Arc<Mutex<HashMap<String, Instant>>>) {
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut tx_count = 1;
        let mut block_start_time = Instant::now();
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("blockchain_metrics.log")
            .expect("Cannot open log file");

        loop {
            let amount = rng.gen_range(1..1000);
            let shard_index = rng.gen_range(0..shards.lock().unwrap().len());

            let transaction_id = format!("tx{}", tx_count);
            let to_shard = hash_to_shard(&transaction_id, shards.lock().unwrap().len());

            let transaction = Transaction {
                id: transaction_id.clone(),
                amount,
                from_shard: shard_index + 1,
                to_shard,
                status: TransactionStatus::Pending,
            };

            tx_start_times.lock().unwrap().insert(transaction_id.clone(), Instant::now());

            println!(
                "Sending Transaction {} from Shard {} to Shard {} (Status: {:?})",
                transaction.id, transaction.from_shard, transaction.to_shard, transaction.status
            );

            {
                let mut shards = shards.lock().unwrap();
                if transaction.from_shard == transaction.to_shard {
                    shards[shard_index].process_transactions(vec![transaction.clone()]);
                } else {
                    shards[shard_index].add_pending_cross_shard_tx(transaction.clone());
                }
            }

            gossip_protocol.lock().unwrap().gossip(&mut shards.lock().unwrap());

            // logging
            {
                let shards = shards.lock().unwrap();
                if shards[shard_index].blocks.len() > tx_count / MAX_TRANSACTIONS_PER_BLOCK {
                    let block_gen_time = block_start_time.elapsed().as_secs_f64();
                    let last_block = shards[shard_index].blocks.last().unwrap();
                    let block_size = std::mem::size_of_val(&last_block);
                    let tx_count_in_block = last_block.poh_entries.len();
                    let log_message = format!(
                        "Shard {}: Block {} Generated: Time: {:.2} seconds, Transactions: {}, Block Size: {} bytes\n",
                        shard_index + 1,
                        last_block.block_number,
                        block_gen_time,
                        tx_count_in_block,
                        block_size
                    );

                    log_file
                        .write_all(log_message.as_bytes())
                        .expect("Failed to write to log file");

                    block_start_time = Instant::now();
                }
            }

            tx_count += 1;
            thread::sleep(Duration::from_secs(1));

            if tx_count % 5 == 0 {
                gossip_protocol.lock().unwrap().periodic_gossip(&mut shards.lock().unwrap());
            }
        }
    });
}

fn hash_to_shard(target: &str, shard_count: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    (hasher.finish() as usize % shard_count) + 1
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut shards = Vec::new();

    for i in 1..=4 {
        let mut validators = Vec::new();
        for j in 1..=NUM_VALIDATORS {
            validators.push(Validator::new(j, i)); // create validators with shard_id
        }
        shards.push(Shard::new(i, 100, MAX_TRANSACTIONS_PER_BLOCK, validators));
    }

    let shards = Arc::new(Mutex::new(shards));

    let gossip_protocol = Arc::new(Mutex::new(GossipProtocol::new()));
    let transaction_start_times = Arc::new(Mutex::new(HashMap::new()));

    let app_state = web::Data::new(AppState { 
        shards: Arc::clone(&shards),
        transaction_start_times: Arc::clone(&transaction_start_times),
    });

    send_random_transactions(
        Arc::clone(&shards),
        Arc::clone(&gossip_protocol),
        Arc::clone(&transaction_start_times),
    );

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_stats)
            .route("/", web::get().to(index))
            .service(actix_files::Files::new("/static", "./static").show_files_listing())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
