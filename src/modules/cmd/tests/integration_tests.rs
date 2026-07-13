// cargo test -- --test-threads=1

#[cfg(test)]
mod tests {
  use cmd_module::{execute, parse_cmd};
  use db_module::Engine;

  async fn setup_test_engine() -> Engine {
    let mut engine = Engine::new().await;
    // Clean up any existing test tables
    let _ = engine.drop_table("people").await;
    engine
  }

  #[tokio::test]
  async fn test_create_table() {
    let mut engine = setup_test_engine().await;
    let sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(result.contains("OK") && result.contains("created"));
  }

  #[tokio::test]
  async fn test_insert_and_select() {
    let mut engine = setup_test_engine().await;

    // Create table
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    // Insert data
    let insert_sql = "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)";
    let cmd = parse_cmd(insert_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(result.contains("OK"));

    // Select data
    let select_sql = "SELECT * FROM people WHERE id = 1";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(result.contains("Alice") && result.contains("30"));
  }

  #[tokio::test]
  async fn test_update() {
    let mut engine = setup_test_engine().await;

    // Setup
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    let insert_sql = "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)";
    let cmd = parse_cmd(insert_sql).unwrap();
    execute(&mut engine, cmd).await;

    // Update
    let update_sql = "UPDATE people SET age = 31 WHERE name = 'Alice'";
    let cmd = parse_cmd(update_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(result.contains("OK") && result.contains("updated"));

    // Verify
    let select_sql = "SELECT age FROM people WHERE name = 'Alice'";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(result.contains("31"));
  }

  #[tokio::test]
  async fn test_delete() {
    let mut engine = setup_test_engine().await;

    // Setup
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    let insert_sql = "INSERT INTO people (id, name, age) VALUES (1, 'Bob', 17)";
    let cmd = parse_cmd(insert_sql).unwrap();
    execute(&mut engine, cmd).await;

    // Delete
    let delete_sql = "DELETE FROM people WHERE age < 18";
    let cmd = parse_cmd(delete_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(result.contains("OK") && result.contains("deleted"));

    // Verify
    let select_sql = "SELECT * FROM people";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();
    assert!(!result.contains("Bob"));
  }

  #[tokio::test]
  async fn test_complex_queries() {
    let mut engine = setup_test_engine().await;

    // Setup table
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    // Insert multiple rows
    let inserts = vec![
      "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)",
      "INSERT INTO people (id, name, age) VALUES (2, 'Bob', 17)",
      "INSERT INTO people (id, name, age) VALUES (3, 'Charlie', 25)",
    ];

    for sql in inserts {
      let cmd = parse_cmd(sql).unwrap();
      execute(&mut engine, cmd).await;
    }

    // Test OR condition - should return Bob (name matches) and Charlie (age >= 25)
    let select_sql = "SELECT * FROM people WHERE name = 'Bob' OR age >= 25";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();

    // Debug output to see what's actually returned
    println!("Query result: {}", result);

    // Bob should be included (name matches)
    assert!(
      result.contains("Bob"),
      "Bob should be in results (name = 'Bob')"
    );

    // Charlie should be included (age >= 25)
    assert!(
      result.contains("Charlie"),
      "Charlie should be in results (age >= 25)"
    );

    assert!(
      result.contains("Alice"),
      "Alice should be in results (age 30 >= 25)"
    );

  }


  #[tokio::test]
  async fn test_or_condition() {
    let mut engine = setup_test_engine().await;

    // Setup
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    let inserts = vec![
      "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)",
      "INSERT INTO people (id, name, age) VALUES (2, 'Bob', 17)",
      "INSERT INTO people (id, name, age) VALUES (3, 'Charlie', 25)",
    ];

    for sql in inserts {
      let cmd = parse_cmd(sql).unwrap();
      execute(&mut engine, cmd).await;
    }
    let select_sql = "SELECT * FROM people WHERE name = 'Bob' OR age >= 30";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();

    println!("Query result: {}", result);

    assert!(
      result.contains("Bob"),
      "Bob should be in results (name = 'Bob')"
    );
    assert!(
      result.contains("Alice"),
      "Alice should be in results (age 30 >= 30)"
    );
    assert!(
      !result.contains("Charlie"),
      "Charlie should NOT be in results (age 25 < 30, name != 'Bob')"
    );
  }

  #[tokio::test]
  async fn test_and_condition() {
    let mut engine = setup_test_engine().await;

    // Setup
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    let inserts = vec![
      "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)",
      "INSERT INTO people (id, name, age) VALUES (2, 'Bob', 17)",
      "INSERT INTO people (id, name, age) VALUES (3, 'Charlie', 25)",
    ];

    for sql in inserts {
      let cmd = parse_cmd(sql).unwrap();
      execute(&mut engine, cmd).await;
    }
    let select_sql = "SELECT * FROM people WHERE age > 18 AND age < 30";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();

    println!("Query result: {}", result);

    assert!(
      result.contains("Charlie"),
      "Charlie should be in results (age 25 between 18 and 30)"
    );
    assert!(
      !result.contains("Alice"),
      "Alice should NOT be in results (age 30 is not < 30)"
    );
    assert!(
      !result.contains("Bob"),
      "Bob should NOT be in results (age 17 is not > 18)"
    );
  }

  #[tokio::test]
  async fn test_complex_queries_fixed() {
    let mut engine = setup_test_engine().await;

    // Setup table
    let create_sql = "CREATE TABLE people (id INT, name VARCHAR(50), age INT)";
    let cmd = parse_cmd(create_sql).unwrap();
    execute(&mut engine, cmd).await;

    // Insert multiple rows
    let inserts = vec![
      "INSERT INTO people (id, name, age) VALUES (1, 'Alice', 30)",
      "INSERT INTO people (id, name, age) VALUES (2, 'Bob', 17)",
      "INSERT INTO people (id, name, age) VALUES (3, 'Charlie', 25)",
    ];

    for sql in inserts {
      let cmd = parse_cmd(sql).unwrap();
      execute(&mut engine, cmd).await;
    }
    let select_sql = "SELECT * FROM people WHERE name = 'Bob' OR age >= 25";
    let cmd = parse_cmd(select_sql).unwrap();
    let result = execute(&mut engine, cmd).await.to_string();

    println!("Query result: {}", result);
    assert!(result.contains("Bob"), "Bob should be in results");
    assert!(result.contains("Charlie"), "Charlie should be in results");
    assert!(result.contains("Alice"), "Alice should be in results");
  }
}
