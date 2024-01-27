use serde_json::{json};
use qdrant::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = qdrant::Qdrant::new();
    // Create a collection with 10-dimensional vectors
    let r = client.create_collection("my_test", 10).await;
    println!("Create collection result is {:?}", r);

    let mut points = Vec::<Point>::new();
    for i in 0..10 {
        let p = Point {
            id: PointId::Num(i),
            vector: vec![i as f32; 10],
            payload: json!({"text": i.to_string()}).as_object().map(|m| m.to_owned()),
        };
        points.push(p);
    }
    println!("Upsert points {:?}", points);
    let r = client.upsert_points("my_test", points).await;
    println!("Upsert points result is {:?}", r);

    let q = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let r = client.search_points("my_test", q, 2).await;
    println!("Search result points are {:?}", r);
    Ok(())
}
