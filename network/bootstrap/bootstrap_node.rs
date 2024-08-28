use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

pub struct BootstrapNode {
    nodes: Arc<Mutex<HashMap<String, String>>>, // store node_id -> ip:port
}

impl BootstrapNode {
    pub fn new() -> Self {
        BootstrapNode {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, port: u16) {
        let listener = TcpListener::bind(("0.0.0.0", port)).unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let nodes = Arc::clone(&self.nodes);
            thread::spawn(move || {
                handle_connection(stream, nodes);
            });
        }
    }
}

fn handle_connection(mut stream: TcpStream, nodes: Arc<Mutex<HashMap<String, String>>>) {
    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();

    // extract node id and address
    let message = String::from_utf8_lossy(&buffer[..]);
    let parts: Vec<&str> = message.trim().split(',').collect();
    if parts.len() == 2 {
        let node_id = parts[0].to_string();
        let addr = parts[1].to_string();

        let mut nodes = nodes.lock().unwrap();
        nodes.insert(node_id.clone(), addr.clone());

        println!("Registered node {}: {}", node_id, addr);

        // respond with the list of nodes
        let response: String = nodes
            .iter()
            .map(|(id, addr)| format!("{},{}", id, addr))
            .collect::<Vec<String>>()
            .join("\n");
        stream.write(response.as_bytes()).unwrap();
    }
}
