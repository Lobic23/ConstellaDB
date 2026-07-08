use std::thread;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

use db_module::Engine;

use crate::job::Job;


/// State for the service
pub struct ServiceState {
  pub ip: String,
  pub max_threads: usize,
  pub job_queue: Arc<Mutex<VecDeque<Job>>>,
  pub query_service_ip: String,
  pub engine: Arc<Mutex<Engine>>,
}

impl ServiceState {
  pub fn new() -> Self {
    let max_thread_count = thread::available_parallelism()
      .unwrap()
      .get();

    Self {
      ip: "".to_string(),
      max_threads: max_thread_count,
      job_queue: Arc::new(Mutex::new(VecDeque::new())),
      query_service_ip: "".to_string(),
      engine: Arc::new(Mutex::new(Engine::new())),
    }
  }
}

