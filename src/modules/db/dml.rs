use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::modules::db::engine::Engine;
use crate::modules::db::types::{Condition, DB_DIR, Data, Entity, Type, Value};

impl Engine {
  pub async fn insert(&mut self, entity: &Entity) -> Result<String, String> {
    let table = self
      .get_table(&entity.of)
      .ok_or_else(|| format!("Table '{}' doesn't exist", entity.of))?
      .clone();

    // Validate the data attributes
    self.validate_entity_data(&table, entity)?;

    for data in &entity.data {
      let attr = table.attrs.iter().find(|a| a.name == data.name).unwrap();
      let path = PathBuf::from(DB_DIR)
        .join(&table.name)
        .join(format!("{}.col", &data.name));

      // Open the required attribute file
      let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.to_str().unwrap())
        .await
        .unwrap();

      // Store the byte
      // NULL is stored as a 0x00 flag byte followed by zero-filled payload bytes to maintain fixed record size.
      // Non-NULL values are stored as a 0x01 flag byte followed by the value encoded in little-endian bytes.
      match (&attr.data_type, &data.value) {
        (_, Value::Null) => {
          let payload_size = match &attr.data_type {
            Type::Int => 4,
            Type::VarChar(size) => *size,
          };
          file.write_all(&[0u8]).await.map_err(|e| e.to_string())?;
          file
            .write_all(&vec![0u8; payload_size])
            .await
            .map_err(|e| e.to_string())?;
        }
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
        _ => return Err("Unreachable!".to_string()),
      }
    }

    Ok("Sucessfully inserted 1 Row".to_string())
  }

  pub async fn select(
    &mut self,
    table_name: &str,
    attrs: Vec<&str>,
    conditions: Vec<Condition>,
  ) -> Result<Vec<Entity>, String> {
    let table = match self.get_table(table_name) {
      Some(t) => t,
      None => return Err(format!("Table with name '{}' doesn't exists", table_name)),
    };

    let select_all = attrs.contains(&"*");

    if !select_all {
      // Verify attributes
      for attr in &attrs {
        if !table.attr_exists(attr) {
          return Err(format!(
            "Attribute '{}' doesn't exists in table {}",
            attr, table.name
          ));
        }
      }
    }

    // Load all the columns
    let mut columns: Vec<Vec<Value>> = Vec::new();
    for attr in &table.attrs {
      columns.push(self.load_column(table, attr).await?);
    }

    // If no columns were fetched then return empty
    if columns.is_empty() {
      return Ok(Vec::new());
    }

    let row_count = columns[0].len();
    let mut result: Vec<Entity> = Vec::new();

    for row in 0..row_count {
      let entity = self.build_entity(&table, &columns, row);

      // Check the condition
      let matches = conditions.iter().all(|c| self.match_condition(&entity, c));

      // If all condition passes then its the result
      if matches {
        let filtered_data: Vec<Data> = if select_all {
          entity.data
        } else {
          entity
            .data
            .into_iter()
            .filter(|d| attrs.contains(&d.name.as_str()))
            .collect()
        };

        result.push(Entity {
          of: entity.of,
          data: filtered_data,
        });
      }
    }

    Ok(result)
  }

  pub async fn delete(
    &mut self,
    table_name: &str,
    conditions: Vec<Condition>,
  ) -> Result<usize, String> {
    let table = match self.get_table(table_name) {
      Some(t) => t,
      None => return Err(format!("Table with name '{}' doesn't exists", table_name)),
    };

    let mut columns = Vec::new();
    for attr in &table.attrs {
      columns.push(self.load_column(&table, attr).await?);
    }

    if columns.is_empty() {
      return Ok(0);
    }

    let row_count = columns[0].len();

    // The columns that is going to rewrite the db
    let mut new_columns: Vec<Vec<Value>> = vec![Vec::new(); columns.len()];
    let mut deleted = 0;

    for row in 0..row_count {
      let entity = self.build_entity(&table, &columns, row);

      let matches = conditions.iter().all(|c| self.match_condition(&entity, c));

      if matches {
        deleted += 1;
        continue;
      }

      // Add only the columns that donot match
      for col in 0..columns.len() {
        new_columns[col].push(columns[col][row].clone());
      }
    }

    // Write the new column
    for (idx, attr) in table.attrs.iter().enumerate() {
      self.write_column(&table, attr, &new_columns[idx]).await?;
    }

    Ok(deleted)
  }

  pub async fn update(
    &mut self,
    table_name: &str,
    updates: Vec<Data>,
    conditions: Vec<Condition>,
  ) -> Result<usize, String> {
    let table = match self.get_table(table_name) {
      Some(t) => t,
      None => return Err(format!("Table with name '{}' doesn't exists", table_name)),
    };

    let mut columns = Vec::new();
    for attr in &table.attrs {
      columns.push(self.load_column(&table, attr).await?);
    }

    if columns.is_empty() {
      return Ok(0);
    }

    let row_count = columns[0].len();
    let mut updated = 0;
    for row in 0..row_count {
      let entity = self.build_entity(&table, &columns, row);
      let matches = conditions.iter().all(|c| self.match_condition(&entity, c));

      if !matches {
        continue;
      }

      updated += 1;
      for update in &updates {

        // Calculate the index of the column
        let col_idx = table
          .attrs
          .iter()
          .position(|a| a.name == update.name)
          .ok_or_else(|| format!("Unknown attribute '{}'", update.name))?;

        // Override the column data
        columns[col_idx][row] = update.value.clone();
      }
    }

    // Write the columns again
    for (idx, attr) in table.attrs.iter().enumerate() {
      self.write_column(&table, attr, &columns[idx]).await?;
    }

    Ok(updated)
  }
}
