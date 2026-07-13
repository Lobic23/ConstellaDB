use tokio::sync::Mutex;
use tokio::net::TcpStream;
use std::sync::Arc;

use constella_db::modules::protocol::{
  handler::{ReadHandler, WriteHandler},
  serializer::BincodeSerializer,
  message::{MessageType, Message},
};

use crate::state::ServiceState;


/// Job thats going to be processed
pub struct Job {
  pub id: String,
  pub msg: Message,
  pub job_owner_write_handler: Arc<Mutex<WriteHandler>>, // Refers to the node connection that allocated the job
}

/// Job processor which calls to the query service
/// and returns the response to the job owner via tcp stream
pub async fn process_job(job: Job, db_service_ip: &str) {
  let stream = TcpStream::connect(db_service_ip).await.unwrap();
  let (reader, writer) = stream.into_split();
  let mut read_handler = ReadHandler::new(reader, Box::new(BincodeSerializer));
  let mut write_handler = WriteHandler::new(writer, Box::new(BincodeSerializer));

  // Clone the command
  let cmd = job
    .msg
    .command
    .clone()
    .expect("Command not found");

  // Create new message
  let msg = Message::new("".to_string(), MessageType::ExecCmd, "".to_string())
    .with_command(cmd);

  // Send the msg to db service
  write_handler.send(&msg).await.unwrap();

  // Wait for response
  let received_msg = read_handler.receive().await.unwrap();

  // send response back to node
  let mut handler = job.job_owner_write_handler.lock().await;

  let response = Message::new(
    "".to_string(),
    MessageType::JobComplete {
      job_id: job.id
    },
    "".to_string(),
  )
  .with_payload(received_msg.msg_type.into_bytes());

  handler
    .send(&response)
    .await
    .unwrap();
}

/// Worker process runs in a multithreaded environment
/// Extracts the job from the queue and processes it
pub async fn worker(state: Arc<Mutex<ServiceState>>) {
  loop {
    let job = {
      let s = state.lock().await;
      let mut q = s.job_queue.lock().await;

      q.pop_front()
    };

    match job {
      Some(job) => {
        let ip = {
          let s = state.lock().await;
          s.db_service_ip.clone()
        };
        process_job(job, &ip).await;
      }
      None => {
        // Wait for 10ms if job queue is empty
        tokio::time::sleep(
          std::time::Duration::from_millis(10)
        ).await;
      }
    }
  }
}


/// Spawns the set number of worker threads
pub async fn spawn_workers(state: Arc<Mutex<ServiceState>>) {
  let s = state.lock().await;
  for _ in 0..s.max_threads {
    let state_clone = state.clone();
    tokio::spawn(async move {
      worker(state_clone).await;
    });
  }
}
