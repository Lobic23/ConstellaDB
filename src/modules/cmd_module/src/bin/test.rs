use cmd_module::{execute, parse_cmd};
use db_module::Engine;

fn main() {
  let mut engine = Engine::new();

  let commands = vec![
    "CREATE TABLE people (id INT, name VARCHAR(50), age INT)",
    "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)",
    "INSERT INTO people (id, name, age) VALUES (2, 'Bob', 17)",
    "INSERT INTO people (id, name, age) VALUES (3, 'Charlie', 25)",
    "READ id, name FROM people WHERE age > 18",
    "SELECT * FROM people WHERE name = 'Bob' OR age >= 25",
    "UPDATE people SET age = 31 WHERE name = 'Alice'",
    "SELECT id, age FROM people WHERE id = 1",
    "DELETE FROM people WHERE age < 18",
    "SELECT * FROM people",
  ];

  for sql in commands {
    println!("\n>>> {sql}");

    let output = match parse_cmd(sql) {
      Ok(cmd) => execute(&mut engine, cmd),
      Err(e) => format!("Parse error: {e}"),
    };

    println!("{output}");
  }
}
