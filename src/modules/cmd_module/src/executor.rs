use crate::Command;
use db_module::{Engine, Entity};

pub fn execute(engine: &mut Engine, cmd: Command) -> String {
  match cmd {
    Command::CreateTable(t) => engine
      .create_table(&t)
      .map(|_| format!("OK: table '{}' created", t.name))
      .unwrap_or_else(|e| format!("Error: {e}")),

    Command::DropTable(name) => engine
      .drop_table(&name)
      .map(|_| format!("OK: table '{}' dropped", name))
      .unwrap_or_else(|e| format!("Error: {e}")),

    Command::Insert(e) => engine
      .insert(&e)
      .map(|_| format!("OK: 1 row inserted into '{}'", e.of))
      .unwrap_or_else(|e| format!("Error: {e}")),

    Command::Select {
      table,
      attrs,
      conditions,
    } => {
      let attrs: Vec<_> = attrs.iter().map(|s| s.as_str()).collect();
      match engine.select(&table, attrs, conditions) {
        Ok(rows) => format_rows(rows),
        Err(e) => format!("Error: {e}"),
      }
    }

    Command::Update {
      table,
      updates,
      conditions,
    } => engine
      .update(&table, updates, conditions)
      .map(|n| format!("OK: {n} row(s) updated"))
      .unwrap_or_else(|e| format!("Error: {e}")),

    Command::Delete { table, conditions } => engine
      .delete(&table, conditions)
      .map(|n| format!("OK: {n} row(s) deleted"))
      .unwrap_or_else(|e| format!("Error: {e}")),
  }
}

fn format_rows(rows: Vec<Entity>) -> String {
  if rows.is_empty() {
    return "OK: 0 row(s)".to_string();
  }
  let mut out = format!("OK: {} row(s)\n", rows.len());
  for row in &rows {
    let fields: Vec<String> = row
      .data
      .iter()
      .map(|d| format!("{}={:?}", d.name, d.value))
      .collect();
    out.push_str(&format!("  {}\n", fields.join(", ")));
  }
  out.trim_end().to_string()
}
