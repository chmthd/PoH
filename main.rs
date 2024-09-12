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
use network::bootstrap::bootstrap_node::BootstrapNode;
use crate::validator::validator::Validator;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use std::fs::OpenOptions;
use std::io::{Write, Read};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::env;
use std::net::TcpStream;
use local_ip_address;

const MAX_TRANSACTIONS_PER_BLOCK: usize = 10;

#[derive(Serialize, Clone)]
struct ShardInfo {
    id: usize,
    ip: String,
    port: u16,
}

#[derive(Serialize)]
struct TransactionDetail {
    id: String,
    status: String,
    processing_time_ms: Option<u128>,
    block_number: String,
    shard_number: usize,
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
    shard_info: Vec<ShardInfo>,
}

struct AppState {
    shards: Arc<Mutex<Vec<Shard>>>,
    transaction_start_times: Arc<Mutex<HashMap<String, Instant>>>,
    shard_info: Vec<ShardInfo>,
    nodes: Arc<Mutex<HashMap<String, String>>>,
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

        for block in &shard.blocks {
            for tx in &block.poh_entries[0].transactions {
                let status = String::from("Completed");
                let block_number = block.block_number.to_string();
                let shard_number = shard.id;
                let processing_time_ms = tx_start_times.get(tx).map(|start_time| start_time.elapsed().as_millis());

                transactions.push(TransactionDetail {
                    id: tx.clone(),
                    status,
                    processing_time_ms,
                    block_number,
                    shard_number,
                });

                if let Some(duration) = processing_time_ms {
                    total_confirmation_time += duration;
                    confirmed_tx_count += 1;
                }
            }
        }

        for tx in shard.get_transaction_pool() {
            let tx_id = &tx.id;
            let status = format!("{:?}", tx.status);
            let shard_number = shard.id;
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
            let block_number = if tx.status == TransactionStatus::Processing {
                "Processing".to_string()
            } else {
                "Unassigned".to_string()
            };

            transactions.push(TransactionDetail {
                id: tx_id.clone(),
                status,
                processing_time_ms,
                block_number,
                shard_number,
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
        shard_info: data.shard_info.clone(),
    };

    HttpResponse::Ok().json(stats)
}

#[get("/api/nodes")]
async fn get_nodes(data: web::Data<AppState>) -> impl Responder {
    let nodes = data.nodes.lock().unwrap();
    let nodes_list: Vec<(String, String)> = nodes.clone().into_iter().collect();
    HttpResponse::Ok().json(nodes_list)
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

fn register_with_bootstrap(bootstrap_address: &str, bootstrap_port: u16, node_id: &str) -> Result<Vec<String>, std::io::Error> {
    let addr = format!("{}:{}", bootstrap_address, bootstrap_port);
    let mut stream = TcpStream::connect(addr)?;
    
    let registration_message = format!("{},{}", node_id, node_id);
    stream.write_all(registration_message.as_bytes())?;
    
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    
    println!("Registration response: {}", response);
    
    // Ensure that only valid node addresses are processed
    let nodes: Vec<String> = response
        .lines()
        .filter(|line| line.contains(':')) // Ensure it's an address line
        .map(String::from)
        .collect();
    
    Ok(nodes)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Parse command-line arguments or configuration
    let args: Vec<String> = env::args().collect();
    let port = args.get(1).map(|p| p.parse::<u16>().unwrap_or(8080)).unwrap_or(8080);
    let bootstrap_port = args.get(2).map(|p| p.parse::<u16>().unwrap_or(8081)).unwrap_or(8081);

    // Define the number of validators dynamically
    let num_validators = args.get(3).map(|v| v.parse::<usize>().unwrap_or(2)).unwrap_or(2);

    // Define dynamic final vote weights for validators
    let final_vote_weight_config: Vec<f64> = vec![0.9, 0.9, 0.9, 0.9]; 

    // Start bootstrap node server if specified
    let nodes = Arc::new(Mutex::new(HashMap::new()));

    if port == bootstrap_port {
        let bootstrap_node = BootstrapNode::new();
        thread::spawn(move || {
            bootstrap_node.start(bootstrap_port);
        });
        println!("Bootstrap node is setting up. Please wait...");
        thread::sleep(Duration::from_secs(5));
        println!("Bootstrap node setup complete. You can now add nodes to the network.");
    } else {
        println!("Waiting for bootstrap node to complete setup...");
        thread::sleep(Duration::from_secs(6));
    }

    // Get IP address of the node
    let ip_address = local_ip_address::local_ip().unwrap_or_else(|_| "127.0.0.1".parse().unwrap()).to_string();

    let mut shards = Vec::new();
    let mut shard_infos = Vec::new(); 

    // Loop through shards and validators
    for i in 1..=5 {
        let mut validators = Vec::new();
        for j in 1..=num_validators {
            let final_vote_weight = final_vote_weight_config[j % final_vote_weight_config.len()]; // Assign weights dynamically
            validators.push(Validator::new(j, i, final_vote_weight));  
        }
        shards.push(Shard::new(i, 100, MAX_TRANSACTIONS_PER_BLOCK, validators));

        // Collect shard info
        shard_infos.push(ShardInfo {
            id: i,
            ip: ip_address.clone(),
            port: port + i as u16, // Assume each shard has a unique port offset by its id for now
        });
    }

    let shards = Arc::new(Mutex::new(shards));

    // Register the node with the bootstrap node
    let node_id = format!("{}:{}", ip_address, port);
    let known_nodes = match register_with_bootstrap(&ip_address, bootstrap_port, &node_id) {
        Ok(nodes) => {
            println!("Successfully registered with bootstrap node. Known nodes: {:?}", nodes);
            nodes
        }
        Err(e) => {
            eprintln!("Failed to register with bootstrap node: {}", e);
            Vec::new()
        }
    };

    let gossip_protocol = Arc::new(Mutex::new(GossipProtocol::new()));
    let transaction_start_times = Arc::new(Mutex::new(HashMap::new()));

    let app_state = web::Data::new(AppState { 
        shards: Arc::clone(&shards),
        transaction_start_times: Arc::clone(&transaction_start_times),
        shard_info: shard_infos,
        nodes: Arc::clone(&nodes),
    });

    // Establish connections with other known nodes
    for known_node in known_nodes {
        println!("Connecting to known node: {}", known_node);
        //
    }

    send_random_transactions(
        Arc::clone(&shards),
        Arc::clone(&gossip_protocol),
        Arc::clone(&transaction_start_times),
    );

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_stats)
            .service(get_nodes)
            .route("/", web::get().to(index))
            .service(actix_files::Files::new("/static", "./static").show_files_listing())
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
