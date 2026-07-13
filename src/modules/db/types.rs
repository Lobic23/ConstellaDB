use serde::{Deserialize, Serialize};
use std::fmt;

pub const DB_DIR: &str = "DB";
pub const SCHEMA_FILE: &str = "schemas.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
  Eq, // ==
  Ne, // !=
  Lt, // <
  Le, // <=
  Gt, // >
  Ge, // >=
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
  Compare {
    attr: String,
    value: Value,
    op: Operator,
  },
  And(Box<Condition>, Box<Condition>),
  Or(Box<Condition>, Box<Condition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
  Int,
  VarChar(usize),
}

impl fmt::Display for Type {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Type::Int => write!(f, "INT"),
      Type::VarChar(size) => write!(f, "VARCHAR({})", size),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attr {
  pub name: String,
  pub data_type: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
  pub name: String,
  pub attrs: Vec<Attr>,
}

impl Table {
  pub fn attr_exists(&self, attr_name: &str) -> bool {
    for attr in &self.attrs {
      if attr_name == attr.name {
        return true;
      }
    }
    return false;
  }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Value {
  Int(i32),
  VarChar(String),
  Null,
  // NULL is stored as a 0x00 flag byte followed by zero-filled payload bytes to maintain fixed record size.
  // Non-NULL values are stored as a 0x01 flag byte followed by the value encoded in little-endian bytes.
}
impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Value::Int(_) => write!(f, "INT"),
      Value::VarChar(_) => write!(f, "VARCHAR"),
      Value::Null => write!(f, "NULL"),
    }
  }
}

impl Value {
  pub fn cast_to(self, target: &Type) -> Result<Value, String> {
    match (self, target) {
      (Value::Null, _) => Ok(Value::Null),
      (Value::Int(v), Type::Int) => Ok(Value::Int(v)),
      (Value::VarChar(s), Type::VarChar(_)) => Ok(Value::VarChar(s)),
      (Value::Int(v), Type::VarChar(_)) => Ok(Value::VarChar(v.to_string())),
      (Value::VarChar(s), Type::Int) => s
        .trim()
        .parse::<i32>()
        .map(Value::Int)
        .map_err(|_| format!("Cannot cast '{}' to INT", s)),
    }
  }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Data {
  pub name: String,
  pub value: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Entity {
  pub of: String,
  pub data: Vec<Data>,
}
