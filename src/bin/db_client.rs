use std::{
    fs,
    time::Instant,
};

use colored::*;
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use rustyline::{
    error::ReadlineError,
    DefaultEditor,
};
use tokio::net::TcpStream;
use uuid::Uuid;

use constella_db::modules::{
    db::Entity,
    protocol::{
        BincodeSerializer,
        Message,
        MessageType,
        ProtocolHandler,
        ResponseData,
    },
};

const HISTORY_FILE: &str = ".constella_history";

fn print_banner(addr: &str, node: &str) {
    println!(
        "{}",
        "══════════════════════════════════════════════════════".cyan()
    );
    println!("{}", "           ConstellaDB Interactive Client".bold());
    println!(
        "{}",
        "══════════════════════════════════════════════════════".cyan()
    );

    println!("Connected to {}", addr.green());
    println!("Node ID      {}", node.green());

    println!();
    println!("{}", "Commands".bold());
    println!("  help");
    println!("  clear");
    println!("  run-file <file.sql>");
    println!();

    println!("{}", "Keyboard".bold());
    println!("  ↑ ↓       History");
    println!("  Ctrl+C    Cancel current input");
    println!("  Ctrl+D    Exit");
    println!();
}

fn print_rows(rows: &[Entity]) {
    if rows.is_empty() {
        println!("{}", "(0 rows)".yellow());
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    let headers = rows[0]
        .data
        .iter()
        .map(|c| Cell::new(&c.name))
        .collect::<Vec<_>>();

    table.set_header(headers);

    for row in rows {
        let cells = row
            .data
            .iter()
            .map(|d| Cell::new(format!("{:?}", d.value)))
            .collect::<Vec<_>>();

        table.add_row(cells);
    }

    println!("{table}");
    println!(
        "{} {}",
        rows.len().to_string().green().bold(),
        "row(s)".green()
    );
}

async fn execute_sql(
    handler: &mut ProtocolHandler,
    sql: &str,
    node_id: &str,
) -> bool {
    let msg = Message::new(
        Uuid::new_v4().to_string(),
        MessageType::Query,
        node_id.to_string(),
    )
    .with_payload(sql.as_bytes().to_vec());

    if let Err(e) = handler.send(&msg).await {
        println!("{} {}", "Send Error:".red(), e);
        return false;
    }

    let start = Instant::now();

    match handler.receive().await {
        Ok(resp) => {
            match resp.msg_type {
                MessageType::Response {
                    sucess,
                    message,
                    data,
                } => {
                    if sucess {
                        println!("{}", "✔ Success".green().bold());
                    } else {
                        println!("{}", "✘ Failed".red().bold());
                    }

                    if let Some(msg) = message {
                        println!("{msg}");
                    }

                    if let Some(data) = data {
                        match data {
                            ResponseData::Rows(rows) => {
                                print_rows(&rows);
                            }

                            ResponseData::Tables(tables) => {
                                if tables.is_empty() {
                                    println!("No tables.");
                                } else {
                                    println!("{}", "Tables".bold());

                                    for t in tables {
                                        println!("  {}", t.green());
                                    }
                                }
                            }
                        }
                    }

                    println!(
                        "{} {:.2?}\n",
                        "Time:".blue(),
                        start.elapsed()
                    );
                }

                _ => {
                    println!("{}", "Unexpected response.".red());
                }
            }
        }

        Err(e) => {
            println!("{} {}", "Receive Error:".red(), e);
            return false;
        }
    }

    true
}

async fn execute_file(
    handler: &mut ProtocolHandler,
    path: &str,
    node: &str,
) {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("{} {}", "Cannot open file:".red(), e);
            return;
        }
    };

    let statements = contents
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    println!(
        "{} {} statement(s)...",
        "Executing".green(),
        statements.len()
    );

    for (i, stmt) in statements.iter().enumerate() {
        println!(
            "[{}/{}] {}",
            i + 1,
            statements.len(),
            stmt.replace('\n', " ")
        );

        execute_sql(handler, stmt, node).await;
    }

    println!("{}", "Done.".green());
}

#[tokio::main]
async fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7979".to_string());

    let node_id = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "client".to_string());

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            println!("{} {}", "Connection failed:".red(), e);
            return;
        }
    };

    let mut handler =
        ProtocolHandler::new(stream, Box::new(BincodeSerializer));

    print_banner(&addr, &node_id);

    let mut rl = DefaultEditor::new().unwrap();

    let _ = rl.load_history(HISTORY_FILE);

    loop {
        match rl.readline("constella_db> ") {
            Ok(line) => {
                let cmd = line.trim();

                if cmd.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(cmd);

                if cmd == "help" {
                    println!("Commands:");
                    println!("  help");
                    println!("  clear");
                    println!("  run-file <file.sql>");
                    println!();
                    println!("Everything else is sent as SQL.");
                    continue;
                }

                if cmd == "clear" {
                    print!("\x1B[2J\x1B[1;1H");
                    continue;
                }

                if let Some(file) = cmd.strip_prefix("run-file ") {
                    execute_file(
                        &mut handler,
                        file.trim(),
                        &node_id,
                    )
                    .await;
                    continue;
                }

                execute_sql(
                    &mut handler,
                    cmd,
                    &node_id,
                )
                .await;
            }

            Err(ReadlineError::Interrupted) => {
                println!();
                println!(
                    "{}",
                    "Input cancelled. Press Ctrl+D to quit."
                        .yellow()
                );
                continue;
            }

            Err(ReadlineError::Eof) => {
                println!();
                println!("{}", "Goodbye!".green().bold());

                let _ = rl.save_history(HISTORY_FILE);

                break;
            }

            Err(err) => {
                println!("{err}");
                break;
            }
        }
    }
}
