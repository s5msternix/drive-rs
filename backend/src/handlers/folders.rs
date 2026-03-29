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

    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify parent folder belongs to user if specified
    if let Some(parent_id) = req.parent_id {
        client
            .query_opt(
                "SELECT * FROM folders WHERE id = $1 AND owner_id = $2",
                &[&parent_id, &auth.user_id],
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
    }

    let row = client
        .query_one(
            "INSERT INTO folders (name, parent_id, owner_id) VALUES ($1, $2, $3) RETURNING *",
            &[&req.name, &req.parent_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::CONFLICT)?;

    Ok((StatusCode::CREATED, Json(Folder::from(row))))
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
                "SELECT * FROM folders WHERE owner_id = $1 AND parent_id = $2 ORDER BY name",
                &[&auth.user_id, &folder_id],
            )
            .await
    } else {
        client
            .query(
                "SELECT * FROM folders WHERE owner_id = $1 AND parent_id IS NULL ORDER BY name",
                &[&auth.user_id],
            )
            .await
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let folders: Vec<Folder> = rows.into_iter().map(Folder::from).collect();
    Ok(Json(folders))
}

pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(folder_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "SELECT * FROM folders WHERE id = $1 AND owner_id = $2",
            &[&folder_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(Folder::from(row)))
}

pub async fn rename(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(folder_id): Path<Uuid>,
    Json(req): Json<RenameFolderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt(
            "UPDATE folders SET name = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3 RETURNING *",
            &[&req.name, &folder_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(Folder::from(row)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(folder_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let count = client
        .execute(
            "DELETE FROM folders WHERE id = $1 AND owner_id = $2",
            &[&folder_id, &auth.user_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if count == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
