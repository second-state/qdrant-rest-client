# A lightweight Qdrant client library for Rust

The key requirements for this unofficial client are two folds:

* Compiles into Wasm and runs under the WasmEdge Runtime.
* Supports basic CRUD operations for vector collections and points.
* Supports TLS for remotely installed Qdrant databases.

## Quick start

Install WasmEdge and Rust tools.

```
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash
source $HOME/.wasmedge/env

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-wasi
```

Start a Qdrant instance in Docker using the [quick start guide](https://qdrant.tech/documentation/quick-start/).

```
mkdir qdrant_storage

docker run -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_storage:/qdrant/storage:z \
    qdrant/qdrant
```

Build and run the `examples` in this repo.

```
cd examples
RUSTFLAGS="--cfg wasmedge --cfg tokio_unstable" cargo build --target wasm32-wasi --release
wasmedge target/wasm32-wasi/release/qdrant_examples.wasm
```

## Examples

Here is the code from the [examples/src/main.rs](examples/src/main.rs) to show how to do CRUD operations.

```rust
    // Create
    let r = client.create_collection("my_test", 4).await;

    // Insert / Update
    let mut points = Vec::<Point>::new();
    points.push(Point{
        id: PointId::Num(1), vector: vec!(0.05, 0.61, 0.76, 0.74), payload: json!({"city": "Berlin"}).as_object().map(|m| m.to_owned())
    });
    points.push(Point{
        id: PointId::Num(2), vector: vec!(0.19, 0.81, 0.75, 0.11), payload: json!({"city": "London"}).as_object().map(|m| m.to_owned())
    });
    points.push(Point{
        id: PointId::Num(3), vector: vec!(0.36, 0.55, 0.47, 0.94), payload: json!({"city": "Moscow"}).as_object().map(|m| m.to_owned())
    });
    points.push(Point{
        id: PointId::Num(4), vector: vec!(0.18, 0.01, 0.85, 0.80), payload: json!({"city": "New York"}).as_object().map(|m| m.to_owned())
    });
    points.push(Point{
        id: PointId::Num(5), vector: vec!(0.24, 0.18, 0.22, 0.44), payload: json!({"city": "Beijing"}).as_object().map(|m| m.to_owned())
    });
    points.push(Point{
        id: PointId::Num(6), vector: vec!(0.35, 0.08, 0.11, 0.44), payload: json!({"city": "Mumbai"}).as_object().map(|m| m.to_owned())
    });
    let r = client.upsert_points("my_test", points).await;
    println!("The collection size is {}", client.collection_info("my_test").await);

    // Retrieve #1
    let ps = client.get_points("my_test", vec!(1, 2, 3, 4, 5, 6)).await;
    println!("The 1-6 points are {:?}", ps);

    // Retrieve Search
    let q = vec![0.2, 0.1, 0.9, 0.7];
    let r = client.search_points("my_test", q, 2).await;
    println!("Search result points are {:?}", r);

    // Delete
    let r = client.delete_points("my_test", vec!(1, 4)).await;
    println!("Delete points result is {:?}", r);
    println!("The collection size is {}", client.collection_info("my_test").await);
```

## Writing code

Add the following patches to `cargo.toml` and then you can use the `qdrant_rest_client` and `tokio` crates as regular dependencies.

```
[patch.crates-io]
socket2 = { git = "https://github.com/second-state/socket2.git", branch = "v0.5.x" }
reqwest = { git = "https://github.com/second-state/wasi_reqwest.git", branch = "0.11.x" }
hyper = { git = "https://github.com/second-state/wasi_hyper.git", branch = "v0.14.x" }
tokio = { git = "https://github.com/second-state/wasi_tokio.git", branch = "v1.36.x" }

[dependencies]
anyhow = "1.0"
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
url = "2.3"
tokio = { version = "1", features = [
    "io-util",
    "fs",
    "net",
    "time",
    "rt",
    "macros",
] }
qdrant_rest_client = "0.1"
```

To build it, you need to pass the Rust compiler flags as the above quick start. 

```
RUSTFLAGS="--cfg wasmedge --cfg tokio_unstable" cargo build --target wasm32-wasi --release
```

Or, you can add to the `.cargo/config.toml` file.

```
[build]
target = "wasm32-wasi"
rustflags = ["--cfg", "wasmedge", "--cfg", "tokio_unstable"]

[target.wasm32-wasi]
runner = "wasmedge"
```

After that you can just `cargo build` and `cargo run` to build and run the application.
