## Run Project

### Step 0 : popluate .env 
```bash 
cp .env.example .env
```

### Step 1: Run Gateway
```bash
cargo gateway
```

### Step 2: Run Nodes
Run this command with different node port for multiple nodes
```bash
cargo node < -p {port} >
```
### Step 3: Run Client
```bash
cargo client
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
