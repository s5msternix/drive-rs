use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Deserialize)]
pub struct ShareAccessRequest {
    pub password: Option<String>,
}
