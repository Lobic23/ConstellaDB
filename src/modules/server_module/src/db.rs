use cmd_module::Command;
use db_module::{Engine, Entity};

pub fn execute(engine: &mut Engine, cmd: Command) -> String {
    match cmd {
        Command::CreateTable(table) => {
            match engine.create_table(&table) {
                Ok(_)  => format!("OK: table '{}' created", table.name),
                Err(e) => format!("Error: {e}"),
            }
        }

        Command::DropTable(name) => {
            match engine.drop_table(&name) {
                Ok(_)  => format!("OK: table '{}' dropped", name),
                Err(e) => format!("Error: {e}"),
            }
        }

        Command::Insert(entity) => {
            match engine.insert(&entity) {
                Ok(_)  => format!("OK: 1 row inserted into '{}'", entity.of),
                Err(e) => format!("Error: {e}"),
            }
        }

        Command::Select { table, attrs, conditions } => {
            let attrs = if attrs == ["*"] {
                vec!["id".to_string(), "name".to_string(), "age".to_string()]
            } else {
                attrs
            };
            let attr_refs: Vec<&str> = attrs.iter().map(|s| s.as_str()).collect();

            match engine.select(&table, attr_refs, conditions) {
                Ok(rows) => format_rows(rows),
                Err(e)   => format!("Error: {e}"),
            }
        }

        Command::Update { table, updates, conditions } => {
            match engine.update(&table, updates, conditions) {
                Ok(n)  => format!("OK: {n} row(s) updated"),
                Err(e) => format!("Error: {e}"),
            }
        }

        Command::Delete { table, conditions } => {
            match engine.delete(&table, conditions) {
                Ok(n)  => format!("OK: {n} row(s) deleted"),
                Err(e) => format!("Error: {e}"),
            }
        }
    }
}

fn format_rows(rows: Vec<Entity>) -> String {
    if rows.is_empty() {
        return "OK: 0 row(s)".to_string();
    }
    let mut out = format!("OK: {} row(s)\n", rows.len());
    for row in &rows {
        let fields: Vec<String> = row.data
            .iter()
            .map(|d| format!("{}={:?}", d.name, d.value))
            .collect();
        out.push_str(&format!("  {}\n", fields.join(", ")));
    }
    out.trim_end().to_string()
}