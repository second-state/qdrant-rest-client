use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointId {
    Uuid(String),
    Num(u64),
}

/// The point struct.
/// A point is a record consisting of a vector and an optional payload.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// Id of the point
    pub id: PointId,

    /// Vectors
    pub vector: Vec<f32>,

    /// Additional information along with vectors
    pub payload: Option<Map<String, Value>>,
}

/// The point struct with the score returned by searching
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredPoint {
    /// Id of the point
    pub id: PointId,

    /// Vectors
    pub vector: Option<Vec<f32>>,

    /// Additional information along with vectors
    pub payload: Option<Map<String, Value>>,

    /// Points vector distance to the query vector
    pub score: f32,
}

pub struct Qdrant {
    pub url_base: String,
}

impl Qdrant {
    pub fn new_with_url(url_base_: String) -> Qdrant {
        Qdrant {
            url_base: url_base_,
        }
    }

    pub fn new() -> Qdrant {
        Qdrant::new_with_url("http://localhost:6333".to_string())
    }
}

impl Qdrant {
    /// Shortcut functions
    pub async fn collection_info(&self, collection_name: &str) -> u64 {
        let v = self.collection_info_api(collection_name).await.unwrap();
        v.get("result").unwrap().get("points_count").unwrap().as_u64().unwrap()
    }

    pub async fn create_collection(&self, collection_name: &str, size: u32) -> Result<(), Error> {
        let params = json!({
            "vectors": {
                "size": size,
                "distance": "Cosine",
                "on_disk": true,
            }
        });
        self.create_collection_api(collection_name, &params).await
    }

    pub async fn upsert_points(&self, collection_name: &str, points: Vec<Point>) -> Result<(), Error> {
        let params = json!({
            "wait": true,
            "points": points,
        });
        self.upsert_points_api(collection_name, &params).await
    }

    pub async fn search_points(&self, collection_name: &str, point: Vec<f32>, limit: u64) -> Vec<ScoredPoint> {
        let params = json!({
            "vector": point,
            "limit": limit,
        });

        let v = self.search_points_api(collection_name, &params).await.unwrap();
        let rs : &Vec<Value> = v.get("result").unwrap().as_array().unwrap();
        let mut sps : Vec<ScoredPoint> = Vec::<ScoredPoint>::new();
        for r in rs {
            let sp : ScoredPoint = serde_json::from_value(r.clone()).unwrap();
            sps.push(sp);
        }
        sps
    }

    /// REST API functions
    pub async fn collection_info_api(&self, collection_name: &str) -> Result<Value, Error> {
        let url = format!(
            "{}/collections/{}",
            self.url_base,
            collection_name,
        );

        let client = reqwest::Client::new();
        let ci = client.get(&url).header("Content-Type", "application/json").send().await?.json().await?;
        Ok(ci)
    }


    pub async fn create_collection_api(&self, collection_name: &str, params: &Value) -> Result<(), Error> {
        let url = format!(
            "{}/collections/{}",
            self.url_base,
            collection_name,
        );

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let res = client.put(&url).header("Content-Type", "application/json").body(body).send().await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to create collection: {}", res.status().as_str()))
        }
    }

    pub async fn delete_collection_api(&self, collection_name: &str) -> Result<(), Error> {
        let url = format!(
            "{}/collections/{}",
            self.url_base,
            collection_name,
        );

        let client = reqwest::Client::new();
        let res = client.delete(&url).header("Content-Type", "application/json").send().await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete collection: {}", res.status().as_str()))
        }
    }

    pub async fn upsert_points_api(&self, collection_name: &str, params: &Value) -> Result<(), Error> {
        let url = format!(
            "{}/collections/{}/points",
            self.url_base,
            collection_name,
        );

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let res = client.put(&url).header("Content-Type", "application/json").body(body).send().await?;
        if res.status().is_success() {
            if res.json::<Value>().await?.get("status").unwrap().as_str().unwrap() == "completed" {
                Ok(())
            } else {
                Err(anyhow!("Failed to upsert points"))
            }
        } else {
            Err(anyhow!("Failed to upsert points: {}", res.status().as_str()))
        }
    }


    pub async fn search_points_api(&self, collection_name: &str, params: &Value) -> Result<Value, Error> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.url_base,
            collection_name,
        );

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let sp = client.post(&url).header("Content-Type", "application/json").body(body).send().await?.json().await?;
        Ok(sp)
    }

}
