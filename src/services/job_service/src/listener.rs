use uuid::Uuid;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::sync::Arc;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::{MessageType, Message},
  serializer::BincodeSerializer,
};

use crate::job::{Job, spawn_workers};
use crate::state::ServiceState;


/// Gets the local IP of the machine
fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}


/// Responsible to handle the read requests and write response
/// to the node
pub async fn connection_listener(
  read_handler_mutex: Arc<Mutex<ReadHandler>>,
  write_handler_mutex: Arc<Mutex<WriteHandler>>,
  state: Arc<Mutex<ServiceState>>
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
    match msg.msg_type {

      // When query message is received, a new job is created
      // and JobInit message is sent back to the node.
      // When the job is completed a JobComplete message will
      // be sent to the node from the worker thread.
      MessageType::Query => {
        println!("[LOG]: New Job\n{:#?}", msg);

        // Create job
        let job_id = Uuid::new_v4().to_string();
        let job = Job {
          id: job_id.clone(),
          msg: msg.clone(),
          job_owner_write_handler: write_handler_mutex.clone(),
        };

        // Store the job in the queue
        {
          let s = state.lock().await;
          let mut jq = s.job_queue.lock().await;
          jq.push_back(job);
        }

        // Send response to the owner
        {
          let s = state.lock().await;
          let mut handler = write_handler_mutex.lock().await;
          let response = Message::new(
            msg.id,
            MessageType::JobInit {job_id: job_id},
            s.ip.clone()
          );
          handler.send(&response).await.unwrap();
        }
      },
      _ => {},
    }
  }
}

/// Starts the listener in which the node will connect
pub async fn start_listener(state: Arc<Mutex<ServiceState>>, port: u32) {
  let ip = get_local_ip().unwrap();
  let listener = TcpListener::bind(
    format!("{}:{}", ip, port)
  ).await.unwrap();

  let bound_port = listener.local_addr().unwrap().port();
  let full_ip = format!("{}:{}", ip, bound_port);
  println!("[LOG] Job Service listening on {}", &full_ip);

  // Store the ip
  {
    let mut s = state.lock().await;
    s.ip = full_ip;
  }

  // Spawn all the workers
  spawn_workers(state.clone()).await;

  loop {
    let (stream, addr) = listener.accept().await.unwrap();
    println!("[LOG] {} connected", addr);

    let state_clone = state.clone();

    let (reader, writer) = stream.into_split();
    let read_handler = Arc::new(Mutex::new(
      ReadHandler::new(reader, Box::new(BincodeSerializer))
    ));
    let write_handler = Arc::new(Mutex::new(
      WriteHandler::new(writer, Box::new(BincodeSerializer))
    ));
    tokio::spawn(async move {
      connection_listener(read_handler, write_handler, state_clone).await;
    });
  }
}
