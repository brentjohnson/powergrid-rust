use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{db::AuthError, AppState};

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub identifier: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResp {
    pub token: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Serialize)]
pub struct ErrResp {
    pub error: String,
}

fn err(status: StatusCode, msg: impl ToString) -> (StatusCode, Json<ErrResp>) {
    (
        status,
        Json(ErrResp {
            error: msg.to_string(),
        }),
    )
}

pub async fn register(
    State(app): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> Result<(StatusCode, Json<AuthResp>), (StatusCode, Json<ErrResp>)> {
    if !req.email.contains('@') {
        return Err(err(StatusCode::BAD_REQUEST, "email must contain @"));
    }
    let ulen = req.username.chars().count();
    if !(3..=32).contains(&ulen) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "username must be 3–32 characters",
        ));
    }
    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "username may only contain letters, digits, hyphens, and underscores",
        ));
    }
    if req.password.len() < 8 {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters",
        ));
    }

    match app
        .db
        .register(&req.email, &req.username, &req.password)
        .await
    {
        Ok(s) => Ok((
            StatusCode::CREATED,
            Json(AuthResp {
                token: s.token,
                user_id: s.user_id,
                username: s.username,
            }),
        )),
        Err(AuthError::EmailTaken) => Err(err(StatusCode::CONFLICT, "email already in use")),
        Err(AuthError::UsernameTaken) => Err(err(StatusCode::CONFLICT, "username already in use")),
        Err(e) => Err(err(StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn login(
    State(app): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<AuthResp>, (StatusCode, Json<ErrResp>)> {
    match app.db.login(&req.identifier, &req.password).await {
        Ok(s) => Ok(Json(AuthResp {
            token: s.token,
            user_id: s.user_id,
            username: s.username,
        })),
        Err(AuthError::InvalidCredentials) => {
            Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"))
        }
        Err(e) => Err(err(StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn logout(State(app): State<AppState>, headers: HeaderMap) -> StatusCode {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let _ = app.db.logout(token).await;
    StatusCode::NO_CONTENT
}
