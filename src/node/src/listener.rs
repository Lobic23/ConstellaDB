/// Here lies the connection listener of the node
/// Handles the flow of data in and out of the node

use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;

use cmd_module::format_rows;
use db_module::Entity;
use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::{MessageType, Message},
  serializer::BincodeSerializer,
};

use crate::node::Node;
use crate::leader::distribute_message;
use crate::instruction::{
  create_new_instruction,
  sucess_instruction_response,
  is_instruction_finished,
};


/// Gets the local ip of the machine
pub fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

/// Handler that handles the messages/commands sent by [leader] node
/// to the [follower] node
pub async fn leader_message_handler(
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

      MessageType::ExecCmd => {
        println!("[LOG] Command to execute:\n{:#?}", &msg.command);

        let mut n = node.lock().await;

        // Store the instruction owner's write handler
        n.instruction_owners.insert(msg.id.clone(), write_handler_mutex.clone());

        // Send the task to the job service
        if let Some((_, write_handler_mutex)) = &n.job_service {
          let mut handler = write_handler_mutex.lock().await;
          handler.send(&msg).await.unwrap();
        }
      },

      _ => println!("Unexpected!"),
    }
  }
}

/// Handler that handles the messages/commands sent by [follower] node
/// to the [leader] node
pub async fn follower_message_handler(
  read_handler_mutex: Arc<Mutex<ReadHandler>>,
  _write_handler_mutex: Arc<Mutex<WriteHandler>>,
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

      // Handles the response sent by the followers to the leader
      MessageType::Response => {
        println!("[LOG] Received Response:\n{:#?}", msg);

        let payload = String::from_utf8(msg.payload).unwrap();
        let payload_json: serde_json::Value = serde_json::from_str(&payload).unwrap();

        // If the node is the leader then the response would be the result
        // back from the followers, so the data is collected here and checked
        // if the instruction is complete and if so then the response is sent
        // to the client
        sucess_instruction_response(&msg.id, &msg.node_id, node.clone()).await;

        // If the rows are present in the response then append to the
        // instructions response rows
        if let Some(rows) = payload_json
          .get("rows")
          .cloned()
          .and_then(|v| serde_json::from_value::<Vec<Entity>>(v).ok())
        {
          let mut n = node.lock().await;

          if let Some(instruction) = n.instructions.get_mut(&msg.id) {
            match &mut instruction.response_rows {
              Some(existing_rows) => existing_rows.extend(rows),
              None => instruction.response_rows = Some(rows),
            }
          }
        }

        // If a message is present in the response then save that to
        // the response message of the instruction
        if let Some(message) = payload_json
          .get("message")
          .and_then(|v| v.as_str())
        {
          let mut n = node.lock().await;
          if let Some(instruction) = n.instructions.get_mut(&msg.id) {
            instruction.response_message = Some(message.to_string());
          }
        }

        // The instruction is completed
        if is_instruction_finished(&msg.id, node.clone()).await {
          println!("[LOG] Instruction {} completed", msg.id);

          // When the instruction is completed, the response is sent back
          // to the client
          let mut n = node.lock().await;
          if let Some(instruction) = n.instructions.get(&msg.id) {
            let mut response = Message::new(
              msg.id.clone(),
              MessageType::Response,
              n.id.clone()
            );

            // If the instruction has a message then it is sent as
            // message have more priority (eg: errors)
            if let Some(msg) = &instruction.response_message {
              response = response.with_payload(msg.clone().into_bytes());
            }

            // If the instruction has rows (from select) then it is
            // formated and returned to the user
            else if let Some(rows) = &instruction.response_rows {
              let rows_string = format_rows(rows.to_vec());
              response = response.with_payload(rows_string.into_bytes());
            }

            // Sending the response
            let mut handler = instruction.client_write_handler.lock().await;
            handler.send(&response).await.unwrap();
            println!("[LOG] Sent: {:#?}", response);
          }

          // After done remove that instruction
          n.instructions.remove(&msg.id);
        } else {
          println!("[LOG] Instruction {} not complete", msg.id);
        }
      },

      _ => println!("Unexpected!")
    }
  }
}

/// Handler that handles the messages/commands sent by [job schedular service]
/// to the [follower/leader] node
pub async fn job_message_handler(
  read_handler_mutex: Arc<Mutex<ReadHandler>>,
  _write_handler_mutex: Arc<Mutex<WriteHandler>>,
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

      // Save the job in the job table when a new job has been initialized
      // by the job service
      MessageType::JobInit { job_id } => {
        let inst_id = msg.id;
        let job_id = job_id;

        let mut n = node.lock().await;
        n.job_table.insert(job_id.clone(), inst_id.clone());

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
              inst_id.clone(),
              MessageType::Response,
              n.id.clone()
            )
              .with_payload(msg.payload.clone());
            handler.send(&response).await.unwrap();
          }
        }

        println!("[LOG] Job {} is completed", job_id);
      }

      _ => println!("Unexpected!")
    }
  }
}

/// Handler that handles the messages/commands sent by [gateway]
/// to the [leader] node
pub async fn gateway_message_handler(
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

      // When lead command is received, a new instruction is created
      // for that operation and work is distributed among the provided followers
      MessageType::Lead { followers } => {
        create_new_instruction(
          &msg,
          node.clone(),
          write_handler_mutex.clone(),
          followers.to_vec()
        ).await;
        distribute_message(&msg, node.clone()).await;
      },

      _ => println!("Unexpected!"),
    }
  }
}


/// Runs the connection listener in which other nodes will connect to
/// Here leader node is connected to the follower node
pub async fn start_listener(node: Arc<Mutex<Node>>, listener: TcpListener) {
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
      leader_message_handler(read_handler, write_handler, n).await;
    });
  }
}
