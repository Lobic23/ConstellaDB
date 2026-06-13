use db_module::{Table, Entity, Data, Condition};

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
}
