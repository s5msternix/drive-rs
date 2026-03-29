use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: Uuid,
    pub name: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub sha256_hash: String,
    pub folder_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub storage_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<tokio_postgres::Row> for FileRecord {
    fn from(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            original_name: row.get("original_name"),
            mime_type: row.get("mime_type"),
            size: row.get("size"),
            sha256_hash: row.get("sha256_hash"),
            folder_id: row.get("folder_id"),
            owner_id: row.get("owner_id"),
            storage_path: row.get("storage_path"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FileUploadParams {
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct FileListParams {
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct FileRenameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FileMoveRequest {
    pub folder_id: Option<Uuid>,
}
