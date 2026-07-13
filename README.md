## Run Project
### Step 1: Run Gateway
```bash
cargo run --bin constella_db run-gateway -c {client_listener_port} -n {node_listener_port}
```

### Step 2: Run Nodes
Run this command with different node port for multiple nodes
```bash
cargo run --bin constella_db run-node -p {node_port} -g {gateway_ip:node_listener_port}
```
### Step 3: Run Client
```bash
cargo run --bin constella_db run-client -g {gateway_ip:client_listener_port}
```

## Create module
```bash
cargo new src/modules/{module_name} --lib
```

## Add to workspace in Cargo.toml
```toml
[workspace]
members = [
  ...
  "src/modules/{module_name}",
]
```

## Make testbed
```bash
cd src/modules/{module_name}/src
mkdir bin
touch bin/test.rs
```

## Run test
```bash
cargo run -p {module_name} --bin test
```

## test the client server stuff
```bash
cargo run --bin db_client
cargo run --bin db_server
cargo run --bin db_cli
```
