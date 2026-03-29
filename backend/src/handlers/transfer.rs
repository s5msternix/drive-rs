use axum::{
    Json,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::AppState;
use crate::models::{CreateTransferRequest, SignalMessage, TransferSession, TransferSessionResponse};

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    hex::encode(bytes)
}

/// Create a new P2P transfer session
pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateTransferRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = generate_token();

    let session = sqlx::query_as::<_, TransferSession>(
        r#"INSERT INTO transfer_sessions (token, file_name, file_size)
           VALUES ($1, $2, $3)
           RETURNING *"#,
    )
    .bind(&token)
    .bind(&req.file_name)
    .bind(req.file_size)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create broadcast channel for this session
    let (tx, _) = broadcast::channel(32);
    state.transfer_channels.write().await.insert(token.clone(), Arc::new(tx));

    Ok((
        StatusCode::CREATED,
        Json(TransferSessionResponse {
            id: session.id,
            token: session.token,
            file_name: session.file_name,
            file_size: session.file_size,
            expires_at: session.expires_at,
        }),
    ))
}

/// Get transfer session info
pub async fn get_session(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let session = sqlx::query_as::<_, TransferSession>(
        "SELECT * FROM transfer_sessions WHERE token = $1 AND is_active = true AND expires_at > NOW()",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(TransferSessionResponse {
        id: session.id,
        token: session.token,
        file_name: session.file_name,
        file_size: session.file_size,
        expires_at: session.expires_at,
    }))
}

/// WebSocket handler for P2P signaling
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_signaling(socket, token, state))
}

async fn handle_signaling(socket: WebSocket, token: String, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Get or create broadcast channel
    let tx = {
        let channels = state.transfer_channels.read().await;
        match channels.get(&token) {
            Some(tx) => Arc::clone(tx),
            None => {
                let _ = sender
                    .send(Message::Text(
                        serde_json::to_string(&SignalMessage::Error {
                            message: "Session not found".to_string(),
                        })
                        .unwrap().into(),
                    ))
                    .await;
                return;
            }
        }
    };

    let mut rx = tx.subscribe();

    // Notify peers that someone joined
    let _ = tx.send(
        serde_json::to_string(&SignalMessage::PeerJoined {
            role: "peer".to_string(),
        })
        .unwrap(),
    );

    // Forward broadcast messages to this WebSocket
    let tx_clone = Arc::clone(&tx);
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Forward WebSocket messages to broadcast channel
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let _ = tx_clone.send(text.to_string());
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    // Notify peers that someone left
    let _ = tx.send(
        serde_json::to_string(&SignalMessage::PeerLeft).unwrap(),
    );
}
