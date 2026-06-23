pub mod node;

use uuid::Uuid;
use tokio::net::{TcpListener, TcpStream};
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::{
  handler::ProtocolHandler,
  message::{MessageType, Message},
  serializer::BincodeSerializer,
};
use node::Node;

/// Commandline args for the node
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  leader: bool,

  #[arg(short, long)]
  followers: Option<Vec<String>>,       // IP's of the followers
}

/// Ran when node is a leader
/// Connects to the follower's server as a client and stores the stream
async fn connect_to_followers(node: Arc<Mutex<Node>>, ips: Vec<String>) {
  let mut n = node.lock().await;

  for follower_ip in ips {
    let stream = TcpStream::connect(&follower_ip).await.unwrap();
    let handler = Arc::new(Mutex::new(
      ProtocolHandler::new(stream, Box::new(BincodeSerializer))
    ));

    println!("[LOG] Connected to follower {}", &follower_ip);
    n.followers.insert(follower_ip, handler);
  }
}

/// Distribute the message to the followers
/// TODO(slok): For now this distributes the message to every follower.
/// Later we need to develop on how the message should be distributed between
/// followers so that the efficiency is maximized
async fn distribute_message(msg: &Message, node: Arc<Mutex<Node>>) {
  let followers = {
    let n = node.lock().await;
    n.followers.clone()
  };

  for (ip, handler) in followers {
    println!("Send: {:?} to {}", msg, ip);

    let mut handler = handler.lock().await;
    handler.send(msg).await;
  }
}

/// Listens to every protocol sent into this stream
/// Here the protocol is interpreted
async fn connection_listener(stream: TcpStream, node: Arc<Mutex<Node>>) {
  let mut handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));
  loop {
    let received = handler.receive().await;
    if let Err(e) = received {
      println!("[LOG] Connection lost due to: {}", e);
      break;
    }

    let msg = received.unwrap();
    match msg.msg_type {
      MessageType::Query => {

        // Extracting the is_leader so that lock is dropped here
        let is_leader = {
          let n = node.lock().await;
          n.leader
        };

        if is_leader {
          distribute_message(&msg, node.clone()).await;
        } else {
          let sql = match String::from_utf8(msg.payload) {
            Ok(s) => s,
            Err(_) => {
              println!("[LOG] invalid UTF-8 payload");
              continue;
            }
          };
          println!("{}: {}", msg.id, sql);
        }
      },
      MessageType::Response=> { },
      MessageType::Heartbeat => { },
      MessageType::Sync => { },
      MessageType::Error => { },
    }
  }
}

async fn start_listener(node: Arc<Mutex<Node>>) {
  let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();

  let addr = listener.local_addr().unwrap();
  let node_id = Uuid::new_v4();
  println!("Node [{}] listening on {}", node_id, addr);

  loop {
    let (stream, addr) = listener.accept().await.unwrap();
    println!("[LOG] {} connected", addr);

    let n = node.clone();
    tokio::spawn(async move { connection_listener(stream, n).await; });
  }
}

#[tokio::main]
async fn main() {
  let node = Arc::new(Mutex::new(Node::new()));

  let args = Args::parse();
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

  start_listener(node).await;
}
