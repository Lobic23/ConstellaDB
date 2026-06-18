use network_module::client::ConnectionPool;
use network_module::message::Message;

#[tokio::main]
async fn main() {
  let addr = "127.0.0.1:7001";

  tokio::spawn(async move {
    network_module::listener::start(addr).await.unwrap();
  });

  tokio::time::sleep(std::time::Duration::from_millis(100)).await;

  let mut pool = ConnectionPool::new();

  // First call — dials a new connection
  let reply = pool.send(addr, &Message::Ping).await.unwrap();
  println!("[test] Ping  → {:?}", reply);

  // Second call — reuses the same connection
  let reply = pool
    .send(
      addr,
      &Message::Put {
        key: "foo".into(),
        value: "bar".into(),
      },
    )
    .await
    .unwrap();
  println!("[test] Put   → {:?}", reply);

  // Third call — still reusing
  let reply = pool
    .send(addr, &Message::Get { key: "foo".into() })
    .await
    .unwrap();
  println!("[test] Get   → {:?}", reply);

  // Simulate error recovery — drop and reconnect
  pool.remove(addr);
  let reply = pool.send(addr, &Message::Ping).await.unwrap();
  println!("[test] Ping after reconnect → {:?}", reply);
}
