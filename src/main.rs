use clap::{Parser, Subcommand};
use rand::RngExt;
use std::env;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

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
  RunGateway(GatewayCli),

  /// Run db client
  RunClient(ClientCli)
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct NodeCli {
  /// Address of the gateway to connect to.
  /// Example: 127.0.0.1:8000
  #[arg(short, long)]
  gateway: String,

  /// Port for the node to listen on.
  /// If not provided, a random port is taken
  #[arg(short, long)]
  port: Option<u32>,
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct GatewayCli {
  /// Port to which it listens for client.
  /// If not provided, a random port is taken
  #[arg(short, long)]
  client_port: Option<u32>,

  /// Port to which it listens for node.
  /// If not provided, a random port is taken
  #[arg(short, long)]
  node_port: Option<u32>,
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct ClientCli {
  /// Address of the gateway to connect to.
  /// Example: 127.0.0.1:8000
  #[arg(short, long)]
  gateway: String,
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
        &n_cli.gateway,
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

    CliSubCommand::RunGateway(g_cli) => {
      let client_port = g_cli.client_port.unwrap_or(0);
      let node_port = g_cli.node_port.unwrap_or(0);

      let gateway_path = format!("{}/gateway", exe_dir.display());
      let mut gateway = Command::new(gateway_path)
        .args([
          "-c", &client_port.to_string(),
          "-n", &node_port.to_string(),
        ])
        .spawn()
        .unwrap();

      gateway.wait().unwrap();
    },

    CliSubCommand::RunClient(c_cli) => {
      let gateway = c_cli.gateway;

      let client_path = format!("{}/db_client", exe_dir.display());
      let mut client = Command::new(client_path)
        .args([&gateway])
        .spawn()
        .unwrap();

      client.wait().unwrap();
    },
  }
}
