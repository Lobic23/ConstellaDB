mod job;
mod state;
mod listener;

use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

use state::ServiceState;
use listener::start_listener;

/// Commandline args for the job service
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  port: Option<u32>,

  #[arg(short, long)]
  threads: Option<usize>,

  #[arg(short, long)]
  query_service: String,
}

#[tokio::main]
async fn main() {
  let state = Arc::new(Mutex::new(ServiceState::new()));

  let args = Args::parse();

  // Extract port from args
  let mut port = 0;
  if let Some(p) = args.port {
    port = p;
  }

  // Extract thread count from args
  if let Some(t) = args.threads {
    let mut s = state.lock().await;
    if t > s.max_threads {
      println!("[ERROR] Your machine only supports {} threads.", s.max_threads);
      return;
    }
    s.max_threads = t;
  }

  // Saving the query service ip
  {
    let query_service_ip = args.query_service;
    let mut s = state.lock().await;
    s.query_service_ip = query_service_ip;
  }

  start_listener(state, port).await;
}
