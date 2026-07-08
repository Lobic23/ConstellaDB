use tokio::sync::Mutex;
use std::sync::Arc;
use serde_json::json;
use reqwest::Client;

use protocol_module::{
  handler::WriteHandler,
  message::{MessageType, Message},
};
use cmd_module::Command;
use db_module::{Value, Condition, Type};

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
  let client = Client::new();

  let cmd = job
    .msg
    .command
    .clone()
    .expect("Command not found");

  let response_text = match cmd {

    Command::CreateTable(table) => {
      client
        .post(format!("http://{}/tables", db_service_ip))
        .json(&json!({
          "name": table.name,
          "attrs": table.attrs
            .into_iter()
            .map(|a| {
              json!({
                "name": a.name,
                "data_type": match a.data_type {
                  Type::Int => "INT".to_string(),
                  Type::VarChar(_) => "STRING".to_string(),
                }
              })
            })
            .collect::<Vec<_>>()
        }))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }


    Command::DropTable(table) => {
      client
        .delete(format!(
          "http://{}/tables/{}",
          db_service_ip,
          table
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }


    Command::ShowTables => {
      client
        .get(format!("http://{}/tables", db_service_ip))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }


    Command::Insert(entity) => {
      let mut data = serde_json::Map::new();

      for d in entity.data {
        let value = match d.value {
          Value::Int(i) => json!(i),
          Value::VarChar(s) => json!(s),
          Value::Null => json!(null),
        };

        data.insert(d.name, value);
      }

      client
        .post(format!(
          "http://{}/tables/{}/rows",
          db_service_ip,
          entity.of
        ))
        .json(&data)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }


    Command::Select {
      table,
      ..
    } => {
      client
        .get(format!(
          "http://{}/tables/{}/rows",
          db_service_ip,
          table
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }


    Command::Update {
      table,
      updates,
      conditions,
    } => {
      let mut update_json = serde_json::Map::new();

      for d in updates {
        update_json.insert(
          d.name,
          db_value_to_json(d.value)
        );
      }


      let mut condition_json = serde_json::Map::new();

      for c in conditions {
        if let Condition::Compare {
          attr,
          value,
          ..
        } = c {
          condition_json.insert(
            attr,
            db_value_to_json(value)
          );
        }
      }


      client
        .put(format!(
          "http://{}/tables/{}/rows",
          db_service_ip,
          table
        ))
        .json(&json!({
          "updates": update_json,
          "conditions": condition_json
        }))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }


    Command::Delete {
      table,
      conditions,
    } => {
      let mut condition_json = serde_json::Map::new();

      for c in conditions {
        if let Condition::Compare {
          attr,
          value,
          ..
        } = c {
          condition_json.insert(
            attr,
            db_value_to_json(value)
          );
        }
      }


      client
        .delete(format!(
          "http://{}/tables/{}/rows",
          db_service_ip,
          table
        ))
        .json(&condition_json)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
    }
  };


  // send response back to node
  let mut handler =
    job.job_owner_write_handler.lock().await;

  let response = Message::new(
    "".to_string(),
    MessageType::JobComplete {
      job_id: job.id
    },
    "".to_string(),
  )
  .with_payload(response_text.into_bytes());


  handler
    .send(&response)
    .await
    .unwrap();
}


fn db_value_to_json(value: Value) -> serde_json::Value {
  match value {
    Value::Int(i) => json!(i),
    Value::VarChar(s) => json!(s),
    Value::Null => json!(null),
  }
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
