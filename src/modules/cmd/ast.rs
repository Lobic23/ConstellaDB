use serde::{Deserialize, Serialize};
use crate::modules::db::{Condition, Data, Entity, Table};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
  CreateTable(Table),
  DropTable(String),
  Insert(Entity),
  Select {
    table: String,
    attrs: Vec<String>,
    conditions: Vec<Condition>,
  },
  Update {
    table: String,
    updates: Vec<Data>,
    conditions: Vec<Condition>,
  },
  Delete {
    table: String,
    conditions: Vec<Condition>,
  },
  ShowTables
}
