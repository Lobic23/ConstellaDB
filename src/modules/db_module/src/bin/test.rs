use db_module::*;

fn main() {
  let mut engine = Engine::new();

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

  let _ = engine.drop_table("user_table");

  engine.create_table(&user_table).unwrap();

  println!("=== INSERT ===");

  for i in 0..10 {
    let user = Entity {
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
    };

    engine.insert(&user).unwrap();
  }

  println!("=== SELECT ALL ===");

  let result = engine.select(
    "user_table",
    vec!["*"],
    vec![],
  ).unwrap();

  println!("{:#?}", result);

  println!("=== SELECT WHERE id = 5 ===");

  let result = engine.select(
    "user_table",
    vec!["id", "name"],
    vec![
      Condition::Compare {
        attr: "id".to_string(),
        value: Value::Int(5),
        op: Operator::Eq,
      },
    ],
  ).unwrap();

  println!("{:#?}", result);

  println!("=== SELECT WHERE id > 3 AND id < 7 ===");

  let result = engine.select(
    "user_table",
    vec!["id", "name"],
    vec![
      Condition::And(
        Box::new(
          Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(3),
            op: Operator::Gt,
          },
        ),
        Box::new(
          Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(7),
            op: Operator::Lt,
          },
        ),
      ),
    ],
  ).unwrap();

  println!("{:#?}", result);

  println!("=== UPDATE id = 5 ===");

  engine.update(
    "user_table",
    vec![
      Data {
        name: "name".to_string(),
        value: Value::VarChar("UPDATED".to_string()),
      },
    ],
    vec![
      Condition::Compare {
        attr: "id".to_string(),
        value: Value::Int(5),
        op: Operator::Eq,
      },
    ],
  ).unwrap();

  let result = engine.select(
    "user_table",
    vec!["id", "name"],
    vec![
      Condition::Compare {
        attr: "id".to_string(),
        value: Value::Int(5),
        op: Operator::Eq,
      },
    ],
  ).unwrap();

  println!("{:#?}", result);

  println!("=== DELETE id < 3 ===");

  engine.delete(
    "user_table",
    vec![
      Condition::Compare {
        attr: "id".to_string(),
        value: Value::Int(3),
        op: Operator::Lt,
      },
    ],
  ).unwrap();

  let result = engine.select(
    "user_table",
    vec!["id", "name"],
    vec![],
  ).unwrap();

  println!("{:#?}", result);

  println!("=== SELECT WHERE id = 4 OR id = 8 ===");

  let result = engine.select(
    "user_table",
    vec!["id", "name"],
    vec![
      Condition::Or(
        Box::new(
          Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(4),
            op: Operator::Eq,
          },
        ),
        Box::new(
          Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(8),
            op: Operator::Eq,
          },
        ),
      ),
    ],
  ).unwrap();

  println!("{:#?}", result);

  // println!("=== DROP TABLE ===");

  // engine.drop_table("user_table").unwrap();
}
