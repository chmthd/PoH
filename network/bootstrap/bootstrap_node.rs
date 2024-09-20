use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

pub struct BootstrapNode {
    nodes: Arc<Mutex<HashMap<String, String>>>,
}

impl BootstrapNode {
    pub fn new() -> Self {
        BootstrapNode {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, port: u16) {
        let listener = TcpListener::bind(("0.0.0.0", port)).unwrap();
        println!("Bootstrap node started on port {}", port);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let nodes = Arc::clone(&self.nodes);
                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, nodes) {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Error accepting connection: {}", e),
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, nodes: Arc<Mutex<HashMap<String, String>>>) -> std::io::Result<()> {
    let mut buffer = [0; 512];
    let bytes_read = stream.read(&mut buffer)?;

    let message = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("Received registration message: {}", message);

    let parts: Vec<&str> = message.trim().split(',').collect();
    if parts.len() == 2 {
        let node_id = parts[0].to_string();
        let addr = parts[1].to_string();

        let mut nodes = nodes.lock().unwrap();
        nodes.insert(node_id.clone(), addr.clone());

        println!("Registered node {}: {}. Current nodes in the network:", node_id, addr);
        for (id, addr) in &*nodes {
            println!("  - {}: {}", id, addr);
        }

        let response: String = nodes
            .iter()
            .map(|(id, addr)| format!("{},{}", id, addr))
            .collect::<Vec<String>>()
            .join("\n");
        stream.write_all(response.as_bytes())?;
    } else {
        println!("Invalid registration message received: {}", message);
        stream.write_all(b"Invalid registration message")?;
    }

    Ok(())
}
