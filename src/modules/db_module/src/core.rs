use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::types::{
  Attr, Condition, DB_DIR, Data, Entity, Operator, SCHEMA_FILE, Table, Type, Value,
};

pub struct Engine {
  pub(crate) tables: Vec<Table>,
}

impl Engine {
  pub fn new() -> Self {
    let db_dir = Path::new(DB_DIR);

    if !db_dir.exists() {
      fs::create_dir_all(db_dir).unwrap();
    }

    let schema_path = db_dir.join(SCHEMA_FILE);

    if !schema_path.exists() {
      fs::write(&schema_path, b"[]").unwrap();
    }

    Self {
      tables: Self::load_schema(),
    }
  }

  pub fn get_tables(&self) -> Vec<Table> {
    self.tables.clone()
  }
  
  pub(crate) fn save_schema(&self) {
    let file = File::create(PathBuf::from(DB_DIR).join(SCHEMA_FILE).to_str().unwrap()).unwrap();

    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &self.tables).unwrap();
  }

  fn load_schema() -> Vec<Table> {
    let path = PathBuf::from(DB_DIR).join(SCHEMA_FILE);

    // Create the file if it doesn't exist.
    if !path.exists() {
      let _ = fs::write(&path, b"[]");
      return Vec::new();
    }

    // Read the contents.
    let contents = match fs::read_to_string(&path) {
      Ok(c) => c,
      Err(_) => return Vec::new(),
    };

    // Empty file -> initialize with an empty schema.
    if contents.trim().is_empty() {
      let _ = fs::write(&path, b"[]");
      return Vec::new();
    }

    // Invalid JSON -> fall back to empty schema.
    serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
  }

  pub(crate) fn table_exists(&self, name: &str) -> bool {
    self.tables.iter().any(|t| t.name == name)
  }

  pub(crate) fn get_table(&self, name: &str) -> Option<&Table> {
    for table in self.tables.iter() {
      if table.name == name {
        return Some(table);
      }
    }
    None
  }

  pub(crate) fn validate_entity_data(&self, table: &Table, entity: &Entity) -> Result<(), String> {
    for attr in &table.attrs {
      // Checking if all the attributes are provided or not
      let data = entity
        .data
        .iter()
        .find(|d| d.name == attr.name)
        .ok_or_else(|| format!("Missing attribute '{}'", attr.name))?;

      // Typechecking the attributes
      match (&attr.data_type, &data.value) {
        (Type::Int, Value::Int(_)) => {}
        (Type::VarChar(_), Value::VarChar(_)) => {}
        _ => {
          return Err(format!(
            "Type mismatch for '{}', required '{}' got '{}'",
            attr.name, attr.data_type, data.value
          ));
        }
      }
    }

    // Check for extra attributes
    for data in &entity.data {
      if !table.attrs.iter().any(|a| a.name == data.name) {
        return Err(format!("Unknown attribute '{}'", data.name));
      }
    }

    Ok(())
  }

  pub(crate) fn load_column(&self, table: &Table, attr: &Attr) -> Result<Vec<Value>, String> {
    if !self.table_exists(&table.name) {
      return Err(format!("Table with name '{}' doesn't exists", table.name));
    }

    if !table.attr_exists(&attr.name) {
      return Err(format!(
        "Attribute '{}' doesn't exists in table {}",
        attr.name, table.name
      ));
    }

    // Open the attribute file
    let path = PathBuf::from(DB_DIR)
      .join(&table.name)
      .join(format!("{}.col", &attr.name));

    let mut file = File::open(path).map_err(|e| e.to_string())?;

    // Final value array
    let mut values: Vec<Value> = Vec::new();

    match attr.data_type {
      Type::Int => {
        // Buffer size should be 4
        let mut buff = [0u8; 4];

        while file.read_exact(&mut buff).is_ok() {
          values.push(Value::Int(i32::from_le_bytes(buff)));
        }
      }

      Type::VarChar(size) => {
        // Buffer size should be size of the varchar size
        let mut buff = vec![0u8; size];

        while file.read_exact(&mut buff).is_ok() {
          let s = String::from_utf8_lossy(&buff)
            .trim_end_matches('\0')
            .to_string();

          values.push(Value::VarChar(s));
        }
      }
    };

    Ok(values)
  }

  pub(crate) fn write_column(
    &self,
    table: &Table,
    attr: &Attr,
    values: &[Value],
  ) -> Result<(), String> {
    let path = PathBuf::from(DB_DIR)
      .join(&table.name)
      .join(format!("{}.col", attr.name));

    let mut file = OpenOptions::new()
      .write(true)
      .truncate(true)
      .open(path)
      .map_err(|e| e.to_string())?;

    for value in values {
      match (&attr.data_type, value) {
        (Type::Int, Value::Int(v)) => {
          file
            .write_all(&v.to_le_bytes())
            .map_err(|e| e.to_string())?;
        }

        (Type::VarChar(size), Value::VarChar(v)) => {
          let mut bytes = v.as_bytes().to_vec();
          bytes.resize(*size, 0);

          file.write_all(&bytes).map_err(|e| e.to_string())?;
        }

        _ => {
          return Err("Type mismatch while writing column".into());
        }
      }
    }

    Ok(())
  }

  pub(crate) fn match_condition(&self, entity: &Entity, condition: &Condition) -> bool {
    match condition {
      Condition::Compare { attr, value, op } => {
        // Find the attribute for the compare
        let Some(data) = entity.data.iter().find(|d| d.name == *attr) else {
          return false;
        };

        match (&data.value, value, op) {
          (Value::Int(a), Value::Int(b), Operator::Eq) => a == b,
          (Value::Int(a), Value::Int(b), Operator::Ne) => a != b,
          (Value::Int(a), Value::Int(b), Operator::Gt) => a > b,
          (Value::Int(a), Value::Int(b), Operator::Ge) => a >= b,
          (Value::Int(a), Value::Int(b), Operator::Lt) => a < b,
          (Value::Int(a), Value::Int(b), Operator::Le) => a <= b,

          (Value::VarChar(a), Value::VarChar(b), Operator::Eq) => a == b,
          (Value::VarChar(a), Value::VarChar(b), Operator::Ne) => a != b,

          _ => false,
        }
      }

      Condition::And(left, right) => {
        self.match_condition(entity, left) && self.match_condition(entity, right)
      }

      Condition::Or(left, right) => {
        self.match_condition(entity, left) || self.match_condition(entity, right)
      }
    }
  }

  pub(crate) fn build_entity(&self, table: &Table, columns: &[Vec<Value>], row: usize) -> Entity {
    let mut data = Vec::new();

    for (idx, attr) in table.attrs.iter().enumerate() {
      data.push(Data {
        name: attr.name.clone(),
        value: columns[idx][row].clone(),
      });
    }

    Entity {
      of: table.name.clone(),
      data,
    }
  }
}
