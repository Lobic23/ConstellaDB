use sqlparser::ast::{
  BinaryOperator, CharacterLength, DataType as SqlDataType, Expr as SqlExpr, FromTable, SetExpr,
  Statement as SqlStatement, Value as SqlValue,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser as SqlParser;

use crate::ast::Command;
use crate::error::CmdError;
use db_module::{Attr, Condition, Data, Entity, Operator, Table, Type, Value};

pub fn parse_cmd(input: &str) -> Result<Command, CmdError> {
  let dialect = GenericDialect {};
  let mut stmts =
    SqlParser::parse_sql(&dialect, input).map_err(|e| CmdError::Syntax(e.to_string()))?;

  if stmts.is_empty() {
    return Err(CmdError::Empty);
  }
  convert_statement(stmts.remove(0))
}

fn convert_statement(stmt: SqlStatement) -> Result<Command, CmdError> {
  match stmt {
    SqlStatement::CreateTable { name, columns, .. } => {
      let attrs = columns
        .into_iter()
        .map(|c| {
          Ok(Attr {
            name: c.name.value,
            data_type: convert_data_type(c.data_type)?,
          })
        })
        .collect::<Result<Vec<_>, CmdError>>()?;
      Ok(Command::CreateTable(Table {
        name: name.to_string(),
        attrs,
      }))
    }

    SqlStatement::ShowTables { .. } => Ok(Command::ShowTables),

    SqlStatement::Drop { names, .. } => {
      let name = names
        .into_iter()
        .next()
        .ok_or_else(|| CmdError::Syntax("missing table name".into()))?
        .to_string();
      Ok(Command::DropTable(name))
    }

    SqlStatement::Insert {
      table_name,
      columns,
      source,
      ..
    } => {
      let col_names: Vec<String> = columns.into_iter().map(|c| c.value).collect();
      let source = source.ok_or_else(|| CmdError::Syntax("INSERT requires VALUES".into()))?;
      let rows = extract_insert_rows(*source)?;
      let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| CmdError::Syntax("INSERT requires a row of values".into()))?;
      if row.len() != col_names.len() {
        return Err(CmdError::Syntax("column/value count mismatch".into()));
      }
      let data = col_names
        .into_iter()
        .zip(row)
        .map(|(name, value)| Data { name, value })
        .collect();
      Ok(Command::Insert(Entity {
        of: table_name.to_string(),
        data,
      }))
    }

    SqlStatement::Query(query) => convert_select(*query),

    SqlStatement::Update {
      table,
      assignments,
      selection,
      ..
    } => {
      let updates = assignments
        .into_iter()
        .map(|a| {
          let name = a.id.last().map(|i| i.value.clone()).unwrap_or_default();
          let value = convert_value(a.value)?;
          Ok(Data { name, value })
        })
        .collect::<Result<Vec<_>, CmdError>>()?;
      let conditions = match selection {
        Some(e) => vec![convert_expr(e)?],
        None => vec![],
      };
      Ok(Command::Update {
        table: table.relation.to_string(),
        updates,
        conditions,
      })
    }

    SqlStatement::Delete {
      from, selection, ..
    } => {
      let tables = match from {
        FromTable::WithFromKeyword(t) | FromTable::WithoutKeyword(t) => t,
      };
      let table = tables
        .into_iter()
        .next()
        .ok_or_else(|| CmdError::Syntax("missing table in DELETE".into()))?
        .relation
        .to_string();
      let conditions = match selection {
        Some(e) => vec![convert_expr(e)?],
        None => vec![],
      };
      Ok(Command::Delete { table, conditions })
    }
    other => Err(CmdError::Unsupported(other.to_string())),
  }
}

fn convert_select(query: sqlparser::ast::Query) -> Result<Command, CmdError> {
  let SetExpr::Select(select) = *query.body else {
    return Err(CmdError::Unsupported("non-SELECT query".into()));
  };
  let table = select
    .from
    .into_iter()
    .next()
    .ok_or_else(|| CmdError::Syntax("missing FROM".into()))?
    .relation
    .to_string();

  let attrs = select
    .projection
    .into_iter()
    .map(|item| match item {
      sqlparser::ast::SelectItem::UnnamedExpr(SqlExpr::Identifier(id)) => Ok(id.value),
      sqlparser::ast::SelectItem::Wildcard(_) => Ok("*".to_string()),
      other => Err(CmdError::UnsupportedExpr(format!("{:?}", other))),
    })
    .collect::<Result<Vec<_>, _>>()?;

  let conditions = match select.selection {
    Some(e) => vec![convert_expr(e)?],
    None => vec![],
  };

  Ok(Command::Select {
    table,
    attrs,
    conditions,
  })
}

fn convert_data_type(dt: SqlDataType) -> Result<Type, CmdError> {
  match dt {
    SqlDataType::Int(_) | SqlDataType::Integer(_) => Ok(Type::Int),
    SqlDataType::Varchar(Some(CharacterLength::IntegerLength { length, .. })) => {
      Ok(Type::VarChar(length as usize))
    }
    other => Err(CmdError::Unsupported(format!("data type {:?}", other))),
  }
}

fn convert_expr(expr: SqlExpr) -> Result<Condition, CmdError> {
  match expr {
    SqlExpr::BinaryOp { left, op, right } => match op {
      BinaryOperator::And => Ok(Condition::And(
        Box::new(convert_expr(*left)?),
        Box::new(convert_expr(*right)?),
      )),
      BinaryOperator::Or => Ok(Condition::Or(
        Box::new(convert_expr(*left)?),
        Box::new(convert_expr(*right)?),
      )),
      BinaryOperator::Eq
      | BinaryOperator::NotEq
      | BinaryOperator::Lt
      | BinaryOperator::Gt
      | BinaryOperator::LtEq
      | BinaryOperator::GtEq => {
        let attr = match *left {
          SqlExpr::Identifier(id) => id.value,
          other => return Err(CmdError::UnsupportedExpr(format!("{:?}", other))),
        };
        let value = convert_value(*right)?;
        let cmp_op = match op {
          BinaryOperator::Eq => Operator::Eq,
          BinaryOperator::NotEq => Operator::Ne,
          BinaryOperator::Lt => Operator::Lt,
          BinaryOperator::Gt => Operator::Gt,
          BinaryOperator::LtEq => Operator::Le,
          BinaryOperator::GtEq => Operator::Ge,
          _ => unreachable!(),
        };
        Ok(Condition::Compare {
          attr,
          value,
          op: cmp_op,
        })
      }
      other => Err(CmdError::UnsupportedExpr(format!("{:?}", other))),
    },
    SqlExpr::Nested(inner) => convert_expr(*inner),
    other => Err(CmdError::UnsupportedExpr(format!("{:?}", other))),
  }
}

fn convert_value(expr: SqlExpr) -> Result<Value, CmdError> {
  match expr {
    SqlExpr::Value(SqlValue::Number(n, _)) => n
      .parse::<i32>()
      .map(Value::Int)
      .map_err(|_| CmdError::Syntax(format!("invalid integer: {n}"))),
    SqlExpr::Value(SqlValue::SingleQuotedString(s)) => Ok(Value::VarChar(s)),
    other => Err(CmdError::UnsupportedExpr(format!("{:?}", other))),
  }
}

fn extract_insert_rows(source: sqlparser::ast::Query) -> Result<Vec<Vec<Value>>, CmdError> {
  let SetExpr::Values(values) = *source.body else {
    return Err(CmdError::Unsupported("INSERT without VALUES".into()));
  };
  values
    .rows
    .into_iter()
    .map(|row| row.into_iter().map(convert_value).collect())
    .collect()
}
