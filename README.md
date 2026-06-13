
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
Terminal 1 — start the server : cargo run --bin db_server -p server_module
Terminal 2 — connect a client (same machine): cargo run --bin db_client -p server_module
From a remote device: cargo run --bin db_client -p server_module -- 192.168.x.x:7878 my-node