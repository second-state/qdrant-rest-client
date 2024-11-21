use anyhow::{anyhow, bail, Error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};

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
        v.get("result")
            .unwrap()
            .get("points_count")
            .unwrap()
            .as_u64()
            .unwrap()
    }

    pub async fn create_collection(&self, collection_name: &str, size: u32) -> Result<(), Error> {
        if self.collection_exists(collection_name).await? {
            bail!("Collection '{}' already exists", collection_name);
        }

        let params = json!({
            "vectors": {
                "size": size,
                "distance": "Cosine",
                "on_disk": true,
            }
        });
        self.create_collection_api(collection_name, &params).await
    }

    pub async fn collection_exists(&self, collection_name: &str) -> Result<bool, Error> {
        self.collection_exists_api(collection_name).await
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<(), Error> {
        if !self.collection_exists(collection_name).await? {
            bail!("Collection '{}' does not exist", collection_name);
        }

        self.delete_collection_api(collection_name).await
    }

    pub async fn upsert_points(
        &self,
        collection_name: &str,
        points: Vec<Point>,
    ) -> Result<(), Error> {
        let params = json!({
            "points": points,
        });
        self.upsert_points_api(collection_name, &params).await
    }

    pub async fn search_points(
        &self,
        collection_name: &str,
        point: Vec<f32>,
        limit: u64,
        score_threshold: Option<f32>,
    ) -> Result<Vec<ScoredPoint>, Error> {
        let score_threshold = match score_threshold {
            Some(v) => v,
            None => 0.0,
        };

        let params = json!({
            "vector": point,
            "limit": limit,
            "with_payload": true,
            "with_vector": true,
            "score_threshold": score_threshold,
        });

        let v = match self.search_points_api(collection_name, &params).await {
            Ok(v) => v,
            Err(e) => {
                return Err(e);
            }
        };

        match v.get("result") {
            Some(v) => match v.as_array() {
                Some(rs) => {
                    let mut sps: Vec<ScoredPoint> = Vec::<ScoredPoint>::new();
                    for r in rs {
                        let sp: ScoredPoint = serde_json::from_value(r.clone()).unwrap();
                        sps.push(sp);
                    }
                    return Ok(sps);
                }
                None => {
                    bail!("[qdrant] The value corresponding to the 'result' key is not an array.");
                }
            },
            None => {
                bail!("[qdrant] The given key 'result' does not exist.");
            }
        }
    }

    pub async fn get_points(&self, collection_name: &str, ids: Vec<u64>) -> Vec<Point> {
        let params = json!({
            "ids": ids,
            "with_payload": true,
            "with_vector": true,
        });

        let v = self.get_points_api(collection_name, &params).await.unwrap();
        let rs: &Vec<Value> = v.get("result").unwrap().as_array().unwrap();
        let mut ps: Vec<Point> = Vec::<Point>::new();
        for r in rs {
            let p: Point = serde_json::from_value(r.clone()).unwrap();
            ps.push(p);
        }
        ps
    }

    pub async fn get_point(&self, collection_name: &str, id: u64) -> Point {
        let v = self.get_point_api(collection_name, id).await.unwrap();
        let r = v.get("result").unwrap();
        serde_json::from_value(r.clone()).unwrap()
    }

    pub async fn delete_points(&self, collection_name: &str, ids: Vec<u64>) -> Result<(), Error> {
        let params = json!({
            "points": ids,
        });
        self.delete_points_api(collection_name, &params).await
    }

    /// REST API functions
    pub async fn collection_info_api(&self, collection_name: &str) -> Result<Value, Error> {
        let url = format!("{}/collections/{}", self.url_base, collection_name,);

        let client = reqwest::Client::new();
        let ci = client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .json()
            .await?;
        Ok(ci)
    }

    pub async fn create_collection_api(
        &self,
        collection_name: &str,
        params: &Value,
    ) -> Result<(), Error> {
        let url = format!("{}/collections/{}", self.url_base, collection_name,);

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let res = client
            .put(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "[qdrant] Failed to create collection: {}",
                res.status().as_str()
            ))
        }
    }

    pub async fn collection_exists_api(&self, collection_name: &str) -> Result<bool, Error> {
        let url = format!("{}/collections/{}/exists", self.url_base, collection_name,);
        let client = reqwest::Client::new();
        let res = client.get(&url).send().await?;
        Ok(res.status().is_success())
    }

    pub async fn delete_collection_api(&self, collection_name: &str) -> Result<(), Error> {
        let url = format!("{}/collections/{}", self.url_base, collection_name,);

        let client = reqwest::Client::new();
        let res = client
            .delete(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "[qdrant] Failed to delete collection: {}",
                res.status().as_str()
            ))
        }
    }

    pub async fn upsert_points_api(
        &self,
        collection_name: &str,
        params: &Value,
    ) -> Result<(), Error> {
        let url = format!(
            "{}/collections/{}/points?wait=true",
            self.url_base, collection_name,
        );

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let res = client
            .put(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        if res.status().is_success() {
            let v = res.json::<Value>().await?;
            let status = v.get("status").unwrap().as_str().unwrap();
            if status == "ok" {
                Ok(())
            } else {
                Err(anyhow!(
                    "[qdrant] Failed to upsert points. Status = {}",
                    status
                ))
            }
        } else {
            Err(anyhow!(
                "[qdrant] Failed to upsert points: {}",
                res.status().as_str()
            ))
        }
    }

    pub async fn search_points_api(
        &self,
        collection_name: &str,
        params: &Value,
    ) -> Result<Value, Error> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.url_base, collection_name,
        );

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status_code = response.status();
        match status_code.is_success() {
            true => {
                let json = response.json().await?;
                Ok(json)
            }
            false => {
                let status = status_code.as_str();
                Err(anyhow!("[qdrant] Failed to search points: {}", status))
            }
        }
    }

    pub async fn get_points_api(
        &self,
        collection_name: &str,
        params: &Value,
    ) -> Result<Value, Error> {
        let url = format!("{}/collections/{}/points", self.url_base, collection_name,);

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let json = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?
            .json()
            .await?;
        Ok(json)
    }

    pub async fn get_point_api(&self, collection_name: &str, id: u64) -> Result<Value, Error> {
        let url = format!(
            "{}/collections/{}/points/{}",
            self.url_base, collection_name, id,
        );

        let client = reqwest::Client::new();
        let json = client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .json()
            .await?;
        Ok(json)
    }

    pub async fn delete_points_api(
        &self,
        collection_name: &str,
        params: &Value,
    ) -> Result<(), Error> {
        let url = format!(
            "{}/collections/{}/points/delete?wait=true",
            self.url_base, collection_name,
        );

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let res = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "[qdrant] Failed to delete points: {}",
                res.status().as_str()
            ))
        }
    }
}
