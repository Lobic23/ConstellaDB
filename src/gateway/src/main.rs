use tokio::net::{TcpStream, TcpListener};
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Gets the local ip of the machine
fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

/// Commandline args for the node
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  client_port: Option<u32>,

  #[arg(short, long)]
  node_port: Option<u32>,
}

struct State {
}

impl State {
  pub fn new() -> Self {
    Self {
    }
  }
}

async fn start_client_listener(state: Arc<Mutex<State>>, client_port: u32) {
  let ip = get_local_ip().unwrap();

  // Listener for external clients
  let client_listener = TcpListener::bind(
    format!("{}:{}", ip, client_port)
  ).await.unwrap();
  let client_bound_port = client_listener.local_addr().unwrap().port();
  let client_listener_full_ip = format!("{}:{}", ip, client_bound_port);
  println!("[LOG] Listening for clients on {}", &client_listener_full_ip);

  loop {
    let (stream, addr) = client_listener.accept().await.unwrap();
    println!("[LOG] Client {} connected", addr);
  };
}

async fn start_node_listener(state: Arc<Mutex<State>>, node_port: u32) {
  let ip = get_local_ip().unwrap();

  // Listener for internal nodes
  let node_listener = TcpListener::bind(
    format!("{}:{}", ip, node_port)
  ).await.unwrap();
  let node_bound_port = node_listener.local_addr().unwrap().port();
  let node_listener_full_ip = format!("{}:{}", ip, node_bound_port);
  println!("[LOG] Listening for nodes on {}", &node_listener_full_ip);

  loop {
    let (stream, addr) = node_listener.accept().await.unwrap();
    println!("[LOG] Node {} connected", addr);
  };
}

async fn start_listener(state: Arc<Mutex<State>>, client_port: u32, node_port: u32) {
  let client_state = state.clone();

  let client_handle = tokio::spawn(async move {
    start_client_listener(client_state, client_port).await;
  });

  let node_handle = tokio::spawn(async move {
    start_node_listener(state, node_port).await;
  });

  let _ = tokio::join!(client_handle, node_handle);
}

#[tokio::main]
async fn main() {
  let args = Args::parse();
  let state = Arc::new(Mutex::new(State::new()));

  // Extract port from args
  let mut client_port = 0;
  if let Some(p) = args.client_port {
    client_port = p;
  }

  let mut node_port = 0;
  if let Some(p) = args.node_port {
    node_port = p;
  }

  start_listener(state, client_port, node_port).await;
}
