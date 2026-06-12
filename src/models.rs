use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- Database Entities ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub hashed_password: String,
    pub role: String, // "admin" or "user"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,   // "todo", "in_progress", "done"
    pub priority: String, // "low", "medium", "high"
    pub created_by_id: Option<Uuid>,
    pub assigned_to_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LoginChallenge {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EmailLog {
    pub id: i32,
    pub user_email: String,
    pub subject: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

// --- JWT Claims ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid, // user_id
    pub email: String,
    pub role: String,
    pub exp: i64,
}

// --- Request/Response DTOs ---

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub login_challenge_id: Uuid,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2FaRequest {
    pub login_challenge_id: Uuid,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct Verify2FaResponse {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub assigned_to_id: Option<Uuid>,
    pub priority: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTaskRequest {
    pub task_ids: Vec<i32>,
    pub user_email: Option<String>, // if provided, assign to this email; otherwise, default to James Bond
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskViewResponse {
    pub tasks: Vec<Task>,
    pub cache_hit: bool,
    pub user: UserInfo,
}

// --- Axum Extractor for Claims ---

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = crate::error::AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| crate::error::AppError::Unauthorized("Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(crate::error::AppError::Unauthorized(
                "Invalid Authorization header format. Must be Bearer <token>".to_string(),
            ));
        }

        let token = &auth_header[7..];
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            "super_secret_jwt_key_that_is_long_enough_to_be_secure_123456".to_string()
        });

        let claims = crate::auth::verify_jwt(token, &secret).map_err(|e| {
            crate::error::AppError::Unauthorized(format!("Invalid or expired JWT token: {}", e))
        })?;

        Ok(claims)
    }
}

