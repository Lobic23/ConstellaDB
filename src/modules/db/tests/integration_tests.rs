// src/modules/db_module >  rm -rf DB/ && cargo test -- --test-threads=1  
// TODO : make shit work without the --test-threads=1 flag


#[cfg(test)]
mod tests {
  use db_module::*;

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  /// Builds a fresh engine with `user_table` already created and populated
  /// with 10 rows (id 0–9).  The table is dropped first so tests that call
  /// this are hermetic even when the DB directory is shared on disk.
  async fn setup_user_table() -> Engine {
    let mut engine = Engine::new().await;

    let _ = engine.drop_table("user_table").await;

    let user_table = Table {
      name: "user_table".to_string(),
      attrs: vec![
        Attr {
          name: "id".to_string(),
          data_type: Type::Int,
        },
        Attr {
          name: "name".to_string(),
          data_type: Type::VarChar(100),
        },
        Attr {
          name: "password".to_string(),
          data_type: Type::VarChar(8),
        },
      ],
    };

    engine.create_table(&user_table).await.unwrap();

    for i in 0..10_i32 {
      engine
        .insert(&Entity {
          of: "user_table".to_string(),
          data: vec![
            Data {
              name: "id".to_string(),
              value: Value::Int(i),
            },
            Data {
              name: "name".to_string(),
              value: Value::VarChar(format!("user{}", i)),
            },
            Data {
              name: "password".to_string(),
              value: Value::VarChar("12345678".to_string()),
            },
          ],
        })
        .await
        .unwrap();
    }

    engine
  }

  // -------------------------------------------------------------------------
  // DDL
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn create_table_succeeds() {
    let mut engine = Engine::new().await;
    let _ = engine.drop_table("ddl_test").await;

    let table = Table {
      name: "ddl_test".to_string(),
      attrs: vec![Attr {
        name: "id".to_string(),
        data_type: Type::Int,
      }],
    };

    assert!(engine.create_table(&table).await.is_ok());
    assert!(engine.select("ddl_test", vec!["*"], vec![]).await.is_ok());

    let _ = engine.drop_table("ddl_test").await;
  }

  #[tokio::test]
  async fn create_table_duplicate_errors() {
    let mut engine = Engine::new().await;
    let _ = engine.drop_table("dup_test").await;

    let table = Table {
      name: "dup_test".to_string(),
      attrs: vec![Attr {
        name: "id".to_string(),
        data_type: Type::Int,
      }],
    };

    engine.create_table(&table).await.unwrap();
    let result = engine.create_table(&table).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));

    let _ = engine.drop_table("dup_test").await;
  }

  #[tokio::test]
  async fn drop_table_succeeds() {
    let mut engine = Engine::new().await;
    let _ = engine.drop_table("drop_test").await;

    let table = Table {
      name: "drop_test".to_string(),
      attrs: vec![Attr {
        name: "id".to_string(),
        data_type: Type::Int,
      }],
    };

    engine.create_table(&table).await.unwrap();
    assert!(engine.drop_table("drop_test").await.is_ok());
    assert!(engine.select("drop_test", vec!["*"], vec![]).await.is_err());
  }

  #[tokio::test]
  async fn drop_nonexistent_table_errors() {
    let mut engine = Engine::new().await;
    let result = engine.drop_table("no_such_table").await;
    assert!(result.is_err());
  }

  // -------------------------------------------------------------------------
  // ALTER TABLE — add / drop / rename column
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn alter_add_column_succeeds() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table(
      "user_table",
      AlterOp::AddColumn(Attr {
        name: "email".to_string(),
        data_type: Type::VarChar(255),
      }),
    ).await;

    assert!(result.is_ok(), "{:?}", result);

    let rows = engine.select("user_table", vec!["email"], vec![]).await;
    assert!(rows.is_ok());
  }

  #[tokio::test]
  async fn alter_add_duplicate_column_errors() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table(
      "user_table",
      AlterOp::AddColumn(Attr {
        name: "id".to_string(),
        data_type: Type::Int,
      }),
    ).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
  }

  #[tokio::test]
  async fn alter_drop_column_succeeds() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table("user_table", AlterOp::DropColumn("password".to_string())).await;

    assert!(result.is_ok(), "{:?}", result);

    assert!(
      engine
        .select("user_table", vec!["password"], vec![])
        .await
        .is_err()
    );
  }

  #[tokio::test]
  async fn alter_drop_nonexistent_column_errors() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table("user_table", AlterOp::DropColumn("no_col".to_string())).await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn alter_drop_last_column_errors() {
    let mut engine = Engine::new().await;
    let _ = engine.drop_table("single_col").await;

    let table = Table {
      name: "single_col".to_string(),
      attrs: vec![Attr {
        name: "only".to_string(),
        data_type: Type::Int,
      }],
    };
    engine.create_table(&table).await.unwrap();

    let result = engine.alter_table("single_col", AlterOp::DropColumn("only".to_string())).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("drop the table instead"));

    let _ = engine.drop_table("single_col").await;
  }

  #[tokio::test]
  async fn alter_rename_column_succeeds() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table(
      "user_table",
      AlterOp::RenameColumn {
        from: "password".to_string(),
        to: "passwd".to_string(),
      },
    ).await;

    assert!(result.is_ok(), "{:?}", result);

    assert!(engine.select("user_table", vec!["passwd"], vec![]).await.is_ok());
    assert!(
      engine
        .select("user_table", vec!["password"], vec![])
        .await
        .is_err()
    );
  }

  #[tokio::test]
  async fn alter_rename_to_existing_column_errors() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table(
      "user_table",
      AlterOp::RenameColumn {
        from: "name".to_string(),
        to: "id".to_string(), // already exists
      },
    ).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
  }

  // -------------------------------------------------------------------------
  // ALTER TABLE — modify_column_type (uses cast_value, not v.cast_to)
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn modify_column_type_int_to_varchar() {
    let mut engine = setup_user_table().await;

    // Cast the Int `id` column to VarChar
    let result = engine.alter_table(
      "user_table",
      AlterOp::ModifyColumnType {
        name: "id".to_string(),
        new_type: Type::VarChar(20),
      },
    ).await;

    assert!(result.is_ok(), "{:?}", result);

    let rows = engine.select("user_table", vec!["id"], vec![]).await.unwrap();

    for row in &rows {
      let id_val = row.data.iter().find(|d| d.name == "id").unwrap();
      assert!(
        matches!(id_val.value, Value::VarChar(_)),
        "Expected VarChar after cast, got {:?}",
        id_val.value
      );
    }
  }

  #[tokio::test]
  async fn modify_column_type_varchar_to_varchar_wider() {
    let mut engine = setup_user_table().await;

    // Widen the `name` column from VarChar(100) to VarChar(255)
    let result = engine.alter_table(
      "user_table",
      AlterOp::ModifyColumnType {
        name: "name".to_string(),
        new_type: Type::VarChar(255),
      },
    ).await;

    assert!(result.is_ok(), "{:?}", result);

    // Verify the column is still selectable after the type change
    assert!(engine.select("user_table", vec!["name"], vec![]).await.is_ok());
  }

  #[tokio::test]
  async fn modify_column_type_nonexistent_column_errors() {
    let mut engine = setup_user_table().await;

    let result = engine.alter_table(
      "user_table",
      AlterOp::ModifyColumnType {
        name: "ghost".to_string(),
        new_type: Type::Int,
      },
    ).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("doesn't exist"));
  }

  #[tokio::test]
  async fn modify_column_type_nonexistent_table_errors() {
    let mut engine = Engine::new().await;

    let result = engine.alter_table(
      "no_such_table",
      AlterOp::ModifyColumnType {
        name: "id".to_string(),
        new_type: Type::VarChar(10),
      },
    ).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("doesn't exist"));
  }

  // -------------------------------------------------------------------------
  // INSERT
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn insert_ten_rows() {
    let mut engine = setup_user_table().await;
    let rows = engine.select("user_table", vec!["*"], vec![]).await.unwrap();
    assert_eq!(rows.len(), 10);
  }

  // -------------------------------------------------------------------------
  // SELECT
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn select_all_returns_all_rows() {
    let mut engine = setup_user_table().await;
    let rows = engine.select("user_table", vec!["*"], vec![]).await.unwrap();
    assert_eq!(rows.len(), 10);
  }

  #[tokio::test]
  async fn select_with_eq_condition() {
    let mut engine = setup_user_table().await;
    let rows = engine
      .select(
        "user_table",
        vec!["id", "name"],
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(5),
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 1);
    let id = rows[0].data.iter().find(|d| d.name == "id").unwrap();
    assert!(matches!(id.value, Value::Int(5)));
  }

  #[tokio::test]
  async fn select_with_and_condition() {
    let mut engine = setup_user_table().await;
    // id > 3 AND id < 7  →  ids 4, 5, 6
    let rows = engine
      .select(
        "user_table",
        vec!["id", "name"],
        vec![Condition::And(
          Box::new(Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(3),
            op: Operator::Gt,
          }),
          Box::new(Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(7),
            op: Operator::Lt,
          }),
        )],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 3);
    let ids: Vec<i32> = rows
      .iter()
      .map(|r| {
        if let Value::Int(v) = r.data.iter().find(|d| d.name == "id").unwrap().value {
          v
        } else {
          panic!("expected Int")
        }
      })
      .collect();
    assert_eq!(ids, vec![4, 5, 6]);
  }

  #[tokio::test]
  async fn select_with_or_condition() {
    let mut engine = setup_user_table().await;
    // id = 4 OR id = 8
    let rows = engine
      .select(
        "user_table",
        vec!["id", "name"],
        vec![Condition::Or(
          Box::new(Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(4),
            op: Operator::Eq,
          }),
          Box::new(Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(8),
            op: Operator::Eq,
          }),
        )],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 2);
    let ids: Vec<i32> = rows
      .iter()
      .map(|r| {
        if let Value::Int(v) = r.data.iter().find(|d| d.name == "id").unwrap().value {
          v
        } else {
          panic!("expected Int")
        }
      })
      .collect();
    assert!(ids.contains(&4));
    assert!(ids.contains(&8));
  }

  // -------------------------------------------------------------------------
  // UPDATE
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn update_single_row() {
    let mut engine = setup_user_table().await;

    engine
      .update(
        "user_table",
        vec![Data {
          name: "name".to_string(),
          value: Value::VarChar("UPDATED".to_string()),
        }],
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(5),
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    let rows = engine
      .select(
        "user_table",
        vec!["id", "name"],
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(5),
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 1);
    let name = rows[0].data.iter().find(|d| d.name == "name").unwrap();
    assert!(matches!(&name.value, Value::VarChar(s) if s == "UPDATED"));
  }

  // -------------------------------------------------------------------------
  // DELETE
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn delete_rows_with_lt_condition() {
    let mut engine = setup_user_table().await;

    // Delete id < 3  →  removes ids 0, 1, 2  →  7 rows remain
    engine
      .delete(
        "user_table",
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(3),
          op: Operator::Lt,
        }],
      )
      .await
      .unwrap();

    let rows = engine
      .select("user_table", vec!["id", "name"], vec![])
      .await
      .unwrap();

    assert_eq!(rows.len(), 7);

    let ids: Vec<i32> = rows
      .iter()
      .map(|r| {
        if let Value::Int(v) = r.data.iter().find(|d| d.name == "id").unwrap().value {
          v
        } else {
          panic!("expected Int")
        }
      })
      .collect();

    assert!(!ids.contains(&0));
    assert!(!ids.contains(&1));
    assert!(!ids.contains(&2));
  }

  // -------------------------------------------------------------------------
  // NULL
  // -------------------------------------------------------------------------

  #[tokio::test]
  async fn insert_null_value() {
    let mut engine = setup_user_table().await;

    engine
      .insert(&Entity {
        of: "user_table".to_string(),
        data: vec![
          Data {
            name: "id".to_string(),
            value: Value::Int(99),
          },
          Data {
            name: "name".to_string(),
            value: Value::Null,
          },
          Data {
            name: "password".to_string(),
            value: Value::Null,
          },
        ],
      })
      .await
      .unwrap();

    let rows = engine
      .select(
        "user_table",
        vec!["id", "name", "password"],
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(99),
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 1);
    let name = rows[0].data.iter().find(|d| d.name == "name").unwrap();
    assert!(matches!(name.value, Value::Null));
    let password = rows[0].data.iter().find(|d| d.name == "password").unwrap();
    assert!(matches!(password.value, Value::Null));
  }

  #[tokio::test]
  async fn null_does_not_match_conditions() {
    let mut engine = setup_user_table().await;

    engine
      .insert(&Entity {
        of: "user_table".to_string(),
        data: vec![
          Data {
            name: "id".to_string(),
            value: Value::Int(99),
          },
          Data {
            name: "name".to_string(),
            value: Value::Null,
          },
          Data {
            name: "password".to_string(),
            value: Value::Null,
          },
        ],
      })
      .await
      .unwrap();

    // NULL = NULL should not match
    let rows = engine
      .select(
        "user_table",
        vec!["id"],
        vec![Condition::Compare {
          attr: "name".to_string(),
          value: Value::Null,
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 0);
  }

  #[tokio::test]
  async fn update_to_null() {
    let mut engine = setup_user_table().await;

    engine
      .update(
        "user_table",
        vec![Data {
          name: "name".to_string(),
          value: Value::Null,
        }],
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(3),
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    let rows = engine
      .select(
        "user_table",
        vec!["id", "name"],
        vec![Condition::Compare {
          attr: "id".to_string(),
          value: Value::Int(3),
          op: Operator::Eq,
        }],
      )
      .await
      .unwrap();

    assert_eq!(rows.len(), 1);
    let name = rows[0].data.iter().find(|d| d.name == "name").unwrap();
    assert!(matches!(name.value, Value::Null));
  }

  #[tokio::test]
  async fn add_column_fills_null_for_existing_rows() {
    let mut engine = setup_user_table().await;

    engine
      .alter_table(
        "user_table",
        AlterOp::AddColumn(Attr {
          name: "email".to_string(),
          data_type: Type::VarChar(255),
        }),
      )
      .await
      .unwrap();

    let rows = engine.select("user_table", vec!["email"], vec![]).await.unwrap();

    assert_eq!(rows.len(), 10);
    for row in &rows {
      let email = row.data.iter().find(|d| d.name == "email").unwrap();
      assert!(matches!(email.value, Value::Null));
    }
  }
}