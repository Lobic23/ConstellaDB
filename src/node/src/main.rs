// TODOs:
// [] Make leader do work too
// [] Proper distribution algorithm
// [] Backup

pub mod node;
pub mod leader;
pub mod listener;
pub mod instruction;

use tokio::net::TcpStream;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  serializer::BincodeSerializer,
};

use node::Node;
use leader::connect_to_followers;
use listener::{start_listener, connection_listener};

/// Commandline args for the node
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  port: Option<u32>,

  #[arg(short, long)]
  leader: bool,

  #[arg(short, long, num_args=1..)]
  followers: Option<Vec<String>>,       // IP's of the followers

  #[arg(short, long)]
  job_service: String,
}

/// Connects to the job scheduling service and runs the listener for
/// the job service responses
async fn connect_to_job_service(node: Arc<Mutex<Node>>, job_service_ip: &str) {
  let stream = TcpStream::connect(job_service_ip).await.unwrap();
  let (reader, writer) = stream.into_split();
  let read_handler = Arc::new(Mutex::new(
    ReadHandler::new(reader, Box::new(BincodeSerializer))
  ));
  let write_handler = Arc::new(Mutex::new(
    WriteHandler::new(writer, Box::new(BincodeSerializer))
  ));
  {
    let mut n = node.lock().await;
    n.job_service = Some((read_handler.clone(), write_handler.clone()));
  }

  // Spawn the listener for the job service
  let x = node.clone();
  tokio::spawn(async move {
    connection_listener(read_handler, write_handler, x).await;
  });
}

#[tokio::main]
async fn main() {
  let node = Arc::new(Mutex::new(Node::new()));

  let args = Args::parse();

  // Extract port from args
  let mut port = 0;
  if let Some(p) = args.port {
    port = p;
  }

  if args.leader {
    {
      let mut n = node.lock().await;
      n.leader = true;
    }

    if let Some(followers) = args.followers {
      connect_to_followers(node.clone(), followers).await;
    } else {
      println!("No followers specified");
      return;
    }
  }

  // Connecting to the job service
  let job_service_ip = args.job_service;
  connect_to_job_service(node.clone(), &job_service_ip).await;

  start_listener(node, port).await;
}
