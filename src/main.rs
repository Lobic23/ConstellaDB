use clap::{Parser, Subcommand};
use rand::RngExt;
use std::env;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

// cargo gateway
// cargo node < -p {port} >
// cargo client

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct Cli {
  #[command(subcommand)]
  command: CliSubCommand,
}

#[derive(Subcommand, Debug)]
enum CliSubCommand {
  /// Run the node
  RunNode(NodeCli),

  /// Run gateway
  RunGateway,

  /// Run db client
  RunClient,
}

#[derive(Parser, Debug)]
struct NodeCli {
  /// OPTIONAL: Port for the node to listen on.
  /// If not provided, a random port is taken
  #[arg(short, long)]
  port: Option<u32>,
}

/// The gateway's IP and ports, loaded from the .env file.
/// This is the one and only source for this information -
/// there is no CLI fallback, and any missing/invalid value is a hard panic.
struct GatewayConfig {
  ip: String,
  client_port: u32,
  node_port: u32,
}

impl GatewayConfig {
  fn load() -> Self {
    // Loads the .env file in the current directory into the process env.
    // Panics if the file is missing or unreadable.
    dotenvy::dotenv().expect("Failed to load .env file");

    let ip = env::var("GATEWAY_IP")
      .expect("GATEWAY_IP must be set in .env");

    let client_port = env::var("GATEWAY_CLIENT_PORT")
      .expect("GATEWAY_CLIENT_PORT must be set in .env")
      .parse::<u32>()
      .expect("GATEWAY_CLIENT_PORT must be a valid u32");

    let node_port = env::var("GATEWAY_NODE_PORT")
      .expect("GATEWAY_NODE_PORT must be set in .env")
      .parse::<u32>()
      .expect("GATEWAY_NODE_PORT must be a valid u32");

    Self { ip, client_port, node_port }
  }

  fn node_addr(&self) -> String {
    format!("{}:{}", self.ip, self.node_port)
  }

  fn client_addr(&self) -> String {
    format!("{}:{}", self.ip, self.client_port)
  }
}

/// Gets the local ip of the machine
pub fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

fn log_command(cmd: &Command) {
  let program = cmd.get_program().to_string_lossy();

  let args = cmd
    .get_args()
    .map(|a| a.to_string_lossy())
    .collect::<Vec<_>>()
    .join(" ");

  println!("$ {} {}", program, args);
}

#[tokio::main]
async fn main() {
  let cli = Cli::parse();
  let exe_dir = env::current_exe()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf();

  // Load gateway ip/ports up front - panics immediately if .env is
  // missing or malformed, before any child processes are spawned.
  let gateway_config = GatewayConfig::load();

  match cli.command {
    CliSubCommand::RunNode(n_cli) => {
      let ip = get_local_ip().unwrap();
      let node_port = n_cli.port.unwrap_or(0);
      let db_port = rand::rng().random_range(49152..65536);
      let job_port = rand::rng().random_range(49152..65536);

      let mut children: Vec<Child> = Vec::new();

      // Start DB
      let db_path = format!("{}/db_service", exe_dir.display());
      let mut db_cmd = Command::new(db_path);
      db_cmd.args(["-p", &db_port.to_string()]);
      log_command(&db_cmd);
      children.push(db_cmd.spawn().unwrap());

      // Give it time to start
      thread::sleep(Duration::from_secs(1));

      // Start Job Service
      let job_path = format!("{}/job_service", exe_dir.display());
      let mut job_cmd = Command::new(job_path);
      job_cmd.args([
        "-p", &job_port.to_string(),
        "-d", &format!("{}:{}", ip, db_port),
      ]);
      log_command(&job_cmd);
      children.push(job_cmd.spawn().unwrap());

      // Give it time to connect
      thread::sleep(Duration::from_secs(2));

      // Start Node
      let node_path = format!("{}/node", exe_dir.display());
      let mut node_cmd = Command::new(node_path);
      node_cmd.args([
        "-p",
        &node_port.to_string(),
        "-j",
        &format!("{}:{}", ip, job_port),
        "-g",
        &gateway_config.node_addr(),
      ]);
      log_command(&node_cmd);
      let mut node = node_cmd.spawn().unwrap();

      // Wait until node exits
      node.wait().unwrap();

      // Kill background services
      for child in &mut children {
        let _ = child.kill();
        let _ = child.wait();
      }
    },

    CliSubCommand::RunGateway => {
      let gateway_path = format!("{}/gateway", exe_dir.display());
      let mut gateway = Command::new(gateway_path)
        .args([
          "-i", &gateway_config.ip,
          "-c", &gateway_config.client_port.to_string(),
          "-n", &gateway_config.node_port.to_string(),
        ])
        .spawn()
        .unwrap();

      gateway.wait().unwrap();
    },

    CliSubCommand::RunClient => {
      let client_path = format!("{}/db_client", exe_dir.display());
      let mut client = Command::new(client_path)
        .args([&gateway_config.client_addr()])
        .spawn()
        .unwrap();

      client.wait().unwrap();
    },
  }
}