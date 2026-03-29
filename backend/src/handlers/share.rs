use axum::{Json, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use rand::Rng;

use crate::AppState;
use crate::middleware::AuthUser;
use crate::models::{CreateShareLinkRequest, FileRecord, Folder, ShareLink, ShareLinkResponse};

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    hex::encode(bytes)
}

pub async fn create_link(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateShareLinkRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    if req.file_id.is_none() && req.folder_id.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    if let Some(file_id) = req.file_id {
        client
            .query_opt(
                "SELECT id FROM files WHERE id = $1 AND owner_id = $2",
                &[&file_id, &auth.user_id],
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
    }
    if let Some(folder_id) = req.folder_id {
        client
            .query_opt(
                "SELECT id FROM folders WHERE id = $1 AND owner_id = $2",
                &[&folder_id, &auth.user_id],
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
    }

    let token = generate_token();

    let expires_at = req.expires_in_hours.map(|hours| {
        chrono::Utc::now() + chrono::Duration::hours(hours)
    });

    let password_hash: Option<String> = if let Some(ref pw) = req.password {
        use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
        use rand::rngs::OsRng;
        let salt = SaltString::generate(&mut OsRng);
        Some(
            Argon2::default()
                .hash_password(pw.as_bytes(), &salt)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .to_string(),
        )
    } else {
        None
    };

    let row = client
        .query_one(
            "INSERT INTO share_links (file_id, folder_id, token, expires_at, max_downloads, password_hash, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
            &[&req.file_id, &req.folder_id, &token, &expires_at, &req.max_downloads, &password_hash, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let link = ShareLink::from(row);

    Ok((
        StatusCode::CREATED,
        Json(ShareLinkResponse {
            id: link.id,
            token: link.token,
            url: format!("/share/{}", token),
            expires_at: link.expires_at,
            max_downloads: link.max_downloads,
        }),
    ))
}

pub async fn get_share_info(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt("SELECT * FROM share_links WHERE token = $1", &[&token])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let link = ShareLink::from(row);

    // Check expiration
    if let Some(expires_at) = link.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(StatusCode::GONE);
        }
    }

    // Check download limit
    if let Some(max) = link.max_downloads {
        if link.download_count >= max {
            return Err(StatusCode::GONE);
        }
    }

    let needs_password = link.password_hash.is_some();

    if let Some(file_id) = link.file_id {
        let file_row = client
            .query_opt("SELECT * FROM files WHERE id = $1", &[&file_id])
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        let file = FileRecord::from(file_row);

        return Ok(Json(serde_json::json!({
            "type": "file",
            "name": file.original_name,
            "size": file.size,
            "mime_type": file.mime_type,
            "needs_password": needs_password,
        })));
    }

    if let Some(folder_id) = link.folder_id {
        let folder_row = client
            .query_opt("SELECT * FROM folders WHERE id = $1", &[&folder_id])
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        let folder = Folder::from(folder_row);

        return Ok(Json(serde_json::json!({
            "type": "folder",
            "name": folder.name,
            "needs_password": needs_password,
        })));
    }

    Err(StatusCode::NOT_FOUND)
}

pub async fn download_shared(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt("SELECT * FROM share_links WHERE token = $1", &[&token])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let link = ShareLink::from(row);

    if let Some(expires_at) = link.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(StatusCode::GONE);
        }
    }

    if let Some(max) = link.max_downloads {
        if link.download_count >= max {
            return Err(StatusCode::GONE);
        }
    }

    let file_id = link.file_id.ok_or(StatusCode::BAD_REQUEST)?;

    let file_row = client
        .query_opt("SELECT * FROM files WHERE id = $1", &[&file_id])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file = FileRecord::from(file_row);

    client
        .execute(
            "UPDATE share_links SET download_count = download_count + 1 WHERE id = $1",
            &[&link.id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let body = tokio::fs::read(&file.storage_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let headers = [
        (axum::http::header::CONTENT_TYPE, file.mime_type.clone()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file.original_name),
        ),
    ];

    Ok((headers, axum::body::Body::from(body)))
}
