use tokio::sync::Mutex;
use std::sync::Arc;
use serde_json::json;
use reqwest::Client;

use protocol_module::{
  handler::WriteHandler,
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
pub async fn process_job(job: Job, state: Arc<Mutex<ServiceState>>) {
  // Extract the query
  let query = String::from_utf8(job.msg.payload).unwrap();
  println!("[LOG] Request: {}", &query);

  /*
  // Send the query to the query service
  let s = state.lock().await;
  let client = Client::new();
  let response = client
    .post(format!("http://{}/query", s.query_service_ip))
    .json(&json!({
      "query": query
    }))
    .send()
    .await
    .unwrap();

  // Get the response from query service
  let response_text = response.text().await.unwrap();
  println!("[LOG] Response: {}", &response_text);
  */

  let response_text = r#"{
      "success": true,
      "message": "Here is the response",
      "rows": null
  }"#.to_string();

  // Send the response back to the node
  let mut handler = job.job_owner_write_handler.lock().await;
  let response = Message::new(
    "".to_string(),
    MessageType::JobComplete { job_id: job.id },
    "".to_string()
  )
    .with_payload(response_text.into_bytes());
  handler.send(&response).await.unwrap();
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
        process_job(job, state.clone()).await;
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
