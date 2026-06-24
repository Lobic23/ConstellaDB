use uuid::Uuid;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use clap::Parser;
use std::thread;
use std::sync::Arc;
use std::collections::VecDeque;
use serde_json::json;
use reqwest::Client;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::{MessageType, Message},
  serializer::BincodeSerializer,
};

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

/// Job thats going to be processed
struct Job {
  pub id: String,
  pub msg: Message,
  pub job_owner_write_handler: Arc<Mutex<WriteHandler>>,
}

/// State for the service
struct ServiceState {
  pub ip: String,
  pub max_threads: usize,
  pub job_queue: Arc<Mutex<VecDeque<Job>>>,
  pub query_service_ip: String,
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
    }
  }
}

/// Gets the local IP of the machine
fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

/// Job processor which calls to the query service
/// and returns the response to the job owner via tcp stream
async fn process_job(job: Job, state: Arc<Mutex<ServiceState>>) {
  // Extract the query
  let query = String::from_utf8(job.msg.payload).unwrap();
  println!("[LOG] Request: {}", &query);

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

  // Send the response back to the node
  let mut handler = job.job_owner_write_handler.lock().await;
  let response = Message::new(
    0,
    MessageType::JobComplete { job_id: job.id },
    "".to_string()
  )
    .with_payload(response_text.into_bytes());
  handler.send(&response).await.unwrap();
}

/// Worker process runs in a multithreaded environment
/// Extracts the job from the queue and processes it
async fn worker(state: Arc<Mutex<ServiceState>>) {
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

async fn connection_listener(
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

async fn start_listener(state: Arc<Mutex<ServiceState>>, port: u32) {
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
  {
    let s = state.lock().await;
    for _ in 0..s.max_threads {
      let state_clone = state.clone();
      tokio::spawn(async move {
        worker(state_clone).await;
      });
    }
  }

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
