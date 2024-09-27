use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::File;

pub struct BootstrapNode {
    nodes: Arc<Mutex<HashMap<String, String>>>, // Stores registered nodes and their IP addresses
}

impl BootstrapNode {
    // Create a new instance of the BootstrapNode
    pub fn new() -> Self {
        BootstrapNode {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Start listening on the provided port and IP for incoming node registrations
    pub fn start(&self, port: u16, bootstrap_ip: &str) {
        // Bind to the provided IP and port
        let listener = TcpListener::bind((bootstrap_ip, port)).expect("Failed to bind to address");
        println!("Bootstrap node started on {}:{}", bootstrap_ip, port);

        // Set the listener to non-blocking mode
        listener.set_nonblocking(true).expect("Cannot set non-blocking");

        let timeout = Duration::from_secs(300); // 5 minutes timeout
        let start_time = Instant::now();

        while start_time.elapsed() < timeout {
            match listener.accept() {
                Ok((stream, _)) => {
                    let nodes = Arc::clone(&self.nodes);
                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, nodes) {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No incoming connection, sleep for a short while
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => eprintln!("Error accepting connection: {}", e),
            }

            // Check if we have registered nodes
            if self.nodes.lock().unwrap().len() > 0 {
                break; // Exit the loop if we have at least one registered node
            }
        }

        println!("Bootstrap node setup complete. Ready for connections.");
        
        // Signal that the bootstrap node setup is complete
        File::create("/tmp/bootstrap_complete").expect("Failed to create bootstrap complete signal file");
    }
}

// Handles individual node registration
fn handle_connection(mut stream: TcpStream, nodes: Arc<Mutex<HashMap<String, String>>>) -> std::io::Result<()> {
    let mut buffer = [0; 512]; // Buffer to read incoming data
    let bytes_read = stream.read(&mut buffer)?;

    let message = String::from_utf8_lossy(&buffer[..bytes_read]); // Convert bytes to string
    println!("Received registration message: {}", message);

    // Split the received message into node_id and its corresponding IP address
    let parts: Vec<&str> = message.trim().split(',').collect();
    if parts.len() == 2 {
        let node_id = parts[0].to_string();
        let addr = parts[1].to_string();

        // Lock the nodes HashMap and register the new node
        let mut nodes = nodes.lock().unwrap();
        nodes.insert(node_id.clone(), addr.clone());

        // Print the current registered nodes in the network
        println!("Registered node {}: {}. Current nodes in the network:", node_id, addr);
        for (id, addr) in &*nodes {
            println!("  - {}: {}", id, addr);
        }

        // Send back the list of all known nodes to the new node
        let response: String = nodes
            .iter()
            .map(|(id, addr)| format!("{},{}", id, addr))
            .collect::<Vec<String>>()
            .join("\n");
        stream.write_all(response.as_bytes())?; // Send the response
    } else {
        // Handle invalid registration messages
        println!("Invalid registration message received: {}", message);
        stream.write_all(b"Invalid registration message")?;
    }

    Ok(())
}