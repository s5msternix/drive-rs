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
use crate::models::{FileListParams, FileMoveRequest, FileRecord, FileRenameRequest, FileUploadParams};

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

        // Check storage limit
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE id = $1",
        )
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

        // Insert record
        let record = sqlx::query_as::<_, FileRecord>(
            r#"INSERT INTO files (id, name, original_name, mime_type, size, sha256_hash, folder_id, owner_id, storage_path)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING *"#,
        )
        .bind(file_id)
        .bind(&original_name)
        .bind(&original_name)
        .bind(&mime_type)
        .bind(size)
        .bind(&hash)
        .bind(params.folder_id)
        .bind(auth.user_id)
        .bind(storage_path.to_string_lossy().as_ref())
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Update user storage
        sqlx::query("UPDATE users SET storage_used = storage_used + $1 WHERE id = $2")
            .bind(size)
            .bind(auth.user_id)
            .execute(&state.db)
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
    let files = if let Some(folder_id) = params.folder_id {
        sqlx::query_as::<_, FileRecord>(
            "SELECT * FROM files WHERE owner_id = $1 AND folder_id = $2 ORDER BY name",
        )
        .bind(auth.user_id)
        .bind(folder_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, FileRecord>(
            "SELECT * FROM files WHERE owner_id = $1 AND folder_id IS NULL ORDER BY name",
        )
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(files))
}

pub async fn download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let file = sqlx::query_as::<_, FileRecord>(
        "SELECT * FROM files WHERE id = $1 AND owner_id = $2",
    )
    .bind(file_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

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
    let file = sqlx::query_as::<_, FileRecord>(
        "SELECT * FROM files WHERE id = $1 AND owner_id = $2",
    )
    .bind(file_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Delete physical file
    let _ = fs::remove_file(&file.storage_path).await;

    // Delete record
    sqlx::query("DELETE FROM files WHERE id = $1")
        .bind(file_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update storage
    sqlx::query("UPDATE users SET storage_used = storage_used - $1 WHERE id = $2")
        .bind(file.size)
        .bind(auth.user_id)
        .execute(&state.db)
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
    let file = sqlx::query_as::<_, FileRecord>(
        "UPDATE files SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 RETURNING *",
    )
    .bind(&req.name)
    .bind(file_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(file))
}

pub async fn move_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<Uuid>,
    Json(req): Json<FileMoveRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let file = sqlx::query_as::<_, FileRecord>(
        "UPDATE files SET folder_id = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 RETURNING *",
    )
    .bind(req.folder_id)
    .bind(file_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(file))
}
