use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpListener;

use constella_db::modules::protocol::{
  handler::{ReadHandler, WriteHandler},
  message::{ResponseData, MessageType, Message},
  serializer::BincodeSerializer,
};
use constella_db::modules::cmd::Command;
use constella_db::modules::db::{
    Engine,
    Entity,
};


/// Commandline arguments
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  port: Option<u32>,
}

/// Gets the local ip of the machine
fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

/// State for the service
struct ServiceState {
  engine: Engine,
}

impl ServiceState {
  pub async fn new() -> Self {
    Self {
      engine: Engine::new().await,
    }
  }
}

/// Response from the database is mapped to this enum
pub enum ExecuteResult {
  SuccessMsg(String),
  ErrorMsg(String),
  Rows(Vec<Entity>),
  Tables(Vec<String>),
}

/// Execute the command and parse the response
async fn handle_command(
  cmd: Command,
  state: Arc<Mutex<ServiceState>>
) -> ExecuteResult {
  match cmd {

    Command::CreateTable(table) => {
      let mut s = state.lock().await;
      match s.engine.create_table(&table).await {
        Ok(o)  => ExecuteResult::SuccessMsg(o),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },

    Command::DropTable(table) => {
      let mut s = state.lock().await;
      match s.engine.drop_table(&table).await {
        Ok(o)  => ExecuteResult::SuccessMsg(o),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },

    // TODO: Support multiple entities during insert
    Command::Insert(entity) => {
      let mut s = state.lock().await;
      match s.engine.insert(&entity).await {
        Ok(o)  => ExecuteResult::SuccessMsg(o),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },

    Command::Select {table, attrs, conditions} => {
      let attrs_ref: Vec<&str> = attrs.iter()
        .map(String::as_str)
        .collect();

      let mut s = state.lock().await;
      match s.engine.select(&table, attrs_ref, conditions).await {
        Ok(o)  => ExecuteResult::Rows(o),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },

    Command::Update {table, updates, conditions} => {
      let mut s = state.lock().await;
      match s.engine.update(&table, updates, conditions).await {
        Ok(o)  => ExecuteResult::SuccessMsg(format!("Updated Rows: {}", o)),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },

    Command::Delete {table, conditions} => {
      let mut s = state.lock().await;
      match s.engine.delete(&table, conditions).await {
        Ok(o)  => ExecuteResult::SuccessMsg(format!("Deleted Rows: {}", o)),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },

    Command::ShowTables => {
      let s = state.lock().await;
      match s.engine.list_tables().await {
        Ok(o)  => ExecuteResult::Tables(o),
        Err(e) => ExecuteResult::ErrorMsg(e),
      }
    },
  }
}

/// Convert the database response to the protocol message response
fn result_to_response(result: ExecuteResult) -> MessageType {
  match result {

    ExecuteResult::SuccessMsg(msg) => {
      MessageType::Response {
        sucess: true,
        message: Some(msg),
        data: None,
      }
    },

    ExecuteResult::ErrorMsg(msg) => {
      MessageType::Response {
        sucess: false,
        message: Some(msg),
        data: None,
      }
    },

    ExecuteResult::Rows(rows) => {
      MessageType::Response {
        sucess: true,
        message: None,
        data: Some(ResponseData::Rows(rows)),
      }
    },

    ExecuteResult::Tables(tables) => {
      MessageType::Response {
        sucess: true,
        message: None,
        data: Some(ResponseData::Tables(tables)),
      }
    },

  }
}

/// Responsible to handle the db requests
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
    let result = match msg.msg_type {

      // Handle the execute command message
      MessageType::ExecCmd => {
        match msg.command {
          Some(cmd) => handle_command(cmd, state.clone()).await,
          None => ExecuteResult::ErrorMsg("Command is Missing".to_string()),
        }
      },

      // Ignore any other message
      _ => ExecuteResult::ErrorMsg("This only handles ExecCmd Message".to_string()),
    };

    // Convert the result to the MessageType::Response with datas'
    let msg_type = result_to_response(result);
    let response_msg = Message::new(
      "".to_string(),
      msg_type,
      "".to_string()
    );

    // Send it back to the job schedular microservice
    let mut writer = write_handler_mutex.lock().await;
    writer.send(&response_msg).await.unwrap();
  }
}

/// Starts the listener in which the job service will connect
async fn start_listener(state: Arc<Mutex<ServiceState>>, port: u32) {
  let ip = get_local_ip().unwrap();
  let listener = TcpListener::bind(
    format!("{}:{}", ip, port)
  ).await.unwrap();

  let bound_port = listener.local_addr().unwrap().port();
  let full_ip = format!("{}:{}", ip, bound_port);
  println!("[LOG] DB Service listening on {}", &full_ip);

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
  let args = Args::parse();
  let port = args.port.unwrap_or(0);

  let state = Arc::new(Mutex::new(ServiceState::new().await));
  start_listener(state, port).await;
}
