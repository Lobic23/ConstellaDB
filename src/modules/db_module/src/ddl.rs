use std::path::PathBuf;
use tokio::fs::{self, File};

use crate::engine::Engine;
use crate::types::{Attr, DB_DIR, Table, Type, Value};

pub enum AlterOp {
  AddColumn(Attr),
  DropColumn(String),
  RenameColumn { from: String, to: String },
  ModifyColumnType { name: String, new_type: Type },
}

impl Engine {
  pub async fn create_table(&mut self, table: &Table) -> Result<(), String> {
    if self.table_exists(&table.name) {
      return Err(format!("Table with name '{}' already exists", table.name));
    }

    // Create the table dir
    let table_dir_path = PathBuf::from(DB_DIR).join(&table.name);

    fs::create_dir_all(table_dir_path.to_str().unwrap())
      .await
      .unwrap();

    // Create attribute files
    for attr in table.attrs.iter() {
      let attr_file_path = table_dir_path.clone().join(format!("{}.col", attr.name));
      
      File::create(attr_file_path.to_str().unwrap())
        .await
        .unwrap();
    }

    // Store the table
    self.tables.push(table.clone());

    // Save the schema
    self.save_schema().await;

    Ok(())
  }

  pub async fn list_tables(&self) -> Result<Vec<String>, String> {
    if self.tables.is_empty() {
      return Err("No tables exist".to_string());
    }
    Ok(self.tables.iter().map(|table| table.name.clone()).collect())
  }

  pub async fn drop_table(&mut self, table_name: &str) -> Result<(), String> {
    if !self.table_exists(table_name) {
      return Err(format!("Table with name '{}' doesnt exists", table_name));
    }

    // Remove from the filesystem
    let path = PathBuf::from(DB_DIR).join(table_name);
    fs::remove_dir_all(path).await.map_err(|e| e.to_string())?;

    // Remove from schema
    self.tables.retain(|t| t.name != table_name);
    self.save_schema().await;

    Ok(())
  }

  pub async fn alter_table(&mut self, table_name: &str, op: AlterOp) -> Result<(), String> {
    let table = self
      .get_table(table_name)
      .ok_or_else(|| format!("Table '{}' doesn't exist", table_name))?
      .clone();

    match op {
      AlterOp::AddColumn(attr) => self.add_column(&table, attr).await,
      AlterOp::DropColumn(name) => self.drop_column(&table, &name).await,
      AlterOp::RenameColumn { from, to } => self.rename_column(&table, &from, &to).await,
      AlterOp::ModifyColumnType { name, new_type } => {
        self.modify_column_type(&table, &name, new_type).await
      }
    }
  }

  async fn add_column(&mut self, table: &Table, attr: Attr) -> Result<(), String> {
    if table.attr_exists(&attr.name) {
      return Err(format!(
        "Column '{}' already exists in table '{}'",
        attr.name, table.name
      ));
    }

    let path = PathBuf::from(DB_DIR)
      .join(&table.name)
      .join(format!("{}.col", attr.name));

    File::create(&path).await.map_err(|e| e.to_string())?;

    // Fill with Null for every existing row so column lengths stay in sync
    let row_count = self.row_count(table).await?;
    if row_count > 0 {
      let nulls = vec![Value::Null; row_count];
      self.write_column(table, &attr, &nulls).await?;
    }

    let t = self
      .tables
      .iter_mut()
      .find(|t| t.name == table.name)
      .unwrap();
    t.attrs.push(attr);
    self.save_schema().await;

    Ok(())
  }

  async fn drop_column(&mut self, table: &Table, col_name: &str) -> Result<(), String> {
    if !table.attr_exists(col_name) {
      return Err(format!(
        "Column '{}' doesn't exist in table '{}'",
        col_name, table.name
      ));
    }

    if table.attrs.len() == 1 {
      return Err(format!(
        "Cannot drop the only column in table '{}'; drop the table instead",
        table.name
      ));
    }

    // Remove the column file

    let path = PathBuf::from(DB_DIR)
      .join(&table.name)
      .join(format!("{}.col", col_name));

    fs::remove_file(&path).await.map_err(|e| e.to_string())?;

    // Update schema
    let t = self
      .tables
      .iter_mut()
      .find(|t| t.name == table.name)
      .unwrap();
    t.attrs.retain(|a| a.name != col_name);
    self.save_schema().await;

    Ok(())
  }

  async fn rename_column(&mut self, table: &Table, from: &str, to: &str) -> Result<(), String> {
    if !table.attr_exists(from) {
      return Err(format!(
        "Column '{}' doesn't exist in table '{}'",
        from, table.name
      ));
    }

    if table.attr_exists(to) {
      return Err(format!(
        "Column '{}' already exists in table '{}'",
        to, table.name
      ));
    }

    let table_dir = PathBuf::from(DB_DIR).join(&table.name);
    let old_path = table_dir.join(format!("{}.col", from));
    let new_path = table_dir.join(format!("{}.col", to));

    fs::rename(&old_path, &new_path)
      .await
      .map_err(|e| e.to_string())?;

    // Update schema
    let t = self
      .tables
      .iter_mut()
      .find(|t| t.name == table.name)
      .unwrap();
    let attr = t.attrs.iter_mut().find(|a| a.name == from).unwrap();
    attr.name = to.to_string();
    self.save_schema().await;

    Ok(())
  }

  async fn modify_column_type(
    &mut self,
    table: &Table,
    col_name: &str,
    new_type: Type,
  ) -> Result<(), String> {
    let attr = table
      .attrs
      .iter()
      .find(|a| a.name == col_name)
      .ok_or_else(|| {
        format!(
          "Column '{}' doesn't exist in table '{}'",
          col_name, table.name
        )
      })?;

    let values = self.load_column(table, attr).await?;

    let new_values: Vec<Value> = values
      .into_iter()
      .map(|v| v.cast_to(&new_type))
      .collect::<Result<_, _>>()?;

    // Update schema — scope the mutable borrow so it drops before save_schema
    let updated_attr = {
      let t = self
        .tables
        .iter_mut()
        .find(|t| t.name == table.name)
        .unwrap();
      let a = t.attrs.iter_mut().find(|a| a.name == col_name).unwrap();
      a.data_type = new_type;
      a.clone() // pull out what we need before t drops
    };

    self.save_schema().await;
    self.write_column(table, &updated_attr, &new_values).await?;

    Ok(())
  }

  pub(crate) async fn row_count(&self, table: &Table) -> Result<usize, String> {
    let first_attr = match table.attrs.first() {
      Some(a) => a,
      None => return Ok(0),
    };
    Ok(self.load_column(table, first_attr).await?.len())
  }
}
