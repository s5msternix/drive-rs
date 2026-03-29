use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub id: Uuid,
    pub file_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub token: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub download_count: i32,
    pub max_downloads: Option<i32>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<tokio_postgres::Row> for ShareLink {
    fn from(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            file_id: row.get("file_id"),
            folder_id: row.get("folder_id"),
            token: row.get("token"),
            expires_at: row.get("expires_at"),
            download_count: row.get("download_count"),
            max_downloads: row.get("max_downloads"),
            password_hash: row.get("password_hash"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateShareLinkRequest {
    pub file_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub expires_in_hours: Option<i64>,
    pub max_downloads: Option<i32>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShareLinkResponse {
    pub id: Uuid,
    pub token: String,
    pub url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<i32>,
}
