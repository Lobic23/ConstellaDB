use cmd_module::{execute, parse_cmd};
use db_module::Engine;
use protocol_module::{
  handler::ProtocolHandler,
  message::{Message, MessageType},
  serializer::BincodeSerializer,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

const LEADER_BIND_ADDR: &str = "0.0.0.0:7878";
const LEADER_CONNECT_ADDR: &str = "127.0.0.1:7878";
const CLIENT_BIND_ADDR: &str = "0.0.0.0:7979";

pub struct Node {
  pub leader: bool,
  pub db_engine: Arc<Mutex<Engine>>,
  pub listener: TcpListener,
  pub followers: Arc<Mutex<HashMap<String, ProtocolHandler>>>,
  pub stream: Option<TcpStream>,
}

impl Node {
  pub async fn new(leader: bool) -> Arc<Mutex<Self>> {
    let listener = if leader {
      TcpListener::bind(LEADER_BIND_ADDR).await.unwrap()
    } else {
      TcpListener::bind("0.0.0.0:0").await.unwrap()
    };

    let stream = if !leader {
      Some(TcpStream::connect(LEADER_CONNECT_ADDR).await.unwrap())
    } else {
      None
    };

    Arc::new(Mutex::new(Self {
      leader,
      db_engine: Arc::new(Mutex::new(Engine::new())),
      listener,
      followers: Arc::new(Mutex::new(HashMap::new())),
      stream,
    }))
  }

  pub async fn run(node: Arc<Mutex<Self>>) {
    let leader = {
      let node = node.lock().await;
      node.leader
    };

    if leader {
      Self::run_as_leader(node).await;
    } else {
      Self::run_as_follower(node).await;
    }
  }

  async fn run_as_leader(node: Arc<Mutex<Self>>) {
    let (follower_listener, client_listener, engine, followers) = {
      let mut n = node.lock().await;
      let follower_listener = std::mem::replace(
        &mut n.listener,
        TcpListener::bind("0.0.0.0:0").await.unwrap(),
      );
      let client_listener = TcpListener::bind(CLIENT_BIND_ADDR).await.unwrap();
      (
        follower_listener,
        client_listener,
        n.db_engine.clone(),
        n.followers.clone(),
      )
    };

    println!("[leader] listening for followers on {}", LEADER_BIND_ADDR);
    println!("[leader] listening for clients on {}", CLIENT_BIND_ADDR);

    // accept followers
    let followers_clone = followers.clone();
    tokio::spawn(async move {
      loop {
        let (stream, addr) = follower_listener.accept().await.unwrap();
        println!("[leader] follower connected: {}", addr);
        let handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));
        followers_clone
          .lock()
          .await
          .insert(addr.to_string(), handler);
      }
    });

    // accept clients
    loop {
      let (stream, peer) = client_listener.accept().await.unwrap();
      println!("[leader] client connected: {}", peer);
      let engine = engine.clone();
      let followers = followers.clone();

      tokio::spawn(async move {
        Self::handle_client(stream, peer.to_string(), engine, followers).await;
      });
    }
  }

  async fn handle_client(
    stream: TcpStream,
    peer: String,
    engine: Arc<Mutex<Engine>>,
    followers: Arc<Mutex<HashMap<String, ProtocolHandler>>>,
  ) {
    let mut handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));

    loop {
      match handler.receive().await {
        Ok(msg) => {
          if msg.msg_type != MessageType::Query {
            eprintln!("[leader] unexpected msg type from {}", peer);
            continue;
          }

          let sql = match String::from_utf8(msg.payload.clone()) {
            Ok(s) => s,
            Err(_) => {
              let err = Message::new(msg.id, MessageType::Error, "leader".to_string())
                .with_payload(b"invalid UTF-8 payload".to_vec());
              let _ = handler.send(&err).await;
              continue;
            }
          };

          println!("[leader] {} >>> {}", peer, sql.trim());

          // execute on leader
          let result = match parse_cmd(&sql) {
            Ok(cmd) => {
              let mut eng = engine.lock().await;
              execute(&mut eng, cmd)
            }
            Err(e) => format!("Parse error: {e}"),
          };

          // replicate raw SQL to all followers
          let mut followers = followers.lock().await;
          let mut dead = vec![];
          let mut next_id: u64 = 0;
          for (addr, follower_handler) in followers.iter_mut() {
            let sync_msg = Message::new(next_id, MessageType::Sync, "leader".to_string())
              .with_payload(sql.as_bytes().to_vec());
            next_id += 1;
            if let Err(e) = follower_handler.send(&sync_msg).await {
              eprintln!("[leader] failed to sync to {}: {}", addr, e);
              dead.push(addr.clone());
            }
          }
          for addr in dead {
            followers.remove(&addr);
          }

          // respond to client
          println!("[leader] result: {}", result);
          let response = Message::new(msg.id, MessageType::Response, "leader".to_string())
            .with_payload(result.into_bytes());
          if let Err(e) = handler.send(&response).await {
            eprintln!("[leader] write error for {}: {}", peer, e);
            break;
          }
        }
        Err(e) => {
          println!("[leader] client {} disconnected: {}", peer, e);
          break;
        }
      }
    }
  }

  async fn run_as_follower(node: Arc<Mutex<Self>>) {
    let (stream, engine) = {
      let mut n = node.lock().await;
      let stream = n
        .stream
        .take()
        .expect("follower must have a stream to leader");
      (stream, n.db_engine.clone())
    };

    println!("[follower] connected to leader at {}", LEADER_CONNECT_ADDR);

    let mut handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));

    loop {
      match handler.receive().await {
        Ok(msg) => {
          if msg.msg_type != MessageType::Sync {
            eprintln!("[follower] unexpected msg type, ignoring");
            continue;
          }

          let sql = match String::from_utf8(msg.payload.clone()) {
            Ok(s) => s,
            Err(_) => {
              eprintln!("[follower] invalid UTF-8 payload");
              continue;
            }
          };

          println!("[follower] syncing >>> {}", sql.trim());

          match parse_cmd(&sql) {
            Ok(cmd) => {
              let mut eng = engine.lock().await;
              let result = execute(&mut eng, cmd);
              println!("[follower] sync result: {}", result);
            }
            Err(e) => eprintln!("[follower] parse error: {e}"),
          }
        }
        Err(e) => {
          println!("[follower] leader disconnected: {}", e);
          break;
        }
      }
    }
  }
}

#[tokio::main]
async fn main() {
  let leader = std::env::args()
    .nth(1)
    .map(|a| a == "--leader")
    .unwrap_or(false);

  let node = Node::new(leader).await;
  Node::run(node).await;
}
