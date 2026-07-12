
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

## Run Project
### Step 1: Run Gateway
```bash
cargo run -p gateway -- -c {client_listener_port} -n {node_listener_port}
```

### Step 2: Run db service
```bash
cargo run -p db_service -- -p {db_service_port}
```

### Step 3: Run job service
```bash
cargo run -p job_service -- -p {job_service_port} -d {db_service_port}
```

### Step 4: Run node
```bash
cargo run -p node -- -p {node_port} -j {job_service_port} -g {node_listener_port}
```

### Step 5: Run Client
```bash
cargo run --bin db_client -- {gateway_ip:client_listener_port}
```

### Repeat step 2-4 for each node with seperate ports. To generate a full suite of environment run test_generator.py

## test the client server stuff
```bash
cargo run --bin db_client
cargo run --bin db_server
cargo run --bin db_cli
```
