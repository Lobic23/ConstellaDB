use std::collections::HashMap;
use tokio::net::{ TcpListener};
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

use constella_db::modules::protocol::{
  handler::{ReadHandler, WriteHandler},
  message::{MessageType, Message},
  serializer::BincodeSerializer,
};
use constella_db::modules::cmd::parse_cmd;

#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  ip: String,

  #[arg(short, long)]
  client_port: u32,

  #[arg(short, long)]
  node_port: u32,
}

struct State {
  pub nodes: HashMap<
    String,
    (Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)
  >,
  pub leader: Option<String>,
  pub requests: HashMap<String, Arc<Mutex<WriteHandler>>>,
}

impl State {
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      leader: None,
      requests: HashMap::new(),
    }
  }
}

/// Handles the client to gateway communication
async fn handle_client_connection(
  state: Arc<Mutex<State>>,
  read_handler_mutex: Arc<Mutex<ReadHandler>>,
  write_handler_mutex: Arc<Mutex<WriteHandler>>
) {
  loop {
    let received = {
      let mut handler = read_handler_mutex.lock().await;
      handler.receive().await
    };

    if let Err(e) = received {
      println!("[LOG] Client connection lost due to: {}", e);
      break;
    }

    let msg = received.unwrap();
    match &msg.msg_type {

      MessageType::Query => {
        let query = String::from_utf8(msg.payload).unwrap();
        match parse_cmd(&query) {
          Ok(cmd) => {
            let mut s = state.lock().await;
            let l = s.leader.clone().expect("Leader is not found");

            // Now the followers will include leader as well
            // to make leader work identical to the follower
            let followers: Vec<String> = s.nodes
              .keys()
              .cloned()
              .collect();

            // Send the leader the lead instruction
            if let Some((_, writer_mutex)) = s.nodes.get(&l) {
              let mut writer = writer_mutex.lock().await;
              let msg_to_leader = Message::new(
                msg.id.clone(),
                MessageType::Lead { followers: followers },
                "gateway".to_string()
              )
                .with_command(cmd);
              writer.send(&msg_to_leader).await.unwrap();

              println!("[LOG] Sent:\n{:#?}", msg_to_leader);
            }

            // Store the request
            s.requests.insert(msg.id.clone(), write_handler_mutex.clone());
          },
          Err(e) => {
            let response = Message::new(
              "".to_string(),
              MessageType::Response {
                sucess: false,
                message: Some(e.to_string()),
                data: None
              },
              "".to_string()
            );

            let mut w = write_handler_mutex.lock().await;
            w.send(&response).await.unwrap();
            continue;
          }
        }
      },

      _ => println!("Unexpected!"),
    }
  }
}

/// Handles the node to gateway communication
async fn handle_node_connection(
  state: Arc<Mutex<State>>,
  read_handler_mutex: Arc<Mutex<ReadHandler>>,
  write_handler_mutex: Arc<Mutex<WriteHandler>>
) {
  let mut node_id: Option<String> = None;

  loop {
    let received = {
      let mut handler = read_handler_mutex.lock().await;
      handler.receive().await
    };

    // Remove the node when connection is closed
    if let Err(e) = received {
      println!("[LOG] Node connection lost due to: {}", e);

      if let Some(id) = node_id {
        let mut s = state.lock().await;

        // Remove disconnected node
        s.nodes.remove(&id);

        // If disconnected node was leader
        if s.leader.as_ref() == Some(&id) {
          println!("[LOG] Leader {} removed", id);

          // Choose a new leader from remaining nodes
          s.leader = s.nodes
            .keys()
            .next()
            .cloned();

          if let Some(new_leader) = &s.leader {
            println!("[LOG] New leader elected: {}", new_leader);
          } else {
            println!("[LOG] No nodes available for leader");
          }
        }
        println!("[LOG] Removed node {}", id);
      }
      break;
    }

    let msg = received.unwrap();
    match &msg.msg_type {

      // Store the connected node's read and write handler
      MessageType::Register => {
        let mut s = state.lock().await;
        s.nodes.insert(
          msg.node_id.clone(),
          (read_handler_mutex.clone(), write_handler_mutex.clone())
        );
        println!("[LOG] Node {} has been registered.", msg.node_id);

        node_id = Some(msg.node_id.clone());

        // NOTE: Leader is assigned from FCFS
        if s.leader.is_none() {
          s.leader = Some(msg.node_id.clone());
        }
      },

      MessageType::Response {..} => {
        let mut s = state.lock().await;
        if let Some(writer_mutex) = s.requests.get(&msg.id) {
          let mut writer = writer_mutex.lock().await;
          writer.send(&msg).await.unwrap();
        }
        s.requests.remove(&msg.id);
      }

      _ => {println!("Unexpected!");},
    }
  }
}

async fn start_client_listener(state: Arc<Mutex<State>>, ip: &str, client_port: u32) {
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

    let s = state.clone();

    let (reader, writer) = stream.into_split();
    let read_handler = Arc::new(Mutex::new(
      ReadHandler::new(reader, Box::new(BincodeSerializer))
    ));
    let write_handler = Arc::new(Mutex::new(
      WriteHandler::new(writer, Box::new(BincodeSerializer))
    ));

    tokio::spawn(async move {
      handle_client_connection(s, read_handler, write_handler).await;
    });
  };
}

async fn start_node_listener(state: Arc<Mutex<State>>, ip: &str, node_port: u32) {
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

    let s = state.clone();

    let (reader, writer) = stream.into_split();
    let read_handler = Arc::new(Mutex::new(
      ReadHandler::new(reader, Box::new(BincodeSerializer))
    ));
    let write_handler = Arc::new(Mutex::new(
      WriteHandler::new(writer, Box::new(BincodeSerializer))
    ));

    tokio::spawn(async move {
      handle_node_connection(s, read_handler, write_handler).await;
    });
  };
}

#[tokio::main]
async fn main() {
  let args = Args::parse();
  let state = Arc::new(Mutex::new(State::new()));

  tokio::select! {
    _ = start_client_listener(state.clone(), &args.ip, args.client_port) => {},
    _ = start_node_listener(state.clone(), &args.ip, args.node_port) => {},
  }
}
