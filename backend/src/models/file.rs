use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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
