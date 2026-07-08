use db_module::{Condition, Data, Entity, Table};

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
