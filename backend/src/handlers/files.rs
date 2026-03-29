use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::AppState;
use crate::middleware::AuthUser;
use crate::models::{FileListParams, FileMoveRequest, FileRecord, FileRenameRequest, FileUploadParams, User};

pub async fn upload(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FileUploadParams>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let original_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        let mime_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let size = data.len() as i64;

        let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Check storage limit
        let user_row = client
            .query_one("SELECT * FROM users WHERE id = $1", &[&auth.user_id])
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let user = User::from(user_row);

        if user.storage_used + size > user.storage_limit {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }

        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hex::encode(hasher.finalize());

        // Store file
        let file_id = Uuid::new_v4();
        let storage_dir = PathBuf::from(&state.config.upload_dir).join(auth.user_id.to_string());
        fs::create_dir_all(&storage_dir)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let storage_path = storage_dir.join(file_id.to_string());
        let mut file = fs::File::create(&storage_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        file.write_all(&data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let storage_path_str = storage_path.to_string_lossy().to_string();

        // Insert record
        let row = client
            .query_one(
                "INSERT INTO files (id, name, original_name, mime_type, size, sha256_hash, folder_id, owner_id, storage_path) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
                &[&file_id, &original_name, &original_name, &mime_type, &size, &hash, &params.folder_id, &auth.user_id, &storage_path_str],
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let record = FileRecord::from(row);

        // Update user storage
        client
            .execute(
                "UPDATE users SET storage_used = storage_used + $1 WHERE id = $2",
                &[&size, &auth.user_id],
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok((StatusCode::CREATED, Json(record)));
    }

    Err(StatusCode::BAD_REQUEST)
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FileListParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = if let Some(folder_id) = params.folder_id {
        client
            .query(
                "SELECT * FROM files WHERE owner_id = $1 AND folder_id = $2 ORDER BY name",
                &[&auth.user_id, &folder_id],
            )
            .await
    } else {
        client
            .query(
                "SELECT * FROM files WHERE owner_id = $1 AND folder_id IS NULL ORDER BY name",
                &[&auth.user_id],
            )
            .await
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let files: Vec<FileRecord> = rows.into_iter().map(FileRecord::from).collect();
    Ok(Json(files))
}

pub async fn download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "SELECT * FROM files WHERE id = $1 AND owner_id = $2",
            &[&file_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file = FileRecord::from(row);

    let body = fs::read(&file.storage_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let headers = [
        (header::CONTENT_TYPE, file.mime_type.clone()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file.original_name),
        ),
    ];

    Ok((headers, Body::from(body)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "SELECT * FROM files WHERE id = $1 AND owner_id = $2",
            &[&file_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file = FileRecord::from(row);

    let _ = fs::remove_file(&file.storage_path).await;

    client
        .execute("DELETE FROM files WHERE id = $1", &[&file_id])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    client
        .execute(
            "UPDATE users SET storage_used = storage_used - $1 WHERE id = $2",
            &[&file.size, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn rename(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<Uuid>,
    Json(req): Json<FileRenameRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "UPDATE files SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 RETURNING *",
            &[&req.name, &file_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(FileRecord::from(row)))
}

pub async fn move_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<Uuid>,
    Json(req): Json<FileMoveRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "UPDATE files SET folder_id = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 RETURNING *",
            &[&req.folder_id, &file_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(FileRecord::from(row)))
}
