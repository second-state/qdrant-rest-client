use clap::Parser;
use qdrant::*;
use serde_json::json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "http://localhost:6333")]
    qdrant_service_endpoint: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    // read api-key from the environment variable
    let api_key = std::env::var("QDRANT_API_KEY").ok();

    let mut client = qdrant::Qdrant::new_with_url(cli.qdrant_service_endpoint);
    if let Some(api_key) = api_key {
        client.set_api_key(api_key);
    }

    let collection_name = "my_test";

    // check if the collection exists. If present, delete it.
    if let Ok(true) = client.collection_exists(collection_name).await {
        println!("Collection `{}` exists", collection_name);
        match client.delete_collection(collection_name).await {
            Ok(_) => println!("Collection `{}` deleted", collection_name),
            Err(e) => println!("Error deleting collection: {:?}", e),
        }
    };

    // Create a collection with 10-dimensional vectors
    let r = client.create_collection("my_test", 4).await;
    println!("Create collection result is {:?}", r);

    let mut points = Vec::<Point>::new();
    points.push(Point {
        id: PointId::Num(1),
        vector: vec![0.05, 0.61, 0.76, 0.74],
        payload: json!({"city": "Berlin"}).as_object().map(|m| m.to_owned()),
    });
    points.push(Point {
        id: PointId::Num(2),
        vector: vec![0.19, 0.81, 0.75, 0.11],
        payload: json!({"city": "London"}).as_object().map(|m| m.to_owned()),
    });
    points.push(Point {
        id: PointId::Num(3),
        vector: vec![0.36, 0.55, 0.47, 0.94],
        payload: json!({"city": "Moscow"}).as_object().map(|m| m.to_owned()),
    });
    points.push(Point {
        id: PointId::Num(4),
        vector: vec![0.18, 0.01, 0.85, 0.80],
        payload: json!({"city": "New York"})
            .as_object()
            .map(|m| m.to_owned()),
    });
    points.push(Point {
        id: PointId::Num(5),
        vector: vec![0.24, 0.18, 0.22, 0.44],
        payload: json!({"city": "Beijing"}).as_object().map(|m| m.to_owned()),
    });
    points.push(Point {
        id: PointId::Num(6),
        vector: vec![0.35, 0.08, 0.11, 0.44],
        payload: json!({"city": "Mumbai"}).as_object().map(|m| m.to_owned()),
    });

    let r = client.upsert_points("my_test", points).await;
    println!("Upsert points result is {:?}", r);

    println!(
        "The collection size is {}",
        client.collection_info("my_test").await
    );

    let p = client.get_point("my_test", 2).await;
    println!("The second point is {:?}", p);

    let ps = client.get_points("my_test", vec![1, 2, 3, 4, 5, 6]).await;
    println!("The 1-6 points are {:?}", ps);

    let q = vec![0.2, 0.1, 0.9, 0.7];
    let r = client.search_points("my_test", q, 2, None).await;
    println!("Search result points are {:?}", r);

    let r = client.delete_points("my_test", vec![1, 4]).await;
    println!("Delete points result is {:?}", r);

    println!(
        "The collection size is {}",
        client.collection_info("my_test").await
    );

    let q = vec![0.2, 0.1, 0.9, 0.7];
    let r = client.search_points("my_test", q, 2, None).await;
    println!("Search result points are {:?}", r);
    Ok(())
}
