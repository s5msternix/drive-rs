use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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
    /// Sender offers a file transfer
    #[serde(rename = "offer")]
    Offer { sdp: String },
    /// Receiver answers the offer
    #[serde(rename = "answer")]
    Answer { sdp: String },
    /// ICE candidate exchange
    #[serde(rename = "ice-candidate")]
    IceCandidate { candidate: String },
    /// Peer joined the session
    #[serde(rename = "peer-joined")]
    PeerJoined { role: String },
    /// Peer left the session
    #[serde(rename = "peer-left")]
    PeerLeft,
    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
}
