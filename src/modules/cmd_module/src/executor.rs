use crate::Command;
use db_module::{Engine, Entity};

pub enum ExecuteResult {
  Ok(String),
  Error(String),
  Rows(Vec<Entity>),
}

pub fn execute(engine: &mut Engine, cmd: Command) -> ExecuteResult {
  match cmd {
    Command::CreateTable(t) => {
      match engine.create_table(&t) {
        Ok(_) => ExecuteResult::Ok(format!("OK: table '{}' created", t.name)),
        Err(e) => ExecuteResult::Error(format!("Error: {e}")),
      }
    },

    Command::DropTable(name) => {
      match engine.drop_table(&name) {
        Ok(_) => ExecuteResult::Ok(format!("OK: table '{}' dropped", name)),
        Err(e) => ExecuteResult::Error(format!("Error: {e}")),
      }
    },

    Command::Insert(e) => {
      match engine.insert(&e) {
        Ok(_) => ExecuteResult::Ok(format!("OK: {} row inserted into '{}'", e.data.len(), e.of)),
        Err(e) => ExecuteResult::Error(format!("Error: {e}")),
      }
    },

    Command::Select {
      table,
      attrs,
      conditions,
    } => {
      let attrs: Vec<_> = attrs.iter().map(|s| s.as_str()).collect();
      match engine.select(&table, attrs, conditions) {
        Ok(rows) => ExecuteResult::Rows(rows),
        Err(e) => ExecuteResult::Error(format!("Error: {e}")),
      }
    },

    Command::Update {
      table,
      updates,
      conditions,
    } => {
      match engine.update(&table, updates, conditions) {
        Ok(n) => ExecuteResult::Ok(format!("OK: {n} row(s) updated")),
        Err(e) => ExecuteResult::Error(format!("Error: {e}")),
      }
    },

    Command::Delete { table, conditions } => {
      match engine.delete(&table, conditions) {
        Ok(n) => ExecuteResult::Ok(format!("OK: {n} row(s) deleted")),
        Err(e) => ExecuteResult::Error(format!("Error: {e}")),
      }
    },
  }
}

pub fn format_rows(rows: Vec<Entity>) -> String {
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
