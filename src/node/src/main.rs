pub mod node;

use tokio::net::{TcpListener, TcpStream};
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::{MessageType, Message},
  serializer::BincodeSerializer,
};
use node::{Node, NodeStatus};

fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

/// Commandline args for the node
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  leader: bool,

  #[arg(short, long, num_args=1..)]
  followers: Option<Vec<String>>,       // IP's of the followers

  #[arg(short, long)]
  job_service: String,
}

/// Ran when node is a leader
/// Connects to the follower's server as a client and stores the stream
async fn connect_to_followers(node: Arc<Mutex<Node>>, ips: Vec<String>) {
  for follower_ip in ips {
    let stream = TcpStream::connect(&follower_ip).await.unwrap();
    let (reader, writer) = stream.into_split();

    let read_handler = Arc::new(Mutex::new(
      ReadHandler::new(reader, Box::new(BincodeSerializer))
    ));
    let write_handler = Arc::new(Mutex::new(
      WriteHandler::new(writer, Box::new(BincodeSerializer))
    ));

    println!("[LOG] Connected to follower {}", &follower_ip);

    {
      let mut n = node.lock().await;
      n.followers.insert(follower_ip, (read_handler.clone(), write_handler.clone()));
    }

    // Spawn the listener for all the follower as well
    let x = node.clone();
    tokio::spawn(async move {
      connection_listener(read_handler, write_handler, x).await;
    });
  }
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

/// Distribute the message to the followers
/// TODO(slok): For now this distributes the message to every follower.
/// Later we need to develop on how the message should be distributed between
/// followers so that the efficiency is maximized
async fn distribute_message(msg: &Message, node: Arc<Mutex<Node>>) {
  let followers = {
    let n = node.lock().await;
    n.followers.clone()
  };
  let instructions = {
    let n = node.lock().await;
    n.instructions.clone()
  };

  for (ip, (_, write_handler)) in followers {
    let mut handler = write_handler.lock().await;
    handler.send(msg).await.unwrap();

    // Save the node to the instruction
    if let Some((status, _)) = instructions.get(&msg.id) {
      let mut s = status.lock().await;
      s.push(NodeStatus { id: ip, status: false });
    }
  }

  if let Some((status, _)) = instructions.get(&msg.id) {
    let s = status.lock().await;
    println!("[LOG] Instruction {} distributed to: {:?}", msg.id, s);
  }
}

/// Allocates a new instruction in the leader's node
async fn create_new_instruction(
  msg: &Message,
  node: Arc<Mutex<Node>>,
  write_handler: Arc<Mutex<WriteHandler>>
) {
  let mut n = node.lock().await;
  n.instructions.insert(
    msg.id,
    (Arc::new(Mutex::new(Vec::new())), write_handler)
  );
  println!("[LOG] New instruction created: {}", msg.id);
}

/// Makes the status to true for node who has completed the task
async fn sucess_instruction_response(
  inst_id: u64,
  node_id: &str,
  node: Arc<Mutex<Node>>
) {
  let n = node.lock().await;
  if let Some((node_status, _)) = n.instructions.get(&inst_id) {
    let mut ns = node_status.lock().await;
    for node in ns.iter_mut() {
      if node.id == node_id {
        node.status = true;
        println!("[LOG] Instruction {} completed by: {:?}", inst_id, node);
      }
    }
  }
}

/// Checks if all the nodes has completed the task for the given instruction
async fn is_instruction_finished(inst_id: u64, node: Arc<Mutex<Node>>) -> bool {
  let n = node.lock().await;
  if let Some((node_status, _)) = n.instructions.get(&inst_id) {
    let mut ns = node_status.lock().await;
    for node in ns.iter_mut() {
      if !node.status {
        return false;
      }
    }
    return true;
  }
  false
}

/// Listens to every protocol sent into this stream
/// Here the protocol is interpreted
async fn connection_listener(
  read_handler_mutex: Arc<Mutex<ReadHandler>>,
  write_handler_mutex: Arc<Mutex<WriteHandler>>,
  node: Arc<Mutex<Node>>
) {
  loop {
    let received = {
      let mut handler = read_handler_mutex.lock().await;
      handler.receive().await
    };

    if let Err(e) = received {
      println!("[LOG] Connection lost due to: {}", e);
      break;
    }

    let msg = received.unwrap();
    match &msg.msg_type {

      // Handles the execution of the instruction
      // If the node is the leader, the query will be distributed
      // among followers.
      // If the node is the follower then the query will be sent to
      // the job service for execution
      MessageType::Query => {

        // Extracting the is_leader so that lock is dropped here
        let is_leader = {
          let n = node.lock().await;
          n.leader
        };

        if is_leader {
          // If the node is leader create a new instruction and
          // distribute that instruction to the followers
          create_new_instruction(&msg, node.clone(), write_handler_mutex.clone()).await;
          distribute_message(&msg, node.clone()).await;
        } else {
          let mut n = node.lock().await;
          // Store the instruction owner's write handler
          n.instruction_owners.insert(msg.id, write_handler_mutex.clone());

          // If the node is the follower, send the task to the job service
          if let Some((_, write_handler_mutex)) = &n.job_service {
            let mut handler = write_handler_mutex.lock().await;
            handler.send(&msg).await.unwrap();
          }
        }
      },

      // Handles the response sent by the followers to the leader
      MessageType::Response => {
        let is_leader = {
          let n = node.lock().await;
          n.leader
        };
        println!("[LOG] Received Response:\n{:#?}", msg);

        // If the node is the leader then the response would be the result
        // back from the followers, so the data is collected here and checked
        // if the instruction is complete and if so then the response is sent
        // to the client
        if is_leader {
          sucess_instruction_response(msg.id, &msg.node_id, node.clone()).await;

          // TODO(slok): Save the response

          // The instruction is completed
          if is_instruction_finished(msg.id, node.clone()).await {
            let n = node.lock().await;

            println!("[LOG] Instruction {} completed", msg.id);

            // TODO(slok): Prepare proper response

            // Sending the response to client
            if let Some((_, client_write_handler)) = n.instructions.get(&msg.id) {
              let response = Message::new(
                msg.id,
                MessageType::Response,
                n.id.clone()
              );
              let mut handler = client_write_handler.lock().await;
              handler.send(&response).await.unwrap();
              println!("sent: {:#?}", response);
            }
          } else {
            println!("[LOG] Instruction {} not complete", msg.id);
          }
        }
      },

      // Save the job in the job table when a new job has been initialized
      // by the job service
      MessageType::JobInit { job_id } => {
        let inst_id = msg.id;
        let job_id = job_id;

        let mut n = node.lock().await;
        n.job_table.insert(job_id.clone(), inst_id);

        println!("[LOG] Initialized new job [{} -> {}]", job_id, inst_id);
      },

      // When job is completed, follower will send the result from the job
      // service to the leader as a response
      MessageType::JobComplete { job_id } => {
        let n = node.lock().await;

        if let Some(inst_id) = n.job_table.get(job_id) {
          if let Some(owner_write_handler) = n.instruction_owners.get(inst_id) {
            let mut handler = owner_write_handler.lock().await;

            // Sending the response to the leader
            let response = Message::new(
              *inst_id,
              MessageType::Response,
              n.id.clone()
            )
              .with_payload(msg.payload.clone());
            handler.send(&response).await.unwrap();
          }
        }

        println!("[LOG] Job {} is completed", job_id);
      }
      _ => {println!("wtf");},
    }
  }
}

async fn start_listener(node: Arc<Mutex<Node>>) {
  let ip = get_local_ip().unwrap();
  let listener = TcpListener::bind(
    format!("{}:0", ip)
  ).await.unwrap();

  let port = listener.local_addr().unwrap().port();
  let full_ip = format!("{}:{}", ip, port);
  {
    let mut n = node.lock().await;
    n.id = full_ip.clone();
  }
  println!("[LOG] Node listening on {}", &full_ip);

  loop {
    let (stream, addr) = listener.accept().await.unwrap();
    println!("[LOG] {} connected", addr);

    let n = node.clone();

    let (reader, writer) = stream.into_split();
    let read_handler = Arc::new(Mutex::new(
      ReadHandler::new(reader, Box::new(BincodeSerializer))
    ));
    let write_handler = Arc::new(Mutex::new(
      WriteHandler::new(writer, Box::new(BincodeSerializer))
    ));

    tokio::spawn(async move {
      connection_listener(read_handler, write_handler, n).await;
    });
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

  // Connecting to the job service
  let job_service_ip = args.job_service;
  connect_to_job_service(node.clone(), &job_service_ip).await;

  start_listener(node).await;
}
