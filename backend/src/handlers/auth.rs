use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::rngs::OsRng;

use crate::AppState;
use crate::middleware::auth::create_token;
use crate::models::{AuthResponse, LoginRequest, RegisterRequest, User, UserPublic};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    if req.email.is_empty() || req.username.is_empty() || req.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_one(
            "INSERT INTO users (email, username, password_hash) VALUES ($1, $2, $3) RETURNING *",
            &[&req.email, &req.username, &password_hash],
        )
        .await
        .map_err(|_| StatusCode::CONFLICT)?;

    let user = User::from(row);
    let token = create_token(user.id, &user.email, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: UserPublic::from(user),
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt("SELECT * FROM users WHERE email = $1", &[&req.email])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = User::from(row);

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token = create_token(user.id, &user.email, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse {
        token,
        user: UserPublic::from(user),
    }))
}

pub async fn me(
    State(state): State<AppState>,
    auth: crate::middleware::AuthUser,
) -> Result<impl IntoResponse, StatusCode> {
    let client = state.db.get().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = client
        .query_opt("SELECT * FROM users WHERE id = $1", &[&auth.user_id])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let user = User::from(row);
    Ok(Json(UserPublic::from(user)))
}
