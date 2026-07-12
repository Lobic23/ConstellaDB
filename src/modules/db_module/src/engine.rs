use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::{Path, PathBuf};

use crate::types::{
  Attr, Condition, DB_DIR, Data, Entity, Operator, SCHEMA_FILE, Table, Type, Value,
};

pub struct Engine {
  pub(crate) tables: Vec<Table>,
}

impl Engine {
  pub async fn new() -> Self {
    let db_dir = Path::new(DB_DIR);

    if !fs::try_exists(db_dir).await.unwrap_or(false) {
      fs::create_dir_all(db_dir).await.unwrap();
    }

    let schema_path = db_dir.join(SCHEMA_FILE);

    if !fs::try_exists(&schema_path).await.unwrap_or(false) {
      fs::write(&schema_path, b"[]").await.unwrap();
    }

    Self {
      tables: Self::load_schema().await,
    }
  }

  pub fn get_tables(&self) -> Vec<Table> {
    self.tables.clone()
  }

  pub(crate) async fn save_schema(&self) {
    let path = PathBuf::from(DB_DIR).join(SCHEMA_FILE);
    // serde_json needs a sync Write; serialize to bytes first, then write async.
    let bytes = serde_json::to_vec_pretty(&self.tables).unwrap();
    fs::write(path, bytes).await.unwrap();
  }

  async fn load_schema() -> Vec<Table> {
    let path = PathBuf::from(DB_DIR).join(SCHEMA_FILE);

    // Create the file if it doesn't exist.
    if !fs::try_exists(&path).await.unwrap_or(false) {
      let _ = fs::write(&path, b"[]").await;
      return Vec::new();
    }

    // Read the contents.
    let contents = match fs::read_to_string(&path).await {
      Ok(c) => c,
      Err(_) => return Vec::new(),
    };

    // Empty file -> initialize with an empty schema.
    if contents.trim().is_empty() {
      let _ = fs::write(&path, b"[]").await;
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
        (_, Value::Null) => {}
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

   pub(crate) async fn load_column(&self, table: &Table, attr: &Attr) -> Result<Vec<Value>, String> {
    if !self.table_exists(&table.name) {
      return Err(format!("Table with name '{}' doesn't exist", table.name));
    }

    if !table.attr_exists(&attr.name) {
      return Err(format!(
        "Attribute '{}' doesn't exist in table {}",
        attr.name, table.name
      ));
    }

    let path = PathBuf::from(DB_DIR)
      .join(&table.name)
      .join(format!("{}.col", &attr.name));

    let mut file = File::open(path).await.map_err(|e| e.to_string())?;
    let mut values: Vec<Value> = Vec::new();
    let mut flag = [0u8; 1];

    match attr.data_type {
      Type::Int => {
        let mut buff = [0u8; 4];
        while file.read_exact(&mut flag).await.is_ok() {
          file.read_exact(&mut buff).await.map_err(|e| e.to_string())?;
          values.push(if flag[0] == 0 {
            Value::Null
          } else {
            Value::Int(i32::from_le_bytes(buff))
          });
        }
      }

      Type::VarChar(size) => {
        let mut buff = vec![0u8; size];
        while file.read_exact(&mut flag).await.is_ok() {
          file.read_exact(&mut buff).await.map_err(|e| e.to_string())?;
          values.push(if flag[0] == 0 {
            Value::Null
          } else {
            let s = String::from_utf8_lossy(&buff)
              .trim_end_matches('\0')
              .to_string();
            Value::VarChar(s)
          });
        }
      }
    }

    Ok(values)
  }


   pub(crate) async fn write_column(
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
      .await
      .map_err(|e| e.to_string())?;

    for value in values {
      if value == &Value::Null {
        let payload_size = match &attr.data_type {
          Type::Int => 4,
          Type::VarChar(size) => *size,
        };
        file.write_all(&[0u8]).await.map_err(|e| e.to_string())?;
        file
          .write_all(&vec![0u8; payload_size])
          .await
          .map_err(|e| e.to_string())?;
        continue;
      }

      match (&attr.data_type, value) {
        (Type::Int, Value::Int(v)) => {
          file.write_all(&[1u8]).await.map_err(|e| e.to_string())?;
          file
            .write_all(&v.to_le_bytes())
            .await
            .map_err(|e| e.to_string())?;
        }
        (Type::VarChar(size), Value::VarChar(v)) => {
          file.write_all(&[1u8]).await.map_err(|e| e.to_string())?;
          let mut bytes = v.as_bytes().to_vec();
          bytes.resize(*size, 0);
          file.write_all(&bytes).await.map_err(|e| e.to_string())?;
        }
        _ => return Err("Type mismatch while writing column".into()),
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
          (Value::Null, _, _) | (_, Value::Null, _) => false,
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
