use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSession {
    pub id: Uuid,
    pub sender_id: Option<Uuid>,
    pub token: String,
    pub file_name: String,
    pub file_size: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl From<tokio_postgres::Row> for TransferSession {
    fn from(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            sender_id: row.get("sender_id"),
            token: row.get("token"),
            file_name: row.get("file_name"),
            file_size: row.get("file_size"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTransferRequest {
    pub file_name: String,
    pub file_size: i64,
}

#[derive(Debug, Serialize)]
pub struct TransferSessionResponse {
    pub id: Uuid,
    pub token: String,
    pub file_name: String,
    pub file_size: i64,
    pub expires_at: DateTime<Utc>,
}

/// WebSocket signaling messages for WebRTC P2P
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalMessage {
    #[serde(rename = "offer")]
    Offer { sdp: String },
    #[serde(rename = "answer")]
    Answer { sdp: String },
    #[serde(rename = "ice-candidate")]
    IceCandidate { candidate: String },
    #[serde(rename = "peer-joined")]
    PeerJoined { role: String },
    #[serde(rename = "peer-left")]
    PeerLeft,
    #[serde(rename = "error")]
    Error { message: String },
}
