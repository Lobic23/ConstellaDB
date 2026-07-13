use constella_db::modules::cmd::{execute, parse_cmd};
use constella_db::modules::db::Engine;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
  let mut engine = Engine::new().await;

  println!("constella_db CLI");
  println!("Type SQL statements. Type 'exit' or 'quit' or 'q' to leave.\n");

  loop {
    print!("constella_db> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).unwrap() == 0 {
      break;
    }

    let sql = input.trim();

    if sql.is_empty() {
      continue;
    }

    if matches!(sql, "exit" | "quit" | "q") {
      break;
    }

    let result = match parse_cmd(sql) {
      Ok(cmd) => execute(&mut engine, cmd).await.to_string(),
      Err(e) => format!("Parse error: {e}"),
    };

    println!("{result}");
  }
}