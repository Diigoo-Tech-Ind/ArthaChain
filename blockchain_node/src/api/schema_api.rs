/// REST API endpoints for Public Schema Registry
/// Implements /api/v1/schema/* endpoints

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

const SCHEMA_DIR: &str = "/Users/sainathtangallapalli/blockchain/ArthaChain/schemas";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub name: String,
    pub active_version: String,
    pub versions: Vec<String>,
    pub deprecated: bool,
    pub deprecation_sunset_epoch: Option<u64>,
    pub last_updated: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaResponse {
    pub name: String,
    #[serde(rename = "activeVersion")]
    pub active_version: String,
    pub versions: Vec<String>,
    pub deprecated: bool,
    pub schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaStatusResponse {
    pub name: String,
    pub version: String,
    pub deprecated: bool,
    #[serde(rename = "sunsetEpoch")]
    pub sunset_epoch: Option<u64>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: u64,
}

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub schema: String,
    pub version: String,
    pub document: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub struct SchemaRegistry {
    schemas: Arc<RwLock<HashMap<String, SchemaMetadata>>>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut schemas = HashMap::new();

        // Initialize with known schemas
        schemas.insert("DIDDoc".to_string(), SchemaMetadata {
            name: "DIDDoc".to_string(),
            active_version: "v1".to_string(),
            versions: vec!["v1".to_string()],
            deprecated: false,
            deprecation_sunset_epoch: None,
            last_updated: 1730592000,
        });

        schemas.insert("AIIDDoc".to_string(), SchemaMetadata {
            name: "AIIDDoc".to_string(),
            active_version: "v1".to_string(),
            versions: vec!["v1".to_string()],
            deprecated: false,
            deprecation_sunset_epoch: None,
            last_updated: 1730592000,
        });

        schemas.insert("VC".to_string(), SchemaMetadata {
            name: "VC".to_string(),
            active_version: "v1".to_string(),
            versions: vec!["v1".to_string()],
            deprecated: false,
            deprecation_sunset_epoch: None,
            last_updated: 1730592000,
        });

        schemas.insert("NodeCert".to_string(), SchemaMetadata {
            name: "NodeCert".to_string(),
            active_version: "v1".to_string(),
            versions: vec!["v1".to_string()],
            deprecated: false,
            deprecation_sunset_epoch: None,
            last_updated: 1730592000,
        });

        schemas.insert("JobSpec".to_string(), SchemaMetadata {
            name: "JobSpec".to_string(),
            active_version: "v1".to_string(),
            versions: vec!["v1".to_string()],
            deprecated: false,
            deprecation_sunset_epoch: None,
            last_updated: 1730592000,
        });

        SchemaRegistry {
            schemas: Arc::new(RwLock::new(schemas)),
        }
    }

    pub fn get_schema_metadata(&self, name: &str) -> Option<SchemaMetadata> {
        let schemas = self.schemas.read().unwrap();
        schemas.get(name).cloned()
    }

    pub fn list_all_schemas(&self) -> Vec<SchemaMetadata> {
        let schemas = self.schemas.read().unwrap();
        schemas.values().cloned().collect()
    }
}

/// GET /api/v1/schema/{name}
/// Get active version of a schema with full JSON
pub async fn get_schema(
    Path(name): Path<String>,
    registry: Arc<SchemaRegistry>,
) -> Result<Json<SchemaResponse>, StatusCode> {
    let metadata = registry
        .get_schema_metadata(&name)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Load schema file from disk
    let schema_path = format!("{}/{}.{}.json", SCHEMA_DIR, name, metadata.active_version);
    let schema_content = std::fs::read_to_string(&schema_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SchemaResponse {
        name: metadata.name,
        active_version: metadata.active_version,
        versions: metadata.versions,
        deprecated: metadata.deprecated,
        schema: schema_json,
    }))
}

/// GET /api/v1/schema/{name}/versions
/// List all versions of a schema
pub async fn get_schema_versions(
    Path(name): Path<String>,
    registry: Arc<SchemaRegistry>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let metadata = registry
        .get_schema_metadata(&name)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(metadata.versions))
}

/// GET /api/v1/schema/{name}@{version}
/// Get specific version of a schema
pub async fn get_schema_version(
    Path((name, version)): Path<(String, String)>,
    registry: Arc<SchemaRegistry>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let metadata = registry
        .get_schema_metadata(&name)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if version exists
    if !metadata.versions.contains(&version) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Load schema file
    let schema_path = format!("{}/{}.{}.json", SCHEMA_DIR, name, version);
    let schema_content = std::fs::read_to_string(&schema_path)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(schema_json))
}

/// GET /api/v1/schema/{name}@{version}/status
/// Check deprecation status of a specific version
pub async fn get_schema_status(
    Path((name, version)): Path<(String, String)>,
    registry: Arc<SchemaRegistry>,
) -> Result<Json<SchemaStatusResponse>, StatusCode> {
    let metadata = registry
        .get_schema_metadata(&name)
        .ok_or(StatusCode::NOT_FOUND)?;

    if !metadata.versions.contains(&version) {
        return Err(StatusCode::NOT_FOUND);
    }

    let deprecated = metadata.deprecated && version != metadata.active_version;

    Ok(Json(SchemaStatusResponse {
        name: metadata.name,
        version,
        deprecated,
        sunset_epoch: if deprecated { metadata.deprecation_sunset_epoch } else { None },
        last_updated: metadata.last_updated,
    }))
}

/// POST /api/v1/schema/validate
/// Validate a document against a schema
pub async fn validate_schema(
    Json(payload): Json<ValidateRequest>,
    registry: Arc<SchemaRegistry>,
) -> Result<Json<ValidateResponse>, StatusCode> {
    let metadata = registry
        .get_schema_metadata(&payload.schema)
        .ok_or(StatusCode::NOT_FOUND)?;

    if !metadata.versions.contains(&payload.version) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Load schema
    let schema_path = format!("{}/{}.{}.json", SCHEMA_DIR, payload.schema, payload.version);
    let schema_content = std::fs::read_to_string(&schema_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Validate using jsonschema crate
    use jsonschema::JSONSchema;

    let compiled_schema = JSONSchema::compile(&schema_json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let validation_result = compiled_schema.validate(&payload.document);

    match validation_result {
        Ok(_) => Ok(Json(ValidateResponse {
            valid: true,
            errors: vec![],
        })),
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .map(|e| format!("{} at {}", e, e.instance_path))
                .collect();

            Ok(Json(ValidateResponse {
                valid: false,
                errors: error_messages,
            }))
        }
    }
}

/// GET /api/v1/schemas
/// List all available schemas
pub async fn list_schemas(
    registry: Arc<SchemaRegistry>,
) -> Json<Vec<SchemaMetadata>> {
    Json(registry.list_all_schemas())
}

/// Build the schema API router
pub fn schema_router(registry: Arc<SchemaRegistry>) -> Router {
    Router::new()
        .route("/api/v1/schemas", get({
            let reg = Arc::clone(&registry);
            move || list_schemas(Arc::clone(&reg))
        }))
        .route("/api/v1/schema/:name", get({
            let reg = Arc::clone(&registry);
            move |path| get_schema(path, Arc::clone(&reg))
        }))
        .route("/api/v1/schema/:name/versions", get({
            let reg = Arc::clone(&registry);
            move |path| get_schema_versions(path, Arc::clone(&reg))
        }))
        .route("/api/v1/schema/:name@:version", get({
            let reg = Arc::clone(&registry);
            move |path| get_schema_version(path, Arc::clone(&reg))
        }))
        .route("/api/v1/schema/:name@:version/status", get({
            let reg = Arc::clone(&registry);
            move |path| get_schema_status(path, Arc::clone(&reg))
        }))
        .route("/api/v1/schema/validate", post({
            let reg = Arc::clone(&registry);
            move |payload| validate_schema(payload, Arc::clone(&reg))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = SchemaRegistry::new();
        let schemas = registry.list_all_schemas();
        assert_eq!(schemas.len(), 5);

        let did_doc = registry.get_schema_metadata("DIDDoc");
        assert!(did_doc.is_some());
        assert_eq!(did_doc.unwrap().active_version, "v1");
    }

    #[test]
    fn test_get_nonexistent_schema() {
        let registry = SchemaRegistry::new();
        let result = registry.get_schema_metadata("NonExistent");
        assert!(result.is_none());
    }
}

