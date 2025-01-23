#[cfg(feature = "logging")]
#[macro_use]
extern crate log;

use anyhow::{anyhow, bail, Error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointId {
    Uuid(String),
    Num(u64),
}
impl From<u64> for PointId {
    fn from(num: u64) -> Self {
        PointId::Num(num)
    }
}
impl From<String> for PointId {
    fn from(uuid: String) -> Self {
        PointId::Uuid(uuid)
    }
}
impl Display for PointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointId::Uuid(uuid) => write!(f, "{}", uuid),
            PointId::Num(num) => write!(f, "{}", num),
        }
    }
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
    api_key: Option<String>,
}

impl Qdrant {
    pub fn new_with_url(url_base_: String) -> Qdrant {
        Qdrant {
            url_base: url_base_,
            api_key: None,
        }
    }

    pub fn new() -> Qdrant {
        Qdrant::new_with_url("http://localhost:6333".to_string())
    }

    pub fn set_api_key(&mut self, api_key: impl Into<String>) {
        self.api_key = Some(api_key.into());
    }
}

impl Qdrant {
    /// Shortcut functions
    pub async fn collection_info(&self, collection_name: &str) -> u64 {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "get collection info: '{}'", collection_name);

        let v = self.collection_info_api(collection_name).await.unwrap();
        v.get("result")
            .unwrap()
            .get("points_count")
            .unwrap()
            .as_u64()
            .unwrap()
    }

    pub async fn create_collection(&self, collection_name: &str, size: u32) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "create collection '{}'", collection_name);

        match self.collection_exists(collection_name).await {
            Ok(false) => (),
            Ok(true) => {
                let err_msg = format!("Collection '{}' already exists", collection_name);

                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", &err_msg);

                bail!(err_msg);
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", e);

                bail!("{}", e);
            }
        }

        let params = json!({
            "vectors": {
                "size": size,
                "distance": "Cosine",
                "on_disk": true,
            }
        });
        if !self.create_collection_api(collection_name, &params).await? {
            bail!("Failed to create collection '{}'", collection_name);
        }
        Ok(())
    }

    pub async fn list_collections(&self) -> Result<Vec<String>, Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "list collections");

        self.list_collections_api().await
    }

    pub async fn collection_exists(&self, collection_name: &str) -> Result<bool, Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "check collection existence: {}", collection_name);

        let collection_names = self.list_collections().await?;

        Ok(collection_names.contains(&collection_name.to_string()))
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "delete collection '{}'", collection_name);

        match self.collection_exists(collection_name).await {
            Ok(true) => (),
            Ok(false) => {
                let err_msg = format!("Not found collection '{}'", collection_name);

                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", &err_msg);

                bail!(err_msg);
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", e);

                bail!("{}", e);
            }
        }

        if !self.delete_collection_api(collection_name).await? {
            bail!("Failed to delete collection '{}'", collection_name);
        }
        Ok(())
    }

    pub async fn upsert_points(
        &self,
        collection_name: &str,
        points: Vec<Point>,
    ) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "upsert {} points to collection '{}'", points.len(), collection_name);

        let params = json!({
            "points": points,
        });
        self.upsert_points_api(collection_name, &params).await
    }

    pub async fn search_points(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        limit: u64,
        score_threshold: Option<f32>,
    ) -> Result<Vec<ScoredPoint>, Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "search points in collection '{}'", collection_name);

        let score_threshold = match score_threshold {
            Some(v) => v,
            None => 0.0,
        };

        let params = json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true,
            "with_vector": true,
            "score_threshold": score_threshold,
        });

        match self.search_points_api(collection_name, &params).await {
            Ok(v) => {
                match v.get("result") {
                    Some(v) => match v.as_array() {
                        Some(rs) => {
                            let mut sps: Vec<ScoredPoint> = Vec::<ScoredPoint>::new();
                            for r in rs {
                                let sp: ScoredPoint = serde_json::from_value(r.clone()).unwrap();
                                sps.push(sp);
                            }
                            Ok(sps)
                        }
                        None => {
                            bail!("[qdrant] The value corresponding to the 'result' key is not an array.")
                        }
                    },
                    None => {
                        let warn_msg = "[qdrant] The given key 'result' does not exist.";

                        #[cfg(feature = "logging")]
                        warn!(target: "stdout", "{}", warn_msg);

                        Ok(vec![])
                    }
                }
            }
            Err(e) => {
                let warn_msg = format!("[qdrant] Failed to search points: {}", e);

                #[cfg(feature = "logging")]
                warn!(target: "stdout", "{}", warn_msg);

                Ok(vec![])
            }
        }
    }

    pub async fn get_points(&self, collection_name: &str, ids: &[PointId]) -> Vec<Point> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "get points from collection '{}'", collection_name);

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

    pub async fn get_point(&self, collection_name: &str, id: &PointId) -> Point {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "get point from collection '{}' with id {}", collection_name, id);

        let v = self.get_point_api(collection_name, id).await.unwrap();
        let r = v.get("result").unwrap();
        serde_json::from_value(r.clone()).unwrap()
    }

    pub async fn delete_points(&self, collection_name: &str, ids: &[PointId]) -> Result<(), Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "delete points from collection '{}'", collection_name);

        let params = json!({
            "points": ids,
        });
        self.delete_points_api(collection_name, &params).await
    }

    /// REST API functions
    pub async fn collection_info_api(&self, collection_name: &str) -> Result<Value, Error> {
        let url = format!("{}/collections/{}", self.url_base, collection_name,);

        let client = reqwest::Client::new();

        let ci = match &self.api_key {
            Some(api_key) => {
                client
                    .get(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?
                    .json()
                    .await?
            }
            None => {
                client
                    .get(&url)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?
                    .json()
                    .await?
            }
        };

        Ok(ci)
    }

    pub async fn create_collection_api(
        &self,
        collection_name: &str,
        params: &Value,
    ) -> Result<bool, Error> {
        let url = format!("{}/collections/{}", self.url_base, collection_name,);

        let body = serde_json::to_vec(params).unwrap_or_default();
        let client = reqwest::Client::new();
        let res = match &self.api_key {
            Some(api_key) => {
                client
                    .put(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
            None => {
                client
                    .put(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
        };

        match res.status().is_success() {
            true => {
                // get response body as json
                let json = res.json::<Value>().await?;
                let sucess = json.get("result").unwrap().as_bool().unwrap();
                Ok(sucess)
            }
            false => Err(anyhow!(
                "[qdrant] Failed to create collection: {}",
                collection_name
            )),
        }
    }

    pub async fn list_collections_api(&self) -> Result<Vec<String>, Error> {
        let url = format!("{}/collections", self.url_base);
        let client = reqwest::Client::new();
        let result = match &self.api_key {
            Some(api_key) => {
                client
                    .get(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .send()
                    .await
            }
            None => {
                client
                    .get(&url)
                    .header("Content-Type", "application/json")
                    .send()
                    .await
            }
        };

        let response = match result {
            Ok(response) => response,
            Err(e) => {
                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", e);

                bail!("{}", e);
            }
        };

        match response.status().is_success() {
            true => match response.json::<Value>().await {
                Ok(json) => match json.get("result") {
                    Some(result) => match result.get("collections") {
                        Some(collections) => match collections.as_array() {
                            Some(collections) => {
                                let mut collection_names: Vec<String> = Vec::<String>::new();

                                for collection in collections {
                                    let name = collection.get("name").unwrap().as_str().unwrap();
                                    collection_names.push(name.to_string());
                                }

                                Ok(collection_names)
                            },
                            None => bail!("[qdrant] The value corresponding to the 'collections' key is not an array."),
                        },
                        None => bail!("[qdrant] The given key 'collections' does not exist."),
                    },
                    None => bail!("[qdrant] The given key 'result' does not exist."),
                },
                Err(e) => {
                    #[cfg(feature = "logging")]
                    error!(target: "stdout", "{}", e);

                    bail!("{}", e);
                }
            }
            false => bail!("[qdrant] Failed to list collections"),
        }
    }

    pub async fn collection_exists_api(&self, collection_name: &str) -> Result<bool, Error> {
        #[cfg(feature = "logging")]
        info!(target: "stdout", "check collection existence: {}", collection_name);

        let url = format!("{}/collections/{}/exists", self.url_base, collection_name,);
        let client = reqwest::Client::new();

        #[cfg(feature = "logging")]
        info!(target: "stdout", "check collection existence: {}", url);

        let result = match &self.api_key {
            Some(api_key) => {
                client
                    .get(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .send()
                    .await
            }
            None => {
                client
                    .get(&url)
                    .header("Content-Type", "application/json")
                    .send()
                    .await
            }
        };

        #[cfg(feature = "logging")]
        info!(target: "stdout", "result: {:?}", result);

        let response = match result {
            Ok(response) => response,
            Err(e) => {
                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", e);

                bail!("{}", e);
            }
        };

        let json = match response.json::<Value>().await {
            Ok(json) => json,
            Err(e) => {
                #[cfg(feature = "logging")]
                error!(target: "stdout", "{}", e);

                bail!("{}", e);
            }
        };

        #[cfg(feature = "logging")]
        info!(target: "stdout", "json: {:?}", json);

        match json.get("result") {
            Some(result) => {
                let exists = result.get("exists").unwrap().as_bool().unwrap();
                Ok(exists)
            }
            None => Err(anyhow!("[qdrant] Failed to check collection existence")),
        }

        // match res.status().is_success() {
        //     true => {
        //         // get response body as json
        //         let json = res.json::<Value>().await?;
        //         let exists = json
        //             .get("result")
        //             .unwrap()
        //             .get("exists")
        //             .unwrap()
        //             .as_bool()
        //             .unwrap();
        //         Ok(exists)
        //     }
        //     false => Err(anyhow!("[qdrant] Failed to check collection existence")),
        // }
    }

    pub async fn delete_collection_api(&self, collection_name: &str) -> Result<bool, Error> {
        let url = format!("{}/collections/{}", self.url_base, collection_name,);

        let client = reqwest::Client::new();

        let res = match &self.api_key {
            Some(api_key) => {
                client
                    .delete(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?
            }
            None => {
                client
                    .delete(&url)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?
            }
        };

        match res.status().is_success() {
            true => {
                // get response body as json
                let json = res.json::<Value>().await?;
                let sucess = json.get("result").unwrap().as_bool().unwrap();
                Ok(sucess)
            }
            false => Err(anyhow!(
                "[qdrant] Failed to delete collection: {}",
                collection_name
            )),
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
        let res = match &self.api_key {
            Some(api_key) => {
                client
                    .put(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
            None => {
                client
                    .put(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
        };

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
        let response = match &self.api_key {
            Some(api_key) => {
                client
                    .post(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
            None => {
                client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
        };

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

        let json = match &self.api_key {
            Some(api_key) => {
                client
                    .post(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
                    .json()
                    .await?
            }
            None => {
                client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
                    .json()
                    .await?
            }
        };

        Ok(json)
    }

    pub async fn get_point_api(&self, collection_name: &str, id: &PointId) -> Result<Value, Error> {
        let url = format!(
            "{}/collections/{}/points/{}",
            self.url_base, collection_name, id,
        );

        let client = reqwest::Client::new();

        let json = match &self.api_key {
            Some(api_key) => {
                client
                    .get(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?
                    .json()
                    .await?
            }
            None => {
                client
                    .get(&url)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?
                    .json()
                    .await?
            }
        };

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

        let res = match &self.api_key {
            Some(api_key) => {
                client
                    .post(&url)
                    .header("api-key", api_key)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
            None => {
                client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await?
            }
        };

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
