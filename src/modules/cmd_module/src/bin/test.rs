use cmd_module::{parse_command, Command};
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
        match parse_command(sql) {
            Ok(cmd) => run(&mut engine, cmd),
            Err(e) => println!("Parse error: {e}"),
        }
    }
}

fn run(engine: &mut Engine, cmd: Command) {
    match cmd {
        Command::CreateTable(table) => {
            match engine.create_table(&table) {
                Ok(_) => println!("OK: table '{}' created", table.name),
                Err(e) => println!("Error: {e}"),
            }
        }
        Command::DropTable(name) => {
            match engine.drop_table(&name) {
                Ok(_) => println!("OK: table '{}' dropped", name),
                Err(e) => println!("Error: {e}"),
            }
        }
        Command::Insert(entity) => {
            match engine.insert(&entity) {
                Ok(_) => println!("OK: 1 row inserted into '{}'", entity.of),
                Err(e) => println!("Error: {e}"),
            }
        }
        Command::Select { table, attrs, conditions } => {
            let attrs = if attrs == ["*"] {
               //hardcoded for now
               vec!["id", "name", "age"].iter().map(|s| s.to_string()).collect()
            } else {
                attrs
            };
            let attr_refs: Vec<&str> = attrs.iter().map(|s| s.as_str()).collect();
            match engine.select(&table, attr_refs, conditions) {
                Ok(rows) => {
                    println!("OK: {} row(s)", rows.len());
                    for row in &rows {
                        let fields: Vec<String> = row.data.iter()
                            .map(|d| format!("{}={:?}", d.name, d.value))
                            .collect();
                        println!("  {}", fields.join(", "));
                    }
                }
                Err(e) => println!("Error: {e}"),
            }
        }
        Command::Update { table, updates, conditions } => {
            match engine.update(&table, updates, conditions) {
                Ok(n) => println!("OK: {n} row(s) updated"),
                Err(e) => println!("Error: {e}"),
            }
        }
        Command::Delete { table, conditions } => {
            match engine.delete(&table, conditions) {
                Ok(n) => println!("OK: {n} row(s) deleted"),
                Err(e) => println!("Error: {e}"),
            }
        }
    }
}


