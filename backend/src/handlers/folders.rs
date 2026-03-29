use axum::{Json, extract::{Path, Query, State}, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

use crate::AppState;
use crate::middleware::AuthUser;
use crate::models::{CreateFolderRequest, FileListParams, Folder, RenameFolderRequest};

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    if req.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify parent folder belongs to user if specified
    if let Some(parent_id) = req.parent_id {
        sqlx::query_as::<_, Folder>(
            "SELECT * FROM folders WHERE id = $1 AND owner_id = $2",
        )
        .bind(parent_id)
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    }

    let folder = sqlx::query_as::<_, Folder>(
        r#"INSERT INTO folders (name, parent_id, owner_id)
           VALUES ($1, $2, $3)
           RETURNING *"#,
    )
    .bind(&req.name)
    .bind(req.parent_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;

    Ok((StatusCode::CREATED, Json(folder)))
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<FileListParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let folders = if let Some(folder_id) = params.folder_id {
        sqlx::query_as::<_, Folder>(
            "SELECT * FROM folders WHERE owner_id = $1 AND parent_id = $2 ORDER BY name",
        )
        .bind(auth.user_id)
        .bind(folder_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Folder>(
            "SELECT * FROM folders WHERE owner_id = $1 AND parent_id IS NULL ORDER BY name",
        )
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(folders))
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(folder_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let folder = sqlx::query_as::<_, Folder>(
        "SELECT * FROM folders WHERE id = $1 AND owner_id = $2",
    )
    .bind(folder_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(folder))
}

pub async fn rename(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(folder_id): Path<Uuid>,
    Json(req): Json<RenameFolderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let folder = sqlx::query_as::<_, Folder>(
        "UPDATE folders SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 RETURNING *",
    )
    .bind(&req.name)
    .bind(folder_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(folder))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(folder_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let result = sqlx::query("DELETE FROM folders WHERE id = $1 AND owner_id = $2")
        .bind(folder_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
